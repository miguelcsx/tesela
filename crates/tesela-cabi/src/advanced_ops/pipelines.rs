use crate::handle::*;
use std::os::raw::{c_char, c_int};
use tesela_core::Value;

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
