//! Bulk-upload pipeline: port trait + buffered in-memory staging.
//!
//! [`UploadPort`] is the agnostic interface; adapters for S3, GCS, or Azure
//! Blob can implement it directly.
//!
//! [`BufferedUploadManager`] stages records in memory, flushes to a backend
//! when the buffer reaches `batch_size`, and exposes upload progress via
//! a [`UploadStatus`].

use crate::ports::AuditSink;
use crate::query::AuditRecord;
use lattice_core::{lock_mutex, ApiName, Error};
use lattice_ir::Record;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Port
// ---------------------------------------------------------------------------

/// A single upload request.
#[derive(Debug, Clone)]
pub struct UploadRequest {
    /// Object type for the records.
    pub object_type: ApiName,
    /// Records to load.
    pub records: Vec<Record>,
    /// Caller-supplied load ID — used for idempotency and rollback.
    pub load_id: String,
}

/// Result of processing one upload batch.
#[derive(Debug, Clone)]
pub struct UploadResult {
    /// Load ID echoed from the request.
    pub load_id: String,
    /// Number of records successfully written.
    pub written: i64,
    /// Per-record errors (index → message).
    pub errors: BTreeMap<usize, String>,
}

/// Agnostic bulk-upload port.
///
/// Implement this to write to Postgres COPY, Parquet files on object storage,
/// or any other bulk destination.
pub trait UploadPort: Send + Sync {
    /// Flush a batch of records and return the result.
    fn flush(&self, req: UploadRequest) -> Result<UploadResult, Error>;
    /// Roll back a previously flushed load.
    fn rollback(&self, object_type: &ApiName, load_id: &str) -> Result<(), Error>;
}

// ---------------------------------------------------------------------------
// Upload progress tracking
// ---------------------------------------------------------------------------

/// Snapshot of upload progress.
#[derive(Debug, Clone, Default)]
pub struct UploadStatus {
    /// Total records submitted.
    pub total: i64,
    /// Records successfully written.
    pub written: i64,
    /// Records that failed.
    pub failed: i64,
    /// Number of batches flushed.
    pub batches_flushed: u64,
}

// ---------------------------------------------------------------------------
// BufferedUploadManager
// ---------------------------------------------------------------------------

struct Pending {
    object_type: ApiName,
    load_id: String,
    records: Vec<Record>,
}

/// In-memory upload manager that buffers records and flushes in batches.
///
/// Records are held in memory until `batch_size` is reached, then forwarded to
/// the configured [`UploadPort`].  Call [`flush_all`] on shutdown to drain any
/// remaining records.
pub struct BufferedUploadManager {
    port: Arc<dyn UploadPort>,
    audit_sink: Option<Arc<dyn AuditSink>>,
    batch_size: usize,
    pending: Mutex<Pending>,
    status: Mutex<UploadStatus>,
}

impl BufferedUploadManager {
    /// Create a new manager.
    ///
    /// - `port` — backend that receives flushed batches
    /// - `object_type` — object type all records in this manager belong to
    /// - `load_id` — idempotency key for the whole upload session
    /// - `batch_size` — flush when this many records have been buffered
    pub fn new(
        port: Arc<dyn UploadPort>,
        object_type: ApiName,
        load_id: impl Into<String>,
        batch_size: usize,
    ) -> Self {
        assert!(batch_size > 0, "batch_size must be positive");
        Self {
            port,
            audit_sink: None,
            batch_size,
            pending: Mutex::new(Pending {
                object_type,
                load_id: load_id.into(),
                records: Vec::new(),
            }),
            status: Mutex::new(UploadStatus::default()),
        }
    }

    /// Attach an audit sink to record flush events.
    pub fn with_audit_sink(mut self, sink: Arc<dyn AuditSink>) -> Self {
        self.audit_sink = Some(sink);
        self
    }

    /// Stage `record` for upload, flushing automatically when batch is full.
    pub fn push(&self, record: Record) -> Result<(), Error> {
        let mut pending = lock_mutex(&self.pending)?;
        pending.records.push(record);
        lock_mutex(&self.status)?.total += 1;

        if pending.records.len() >= self.batch_size {
            let batch: Vec<Record> = pending.records.drain(..).collect();
            let req = UploadRequest {
                object_type: pending.object_type.clone(),
                records: batch,
                load_id: pending.load_id.clone(),
            };
            drop(pending);
            self.do_flush(req)?;
        }
        Ok(())
    }

    /// Flush any remaining buffered records to the backend.
    pub fn flush_all(&self) -> Result<(), Error> {
        let mut pending = lock_mutex(&self.pending)?;
        if pending.records.is_empty() {
            return Ok(());
        }
        let batch: Vec<Record> = pending.records.drain(..).collect();
        let req = UploadRequest {
            object_type: pending.object_type.clone(),
            records: batch,
            load_id: pending.load_id.clone(),
        };
        drop(pending);
        self.do_flush(req)
    }

    /// Return a snapshot of current upload progress.
    pub fn status(&self) -> Result<UploadStatus, Error> {
        Ok(lock_mutex(&self.status)?.clone())
    }

    fn do_flush(&self, req: UploadRequest) -> Result<(), Error> {
        let result = self.port.flush(req)?;
        let mut status = lock_mutex(&self.status)?;
        status.written += result.written;
        status.failed += result.errors.len() as i64;
        status.batches_flushed += 1;

        if let Some(sink) = &self.audit_sink {
            let _ = sink.write_audit(AuditRecord {
                id: uuid::Uuid::new_v4().to_string(),
                operation: "bulk_load".to_string(),
                actor_user_id: "upload-manager".to_string(),
                resource_kind: "object_type".to_string(),
                resource: result.load_id.clone(),
                decision: "allow".to_string(),
                result_count: Some(result.written),
                error_code: None,
                occurred_at: chrono::Utc::now().to_rfc3339(),
                metadata: std::collections::BTreeMap::new(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI64, Ordering};

    struct CountingPort(AtomicI64);

    impl UploadPort for CountingPort {
        fn flush(&self, req: UploadRequest) -> Result<UploadResult, Error> {
            let n = req.records.len() as i64;
            self.0.fetch_add(n, Ordering::Relaxed);
            Ok(UploadResult {
                load_id: req.load_id,
                written: n,
                errors: BTreeMap::new(),
            })
        }
        fn rollback(&self, _: &ApiName, _: &str) -> Result<(), Error> {
            Ok(())
        }
    }

    fn name(s: &str) -> ApiName {
        ApiName::new_unchecked(s)
    }

    fn record() -> Record {
        Record {
            primary_key: None,
            values: BTreeMap::new(),
        }
    }

    #[test]
    fn auto_flushes_at_batch_size() {
        let port = Arc::new(CountingPort(AtomicI64::new(0)));
        let mgr = BufferedUploadManager::new(port.clone(), name("User"), "load-1", 3);
        mgr.push(record()).unwrap();
        mgr.push(record()).unwrap();
        assert_eq!(port.0.load(Ordering::Relaxed), 0, "not yet flushed");
        mgr.push(record()).unwrap(); // triggers flush
        assert_eq!(port.0.load(Ordering::Relaxed), 3, "batch flushed");
    }

    #[test]
    fn flush_all_drains_remainder() {
        let port = Arc::new(CountingPort(AtomicI64::new(0)));
        let mgr = BufferedUploadManager::new(port.clone(), name("User"), "load-2", 10);
        mgr.push(record()).unwrap();
        mgr.push(record()).unwrap();
        mgr.flush_all().unwrap();
        assert_eq!(port.0.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn status_tracks_progress() {
        let port = Arc::new(CountingPort(AtomicI64::new(0)));
        let mgr = BufferedUploadManager::new(port, name("User"), "load-3", 2);
        mgr.push(record()).unwrap();
        mgr.push(record()).unwrap(); // flush
        mgr.push(record()).unwrap();
        mgr.flush_all().unwrap();
        let s = mgr.status().unwrap();
        assert_eq!(s.total, 3);
        assert_eq!(s.written, 3);
        assert_eq!(s.batches_flushed, 2);
    }
}
