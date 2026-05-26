use crate::advanced_ops::helpers::*;
use crate::handle::*;
use std::os::raw::{c_char, c_int};
use tesela_core::Value;

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
