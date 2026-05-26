//! Data operation exports (search, get, mutate, actions, explain, traverse, aggregate, upload).

use crate::handle::*;
use tesela_core::Value;
use serde_json::json;
use std::os::raw::{c_char, c_int, c_void};

/// Search an object type.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_search_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    object_name: *const c_char,
    query_json: *const c_char,
    query_len: c_int,
) -> TeselaBuffer {
    let rt = {
        let mut ht = lock_handles!(or return TeselaBuffer::empty());
        match ht.get(handle).cloned() {
            Some(r) => r,
            None => {
                ht.set_error("invalid handle");
                return TeselaBuffer::empty();
            }
        }
    };
    let actor = unsafe { extract_actor(actor_json, actor_len) };
    let obj_name = match unsafe { parse_api_name(object_name) } {
        Ok(n) => n,
        Err(e) => {
            set_last_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let query: tesela_runtime::query::Query = match unsafe { decode_json(query_json, query_len) } {
        Ok(q) => q,
        Err(e) => {
            set_last_error(&e);
            return TeselaBuffer::empty();
        }
    };
    match rt.search(&actor, &obj_name, query) {
        Ok(page) => marshal_result_global(&page),
        Err(e) => {
            set_last_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Get a single record by primary key.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_get_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    object_name: *const c_char,
    pk_json: *const c_char,
    pk_len: c_int,
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
    match rt.get(&actor, &obj_name, &Value::new(pk)) {
        Ok(record) => marshal_result(&record, &mut ht),
        Err(e) => {
            ht.set_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Mutate (create / update / delete) records.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_mutate_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    object_name: *const c_char,
    mutation_json: *const c_char,
    mutation_len: c_int,
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
    let mutation: tesela_runtime::query::Mutation =
        match unsafe { decode_json(mutation_json, mutation_len) } {
            Ok(m) => m,
            Err(e) => {
                ht.set_error(&e);
                return TeselaBuffer::empty();
            }
        };
    match rt.mutate(&actor, &obj_name, mutation) {
        Ok(result) => marshal_result(&result, &mut ht),
        Err(e) => {
            ht.set_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Execute an action.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_execute_action_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    action_name: *const c_char,
    input_json: *const c_char,
    input_len: c_int,
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
    let name = match unsafe { parse_api_name(action_name) } {
        Ok(n) => n,
        Err(e) => {
            ht.set_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let input: serde_json::Value = if input_len > 0 {
        match unsafe { decode_json(input_json, input_len) } {
            Ok(v) => v,
            Err(e) => {
                ht.set_error(&e);
                return TeselaBuffer::empty();
            }
        }
    } else {
        json!({})
    };
    // Check for a CABI-registered action handler first
    if let Some(cb) = ht.action_by_name_callbacks.get(name.as_ref()).cloned() {
        let req = tesela_runtime::query::ActionRequest {
            action: name.clone(),
            input: Value::new(input),
            actor: actor.clone(),
            run_id: None,
        };
        let req_json = match serde_json::to_vec(&req) {
            Ok(v) => v,
            Err(e) => {
                ht.set_error(&e.to_string());
                return TeselaBuffer::empty();
            }
        };
        let mut out_len: c_int = 0;
        let resp_ptr = (cb.callback)(
            cb.user_data,
            req_json.as_ptr() as *const c_char,
            req_json.len() as c_int,
            &mut out_len,
        );
        if resp_ptr.is_null() || out_len <= 0 {
            ht.set_error("action callback returned null");
            return TeselaBuffer::empty();
        }
        let slice = unsafe { std::slice::from_raw_parts(resp_ptr as *const u8, out_len as usize) };
        let buf = TeselaBuffer::from_bytes(slice.to_vec());
        unsafe { libc::free(resp_ptr as *mut c_void) };
        return buf;
    }

    match rt.execute_action(&actor, &name, Value::new(input)) {
        Ok(result) => marshal_result(&result, &mut ht),
        Err(e) => {
            ht.set_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Explain a search query.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_explain_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    object_name: *const c_char,
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
    let obj_name = match unsafe { parse_api_name(object_name) } {
        Ok(n) => n,
        Err(e) => {
            ht.set_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let query: tesela_runtime::query::Query = match unsafe { decode_json(query_json, query_len) } {
        Ok(q) => q,
        Err(e) => {
            ht.set_error(&e);
            return TeselaBuffer::empty();
        }
    };
    match rt.explain(&actor, &obj_name, query) {
        Ok(plan) => marshal_result(&plan, &mut ht),
        Err(e) => {
            ht.set_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Traverse a link type.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_traverse_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    link_name: *const c_char,
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
    let lname = match unsafe { parse_api_name(link_name) } {
        Ok(n) => n,
        Err(e) => {
            ht.set_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let query: tesela_runtime::query::TraversalQuery =
        match unsafe { decode_json(query_json, query_len) } {
            Ok(q) => q,
            Err(e) => {
                ht.set_error(&e);
                return TeselaBuffer::empty();
            }
        };
    match rt.traverse(&actor, &lname, query) {
        Ok(page) => marshal_result(&page, &mut ht),
        Err(e) => {
            ht.set_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Run an aggregation query.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_aggregate_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    object_name: *const c_char,
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
    let obj_name = match unsafe { parse_api_name(object_name) } {
        Ok(n) => n,
        Err(e) => {
            ht.set_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let query: tesela_runtime::query::AggregateQuery =
        match unsafe { decode_json(query_json, query_len) } {
            Ok(q) => q,
            Err(e) => {
                ht.set_error(&e);
                return TeselaBuffer::empty();
            }
        };
    match rt.aggregate(&actor, &obj_name, query) {
        Ok(result) => marshal_result(&result, &mut ht),
        Err(e) => {
            ht.set_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Rollback a bulk upload by load ID.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_rollback_upload_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    object_name: *const c_char,
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
    let obj_name = match unsafe { parse_api_name(object_name) } {
        Ok(n) => n,
        Err(e) => {
            ht.set_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let body: serde_json::Value = match unsafe { decode_json(body_json, body_len) } {
        Ok(v) => v,
        Err(e) => {
            ht.set_error(&e);
            return TeselaBuffer::empty();
        }
    };
    let load_id = body
        .get("load_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    match rt.rollback_upload(&actor, &obj_name, &load_id) {
        Ok(()) => {
            let response = json!({"status": "rolled_back", "load_id": load_id});
            marshal_result(&response, &mut ht)
        }
        Err(e) => {
            ht.set_error(&e.to_string());
            TeselaBuffer::empty()
        }
    }
}

/// Bulk upload (not yet implemented).
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_upload_json(
    _handle: u64,
    _actor_json: *const c_char,
    _actor_len: c_int,
    _object_name: *const c_char,
    _upload_json: *const c_char,
    _upload_len: c_int,
    _content: *const c_char,
    _content_len: c_int,
) -> TeselaBuffer {
    let err =
        json!({"error": {"message": "upload not yet implemented", "code": "not_implemented"}});
    TeselaBuffer::from_bytes(serde_json::to_vec(&err).unwrap_or_default())
}
