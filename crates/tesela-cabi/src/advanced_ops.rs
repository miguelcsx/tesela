//! Advanced operation CABI exports (vector search, branches, pipelines, lineage, etc.).

use crate::handle::*;
use tesela_core::Value;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

fn runtime_from_handle(handle: u64) -> Result<std::sync::Arc<tesela_runtime::Runtime>, String> {
    let ht = handles()
        .lock()
        .map_err(|_| "handle table lock poisoned".to_string())?;
    ht.get(handle)
        .cloned()
        .ok_or_else(|| "invalid handle".to_string())
}

/// Vector / semantic search.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_vector_search_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    query_json: *const c_char,
    query_len: c_int,
) -> TeselaBuffer {
    let mut ht = lock_handles!(or return TeselaBuffer::empty());
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid handle");
            return TeselaBuffer::empty();
        }
    };
    let actor = unsafe { extract_actor(actor_json, actor_len) };
    let body: serde_json::Value = match unsafe { decode_json(query_json, query_len) } {
        Ok(v) => v,
        Err(e) => {
            ht.set_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let obj_str = body
        .get("object_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let object_type = match obj_str.parse::<tesela_core::ApiName>() {
        Ok(n) => n,
        Err(e) => {
            ht.set_error(&format!("invalid object_type: {}", e));
            return TeselaBuffer::empty();
        }
    };
    let query_vector: Vec<f32> = body
        .get("query_vector")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect()
        })
        .unwrap_or_default();
    let query = tesela_runtime::ports::VectorSearchQuery {
        object_type,
        query_vector,
        top_k: body.get("top_k").and_then(|v| v.as_u64()).unwrap_or(10) as usize,
        ef: body.get("ef").and_then(|v| v.as_u64()).unwrap_or(50) as usize,
        filter: body
            .get("filter")
            .and_then(|v| serde_json::from_value(v.clone()).ok()),
    };
    match rt.vector_search(&actor, query) {
        Ok(results) => marshal_result(&results, &mut ht),
        Err(e) => {
            ht.set_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Issue a capability grant.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_issue_capability_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    grant_name: *const c_char,
    body_json: *const c_char,
    body_len: c_int,
) -> TeselaBuffer {
    let rt = match runtime_from_handle(handle) {
        Ok(rt) => rt,
        Err(e) => {
            set_last_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let actor = unsafe { extract_actor(actor_json, actor_len) };
    let name = match unsafe { parse_api_name(grant_name) } {
        Ok(n) => n,
        Err(e) => {
            set_last_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let body: std::collections::BTreeMap<String, Value> = if body_len > 0 {
        match unsafe { decode_json(body_json, body_len) } {
            Ok(v) => v,
            Err(e) => {
                set_last_error(&e);
                return TeselaBuffer::empty();
            }
        }
    } else {
        Default::default()
    };
    match rt.issue_capability(&actor, &name, body) {
        Ok(result) => marshal_result_global(&result),
        Err(e) => {
            set_last_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Authorize artifact read and return a locator.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_authorize_artifact_read_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    artifact_name: *const c_char,
    body_json: *const c_char,
    body_len: c_int,
    ttl_secs: u64,
) -> TeselaBuffer {
    let rt = match runtime_from_handle(handle) {
        Ok(rt) => rt,
        Err(e) => {
            set_last_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let actor = unsafe { extract_actor(actor_json, actor_len) };
    let name = match unsafe { parse_api_name(artifact_name) } {
        Ok(n) => n,
        Err(e) => {
            set_last_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let body: std::collections::BTreeMap<String, Value> = if body_len > 0 {
        match unsafe { decode_json(body_json, body_len) } {
            Ok(v) => v,
            Err(e) => {
                set_last_error(&e);
                return TeselaBuffer::empty();
            }
        }
    } else {
        Default::default()
    };
    match rt.authorize_artifact_read(&actor, &name, body, ttl_secs) {
        Ok(result) => marshal_result_global(&result),
        Err(e) => {
            set_last_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Initiate an upload flow and return a signed upload locator.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_initiate_upload_flow_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    flow_name: *const c_char,
    body_json: *const c_char,
    body_len: c_int,
    ttl_secs: u64,
) -> TeselaBuffer {
    let rt = match runtime_from_handle(handle) {
        Ok(rt) => rt,
        Err(e) => {
            set_last_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let actor = unsafe { extract_actor(actor_json, actor_len) };
    let name = match unsafe { parse_api_name(flow_name) } {
        Ok(n) => n,
        Err(e) => {
            set_last_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let body: std::collections::BTreeMap<String, Value> = if body_len > 0 {
        match unsafe { decode_json(body_json, body_len) } {
            Ok(v) => v,
            Err(e) => {
                set_last_error(&e);
                return TeselaBuffer::empty();
            }
        }
    } else {
        Default::default()
    };
    match rt.initiate_upload_flow(&actor, &name, body, ttl_secs) {
        Ok(result) => marshal_result_global(&result),
        Err(e) => {
            set_last_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Start a declared job.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_start_job_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    job_name: *const c_char,
    body_json: *const c_char,
    body_len: c_int,
) -> TeselaBuffer {
    let rt = match runtime_from_handle(handle) {
        Ok(rt) => rt,
        Err(e) => {
            set_last_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let actor = unsafe { extract_actor(actor_json, actor_len) };
    let name = match unsafe { parse_api_name(job_name) } {
        Ok(n) => n,
        Err(e) => {
            set_last_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let body: serde_json::Value = if body_len > 0 {
        match unsafe { decode_json(body_json, body_len) } {
            Ok(v) => v,
            Err(e) => {
                set_last_error(&e);
                return TeselaBuffer::empty();
            }
        }
    } else {
        serde_json::json!({})
    };
    let input = body
        .get("input")
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let idempotency_key = body
        .get("idempotency_key")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);
    match rt.start_job(&actor, &name, input, idempotency_key) {
        Ok(result) => marshal_result_global(&result),
        Err(e) => {
            set_last_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Complete an upload flow.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_complete_upload_flow_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    flow_name: *const c_char,
    body_json: *const c_char,
    body_len: c_int,
) -> TeselaBuffer {
    let rt = match runtime_from_handle(handle) {
        Ok(rt) => rt,
        Err(e) => {
            set_last_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let actor = unsafe { extract_actor(actor_json, actor_len) };
    let name = match unsafe { parse_api_name(flow_name) } {
        Ok(n) => n,
        Err(e) => {
            set_last_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let body: serde_json::Value = match unsafe { decode_json(body_json, body_len) } {
        Ok(v) => v,
        Err(e) => {
            set_last_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let path = body.get("path").and_then(|v| v.as_str()).unwrap_or("");
    match rt.complete_upload_flow(&actor, &name, path) {
        Ok(result) => marshal_result_global(&result),
        Err(e) => {
            set_last_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Load records through an upload flow.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_load_upload_flow_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    flow_name: *const c_char,
    body_json: *const c_char,
    body_len: c_int,
) -> TeselaBuffer {
    let rt = match runtime_from_handle(handle) {
        Ok(rt) => rt,
        Err(e) => {
            set_last_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let actor = unsafe { extract_actor(actor_json, actor_len) };
    let name = match unsafe { parse_api_name(flow_name) } {
        Ok(n) => n,
        Err(e) => {
            set_last_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let body: serde_json::Value = match unsafe { decode_json(body_json, body_len) } {
        Ok(v) => v,
        Err(e) => {
            set_last_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let records = body
        .get("records")
        .cloned()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let load_id = body
        .get("load_id")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);
    match rt.load_upload_flow_records(&actor, &name, records, load_id) {
        Ok(result) => marshal_result_global(&result),
        Err(e) => {
            set_last_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Execute a named aggregate view.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_aggregate_view_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    view_name: *const c_char,
) -> TeselaBuffer {
    let rt = match runtime_from_handle(handle) {
        Ok(rt) => rt,
        Err(e) => {
            set_last_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let actor = unsafe { extract_actor(actor_json, actor_len) };
    let name = match unsafe { parse_api_name(view_name) } {
        Ok(n) => n,
        Err(e) => {
            set_last_error(&e);
            return TeselaBuffer::empty();
        }
    };
    match rt.aggregate_view(&actor, &name) {
        Ok(result) => marshal_result_global(&result),
        Err(e) => {
            set_last_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Resolve an object set by name.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_resolve_object_set_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    set_name: *const c_char,
) -> TeselaBuffer {
    let mut ht = lock_handles!(or return TeselaBuffer::empty());
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid handle");
            return TeselaBuffer::empty();
        }
    };
    let actor = unsafe { extract_actor(actor_json, actor_len) };
    let name = match unsafe { parse_api_name(set_name) } {
        Ok(n) => n,
        Err(e) => {
            ht.set_error(&e);
            return TeselaBuffer::empty();
        }
    };
    match rt.resolve_object_set(&actor, &name) {
        Ok(page) => marshal_result(&page, &mut ht),
        Err(e) => {
            ht.set_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Compose object sets.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_compose_object_sets_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    body_json: *const c_char,
    body_len: c_int,
) -> TeselaBuffer {
    let mut ht = lock_handles!(or return TeselaBuffer::empty());
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid handle");
            return TeselaBuffer::empty();
        }
    };
    let actor = unsafe { extract_actor(actor_json, actor_len) };
    let body: serde_json::Value = match unsafe { decode_json(body_json, body_len) } {
        Ok(v) => v,
        Err(e) => {
            ht.set_error(&e);
            return TeselaBuffer::empty();
        }
    };

    let names: Vec<tesela_core::ApiName> = body
        .get("names")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .filter_map(|s| s.parse().ok())
                .collect()
        })
        .unwrap_or_default();

    let op = match body.get("op").and_then(|v| v.as_str()) {
        Some("intersect") => tesela_ir::SetOp::Intersect,
        Some("subtract") => tesela_ir::SetOp::Subtract,
        _ => tesela_ir::SetOp::Union,
    };

    match rt.compose_object_sets(&actor, &names, op) {
        Ok(page) => marshal_result(&page, &mut ht),
        Err(e) => {
            ht.set_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Execute a transform pipeline.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_execute_pipeline_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    pipeline_name: *const c_char,
    body_json: *const c_char,
    body_len: c_int,
) -> TeselaBuffer {
    let mut ht = lock_handles!(or return TeselaBuffer::empty());
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid handle");
            return TeselaBuffer::empty();
        }
    };
    let actor = unsafe { extract_actor(actor_json, actor_len) };
    let name = match unsafe { parse_api_name(pipeline_name) } {
        Ok(n) => n,
        Err(e) => {
            ht.set_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let body: serde_json::Value = if body_len > 0 {
        match unsafe { decode_json(body_json, body_len) } {
            Ok(v) => v,
            Err(e) => {
                ht.set_error(&e);
                return TeselaBuffer::empty();
            }
        }
    } else {
        serde_json::json!({})
    };
    let mode = match body.get("mode").and_then(|v| v.as_str()) {
        Some("snapshot") => tesela_ir::ExecutionMode::Snapshot,
        _ => tesela_ir::ExecutionMode::Incremental,
    };
    match rt.execute_pipeline(&actor, &name, mode) {
        Ok(result) => marshal_result(&result, &mut ht),
        Err(e) => {
            ht.set_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Get lineage edges for a record.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_get_lineage_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    object_name: *const c_char,
    pk_json: *const c_char,
    pk_len: c_int,
    depth: c_int,
) -> TeselaBuffer {
    let mut ht = lock_handles!(or return TeselaBuffer::empty());
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid handle");
            return TeselaBuffer::empty();
        }
    };
    let actor = unsafe { extract_actor(actor_json, actor_len) };
    let obj_name = match unsafe { parse_api_name(object_name) } {
        Ok(n) => n,
        Err(e) => {
            ht.set_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let pk: serde_json::Value = match unsafe { decode_json(pk_json, pk_len) } {
        Ok(v) => v,
        Err(e) => {
            ht.set_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let depth_opt = if depth > 0 { Some(depth as u32) } else { None };
    match rt.get_lineage(&actor, &obj_name, &Value::new(pk), depth_opt) {
        Ok(edges) => marshal_result(&edges, &mut ht),
        Err(e) => {
            ht.set_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Federated / cross-datasource search.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_cross_search_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    queries_json: *const c_char,
    queries_len: c_int,
) -> TeselaBuffer {
    let mut ht = lock_handles!(or return TeselaBuffer::empty());
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid handle");
            return TeselaBuffer::empty();
        }
    };
    let actor = unsafe { extract_actor(actor_json, actor_len) };
    let body: Vec<serde_json::Value> = match unsafe { decode_json(queries_json, queries_len) } {
        Ok(v) => v,
        Err(e) => {
            ht.set_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let mut queries = Vec::new();
    for item in &body {
        let ds_str = item
            .get("datasource")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let ot_str = item
            .get("object_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let ds = match ds_str.parse::<tesela_core::ApiName>() {
            Ok(n) => n,
            Err(e) => {
                ht.set_error(&format!("invalid datasource: {}", e));
                return TeselaBuffer::empty();
            }
        };
        let ot = match ot_str.parse::<tesela_core::ApiName>() {
            Ok(n) => n,
            Err(e) => {
                ht.set_error(&format!("invalid object_type: {}", e));
                return TeselaBuffer::empty();
            }
        };
        let query = tesela_runtime::query::Query {
            filter: item
                .get("filter")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            sort: item
                .get("sort")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default(),
            limit: item.get("limit").and_then(|v| v.as_i64()).map(|v| v as i32),
            offset: None,
            cursor: None,
        };
        queries.push(tesela_runtime::ports::FederatedQuery {
            datasource: ds,
            object_type: ot,
            query,
        });
    }
    match rt.cross_search(&actor, queries) {
        Ok(page) => marshal_result(&page, &mut ht),
        Err(e) => {
            ht.set_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Create a draft branch.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_create_branch_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    display: *const c_char,
) -> TeselaBuffer {
    let mut ht = lock_handles!(or return TeselaBuffer::empty());
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid handle");
            return TeselaBuffer::empty();
        }
    };
    let actor = unsafe { extract_actor(actor_json, actor_len) };
    let display_str = unsafe { CStr::from_ptr(display) }
        .to_str()
        .unwrap_or("")
        .to_string();
    match rt.create_branch(&actor, &display_str) {
        Ok(branch) => marshal_result(&branch, &mut ht),
        Err(e) => {
            ht.set_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Update the draft spec on a branch.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_update_branch_spec_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    branch_id: *const c_char,
    spec_json: *const c_char,
    spec_len: c_int,
) -> TeselaBuffer {
    let mut ht = lock_handles!(or return TeselaBuffer::empty());
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid handle");
            return TeselaBuffer::empty();
        }
    };
    let actor = unsafe { extract_actor(actor_json, actor_len) };
    let bid = unsafe { CStr::from_ptr(branch_id) }
        .to_str()
        .unwrap_or("")
        .to_string();
    let spec: tesela_ir::Spec = match unsafe { decode_json(spec_json, spec_len) } {
        Ok(s) => s,
        Err(e) => {
            ht.set_error(&e);
            return TeselaBuffer::empty();
        }
    };
    match rt.update_branch_spec(&actor, &bid, spec) {
        Ok(()) => marshal_result(&serde_json::json!({"status": "updated"}), &mut ht),
        Err(e) => {
            ht.set_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Merge a branch into the live spec.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_merge_branch_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    branch_id: *const c_char,
) -> TeselaBuffer {
    let mut ht = lock_handles!(or return TeselaBuffer::empty());
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid handle");
            return TeselaBuffer::empty();
        }
    };
    let actor = unsafe { extract_actor(actor_json, actor_len) };
    let bid = unsafe { CStr::from_ptr(branch_id) }
        .to_str()
        .unwrap_or("")
        .to_string();
    match rt.merge_branch(&actor, &bid) {
        Ok(diff) => marshal_result(
            &serde_json::json!({
                "status": "merged",
                "added": diff.added.len(),
                "removed": diff.removed.len(),
                "changed": diff.changed.len(),
            }),
            &mut ht,
        ),
        Err(e) => {
            ht.set_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// List all branches.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_list_branches_json(handle: u64) -> TeselaBuffer {
    let mut ht = lock_handles!(or return TeselaBuffer::empty());
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid handle");
            return TeselaBuffer::empty();
        }
    };
    match rt.list_branches() {
        Ok(branches) => marshal_result(&branches, &mut ht),
        Err(e) => {
            ht.set_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Apply spec with migration support.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_apply_spec_with_migration_json(
    handle: u64,
    spec_json: *const c_char,
    spec_len: c_int,
) -> TeselaBuffer {
    let mut ht = lock_handles!(or return TeselaBuffer::empty());
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid handle");
            return TeselaBuffer::empty();
        }
    };
    let spec: tesela_ir::Spec = match unsafe { decode_json(spec_json, spec_len) } {
        Ok(s) => s,
        Err(e) => {
            ht.set_error(&e);
            return TeselaBuffer::empty();
        }
    };
    match rt.apply_spec_with_migration(spec) {
        Ok(diff) => marshal_result(
            &serde_json::json!({
                "status": "applied",
                "added": diff.added.len(),
                "removed": diff.removed.len(),
                "changed": diff.changed.len(),
            }),
            &mut ht,
        ),
        Err(e) => {
            ht.set_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Get the schema graph as JSON.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_schema_graph_json(handle: u64) -> TeselaBuffer {
    let mut ht = lock_handles!(or return TeselaBuffer::empty());
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid handle");
            return TeselaBuffer::empty();
        }
    };
    let spec = match rt.spec() {
        Ok(s) => s,
        Err(e) => {
            ht.set_error(&e.to_string());
            return TeselaBuffer::empty();
        }
    };
    let nodes: Vec<String> = spec
        .object_types
        .iter()
        .map(|ot| ot.api_name.to_string())
        .collect();
    let edges: Vec<serde_json::Value> = spec
        .link_types
        .iter()
        .map(|lt| {
            serde_json::json!({
                "from": lt.from.to_string(),
                "to": lt.to.to_string(),
                "link": lt.api_name.to_string(),
            })
        })
        .collect();
    let result = serde_json::json!({ "nodes": nodes, "edges": edges });
    marshal_result(&result, &mut ht)
}
