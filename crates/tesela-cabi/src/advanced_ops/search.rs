use crate::handle::*;
use std::os::raw::{c_char, c_int};

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
