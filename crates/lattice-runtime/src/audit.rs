//! Audit sink implementations.

use crate::ports::AuditSink;
use crate::query::AuditRecord;
use lattice_core::{lock_mutex, Error};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Audit sink that discards all records.
pub struct NoopAuditSink;

impl AuditSink for NoopAuditSink {
    fn write_audit(&self, _record: AuditRecord) -> Result<(), Error> {
        Ok(())
    }
}

/// Audit sink that stores records in a Vec for testing.
pub struct VecAuditSink {
    records: Mutex<Vec<AuditRecord>>,
}

impl VecAuditSink {
    /// Create a new vec sink.
    pub fn new() -> Self {
        Self {
            records: Mutex::new(Vec::new()),
        }
    }

    /// Drain all records.
    pub fn drain(&self) -> Result<Vec<AuditRecord>, Error> {
        Ok(lock_mutex(&self.records)?.drain(..).collect())
    }
}

impl Default for VecAuditSink {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditSink for VecAuditSink {
    fn write_audit(&self, record: AuditRecord) -> Result<(), Error> {
        lock_mutex(&self.records)?.push(record);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// BufferedAuditSink
// ---------------------------------------------------------------------------

/// Overflow behavior for [`BufferedAuditSink`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditOverflowPolicy {
    /// Return an error when the buffer is full.
    Error,
    /// Drop the record and increment the drop counter.
    Drop,
}

/// Configuration for [`BufferedAuditSink`].
#[derive(Debug, Clone)]
pub struct BufferedAuditConfig {
    /// Number of records to buffer before a forced flush.
    pub batch_size: usize,
    /// Maximum time between flushes (records can sit for up to this long).
    pub flush_interval: Duration,
    /// Internal channel capacity.
    pub buffer_size: usize,
    /// What to do when `buffer_size` is exhausted.
    pub overflow_policy: AuditOverflowPolicy,
}

impl Default for BufferedAuditConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            flush_interval: Duration::from_secs(5),
            buffer_size: 4096,
            overflow_policy: AuditOverflowPolicy::Error,
        }
    }
}

/// Channel-based buffering wrapper around any [`AuditSink`].
///
/// `write_audit` returns immediately; a background thread drains the channel
/// in batches. In production configuration the sink returns an error when the
/// channel is full. Tests can opt into dropping records by setting
/// [`AuditOverflowPolicy::Drop`].
pub struct BufferedAuditSink {
    tx: Option<std::sync::mpsc::SyncSender<AuditRecord>>,
    drop_count: Arc<Mutex<u64>>,
    overflow_policy: AuditOverflowPolicy,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl BufferedAuditSink {
    /// Create a new buffered sink wrapping `inner`.
    pub fn new(inner: Arc<dyn AuditSink>, config: BufferedAuditConfig) -> Self {
        let (tx, rx) = std::sync::mpsc::sync_channel::<AuditRecord>(config.buffer_size);
        let drop_count = Arc::new(Mutex::new(0u64));

        let drop_count_bg = drop_count.clone();
        let handle = std::thread::Builder::new()
            .name("lattice-audit-flush".to_string())
            .spawn(move || {
                let mut batch: Vec<AuditRecord> = Vec::with_capacity(config.batch_size);
                let deadline = std::time::Instant::now() + config.flush_interval;

                loop {
                    match rx.recv_timeout(config.flush_interval) {
                        Ok(rec) => {
                            batch.push(rec);
                            if batch.len() >= config.batch_size
                                || std::time::Instant::now() >= deadline
                            {
                                Self::flush_batch(&*inner, &mut batch, &drop_count_bg);
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            Self::flush_batch(&*inner, &mut batch, &drop_count_bg);
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                            Self::flush_batch(&*inner, &mut batch, &drop_count_bg);
                            break;
                        }
                    }
                }
            })
            .expect("failed to spawn audit flush thread");

        Self {
            tx: Some(tx),
            drop_count,
            overflow_policy: config.overflow_policy,
            handle: Some(handle),
        }
    }

    /// Number of records dropped due to a full channel.
    pub fn drop_count(&self) -> Result<u64, Error> {
        Ok(*lock_mutex(&self.drop_count)?)
    }

    fn flush_batch(sink: &dyn AuditSink, batch: &mut Vec<AuditRecord>, drops: &Mutex<u64>) {
        for rec in batch.drain(..) {
            if let Err(_e) = sink.write_audit(rec) {
                if let Ok(mut count) = drops.lock() {
                    *count += 1;
                }
            }
        }
    }
}

impl Drop for BufferedAuditSink {
    fn drop(&mut self) {
        drop(self.tx.take());
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl AuditSink for BufferedAuditSink {
    fn write_audit(&self, record: AuditRecord) -> Result<(), Error> {
        let tx = self
            .tx
            .as_ref()
            .ok_or_else(|| Error::internal("audit sink shut down"))?;
        match tx.try_send(record) {
            Ok(()) => Ok(()),
            Err(std::sync::mpsc::TrySendError::Full(_)) => {
                if let Ok(mut count) = self.drop_count.lock() {
                    *count += 1;
                }
                match self.overflow_policy {
                    AuditOverflowPolicy::Drop => Ok(()),
                    AuditOverflowPolicy::Error => Err(Error::internal("audit buffer full")),
                }
            }
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                Err(Error::internal("audit flush thread disconnected"))
            }
        }
    }
}
