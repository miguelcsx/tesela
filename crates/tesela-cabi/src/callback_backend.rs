//! C ABI callback backend — forwards runtime operations to a user-provided C callback.
//!
//! The callback receives a JSON request like
//! `{"op":"search","object_type":"user","query":{...}}`
//! and must return a JSON response matching the expected return type.

use crate::handle::CCallback;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::os::raw::{c_char, c_int};
use tesela_core::{ApiName, Error, Value};
use tesela_ir::{AggregateResult, ExplainPlan, MutationResult, Page, Record};
use tesela_runtime::ports::{
    Aggregator, Backend, BulkLoader, Getter, Mutator, Rollbacker, SearchExplainer, Searcher,
    Traverser,
};
use tesela_runtime::query::{AggregateQuery, BackendCapabilities, Mutation, Query, TraversalQuery};

/// A backend that delegates every operation to a C callback.
pub(crate) struct CAbiBackend {
    adapter_type: String,
    callback: CCallback,
}

impl CAbiBackend {
    pub(crate) fn new(adapter_type: String, callback: CCallback) -> Self {
        Self {
            adapter_type,
            callback,
        }
    }

    unsafe fn call_json<T: serde::de::DeserializeOwned>(
        &self,
        req: &impl Serialize,
    ) -> Result<T, Error> {
        let req_bytes = serde_json::to_vec(req).map_err(|e| Error::adapter(e.to_string()))?;
        let req_len = c_int::try_from(req_bytes.len())
            .map_err(|_| Error::internal("callback request exceeds c_int::MAX"))?;
        let mut out_len: c_int = 0;
        let resp_ptr = (self.callback.callback)(
            self.callback.user_data,
            req_bytes.as_ptr() as *const c_char,
            req_len,
            &mut out_len,
        );
        if resp_ptr.is_null() || out_len <= 0 {
            return Err(Error::adapter("callback returned null or empty response"));
        }
        let slice = unsafe { std::slice::from_raw_parts(resp_ptr as *const u8, out_len as usize) };
        let json_str = std::str::from_utf8(slice).map_err(|e| Error::adapter(e.to_string()))?;
        let payload: serde_json::Value =
            serde_json::from_str(json_str).map_err(|e| Error::adapter(e.to_string()))?;
        unsafe { libc::free(resp_ptr as *mut libc::c_void) };
        if let Some(message) = payload
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
        {
            return Err(Error::adapter(message.to_string()));
        }
        let payload = payload.get("value").cloned().unwrap_or(payload);
        let result: T =
            serde_json::from_value(payload).map_err(|e| Error::adapter(e.to_string()))?;
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Request/response types used for JSON serialization across the C boundary
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct SearchReq {
    op: &'static str,
    object_type: String,
    query: Query,
}

#[derive(Serialize)]
struct GetReq {
    op: &'static str,
    object_type: String,
    primary_key: Value,
}

#[derive(Serialize)]
struct MutateReq {
    op: &'static str,
    object_type: String,
    mutation: Mutation,
}

#[derive(Serialize)]
struct AggregateReq {
    op: &'static str,
    object_type: String,
    query: AggregateQuery,
}

#[derive(Serialize)]
struct TraverseReq {
    op: &'static str,
    link_type: String,
    query: TraversalQuery,
}

#[derive(Serialize)]
struct ExplainReq {
    op: &'static str,
    object_type: String,
    query: Query,
}

#[derive(Serialize)]
struct BulkLoadReq {
    op: &'static str,
    object_type: String,
    records: Vec<Record>,
    load_id: String,
}

#[derive(Serialize)]
struct RollbackReq {
    op: &'static str,
    object_type: String,
    load_id: String,
}

#[derive(Deserialize)]
struct PageResp {
    records: Vec<Record>,
    #[serde(default)]
    next_cursor: Option<String>,
}

#[derive(Deserialize)]
struct RecordOptResp {
    record: Option<Record>,
}

#[derive(Deserialize)]
struct MutationResultResp {
    record: Option<Record>,
    #[serde(default)]
    rows_affected: Option<i64>,
}

#[derive(Deserialize)]
struct CountResp {
    count: i64,
}

#[derive(Deserialize)]
struct ExplainResp {
    steps: Vec<BTreeMap<String, Value>>,
}

impl Backend for CAbiBackend {
    fn backend_type(&self) -> &str {
        &self.adapter_type
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            search: true,
            get: true,
            mutate: true,
            aggregate: true,
            traverse: true,
            bulk_load: true,
            rollback: true,
            explain: true,
        }
    }

    fn close(&self) -> Result<(), Error> {
        Ok(())
    }

    fn as_searcher(&self) -> Option<&dyn Searcher> {
        Some(self)
    }
    fn as_getter(&self) -> Option<&dyn Getter> {
        Some(self)
    }
    fn as_mutator(&self) -> Option<&dyn Mutator> {
        Some(self)
    }
    fn as_aggregator(&self) -> Option<&dyn Aggregator> {
        Some(self)
    }
    fn as_traverser(&self) -> Option<&dyn Traverser> {
        Some(self)
    }
    fn as_bulk_loader(&self) -> Option<&dyn BulkLoader> {
        Some(self)
    }
    fn as_rollbacker(&self) -> Option<&dyn Rollbacker> {
        Some(self)
    }
    fn as_explainer(&self) -> Option<&dyn SearchExplainer> {
        Some(self)
    }
}

impl Searcher for CAbiBackend {
    fn search(&self, object_type: &ApiName, query: &Query) -> Result<Page, Error> {
        let req = SearchReq {
            op: "search",
            object_type: object_type.to_string(),
            query: query.clone(),
        };
        let resp: PageResp = unsafe { self.call_json(&req)? };
        Ok(Page {
            records: resp.records,
            next_cursor: resp.next_cursor,
        })
    }
}

impl Getter for CAbiBackend {
    fn get(&self, object_type: &ApiName, pk: &Value) -> Result<Option<Record>, Error> {
        let req = GetReq {
            op: "get",
            object_type: object_type.to_string(),
            primary_key: pk.clone(),
        };
        let resp: RecordOptResp = unsafe { self.call_json(&req)? };
        Ok(resp.record)
    }
}

impl Mutator for CAbiBackend {
    fn mutate(&self, object_type: &ApiName, mutation: &Mutation) -> Result<MutationResult, Error> {
        let req = MutateReq {
            op: "mutate",
            object_type: object_type.to_string(),
            mutation: mutation.clone(),
        };
        let resp: MutationResultResp = unsafe { self.call_json(&req)? };
        Ok(MutationResult {
            record: resp.record,
            rows_affected: resp.rows_affected,
        })
    }
}

impl Aggregator for CAbiBackend {
    fn aggregate(
        &self,
        object_type: &ApiName,
        query: &AggregateQuery,
    ) -> Result<AggregateResult, Error> {
        let req = AggregateReq {
            op: "aggregate",
            object_type: object_type.to_string(),
            query: query.clone(),
        };
        let resp: AggregateResult = unsafe { self.call_json(&req)? };
        Ok(resp)
    }
}

impl Traverser for CAbiBackend {
    fn traverse(&self, link_type: &ApiName, query: &TraversalQuery) -> Result<Page, Error> {
        let req = TraverseReq {
            op: "traverse",
            link_type: link_type.to_string(),
            query: query.clone(),
        };
        let resp: PageResp = unsafe { self.call_json(&req)? };
        Ok(Page {
            records: resp.records,
            next_cursor: resp.next_cursor,
        })
    }
}

impl BulkLoader for CAbiBackend {
    fn bulk_load(
        &self,
        object_type: &ApiName,
        records: Vec<Record>,
        load_id: &str,
    ) -> Result<i64, Error> {
        let req = BulkLoadReq {
            op: "bulk_load",
            object_type: object_type.to_string(),
            records,
            load_id: load_id.to_string(),
        };
        let resp: CountResp = unsafe { self.call_json(&req)? };
        Ok(resp.count)
    }
}

impl Rollbacker for CAbiBackend {
    fn rollback(&self, _object_type: &ApiName, load_id: &str) -> Result<(), Error> {
        let req = RollbackReq {
            op: "rollback",
            object_type: _object_type.to_string(),
            load_id: load_id.to_string(),
        };
        let _: serde_json::Value = unsafe { self.call_json(&req)? };
        Ok(())
    }
}

impl SearchExplainer for CAbiBackend {
    fn explain_search(&self, object_type: &ApiName, query: &Query) -> Result<ExplainPlan, Error> {
        let req = ExplainReq {
            op: "explain",
            object_type: object_type.to_string(),
            query: query.clone(),
        };
        let resp: ExplainResp = unsafe { self.call_json(&req)? };
        Ok(ExplainPlan { steps: resp.steps })
    }
}
