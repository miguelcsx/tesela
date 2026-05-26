use crate::handle::*;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

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

/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_add_entity_json(
    handle: u64,
    kind: *const c_char,
    entity_json: *const c_char,
    entity_len: c_int,
) -> TeselaBuffer {
    let mut ht = lock_handles!(or return TeselaBuffer::empty());
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid handle");
            return TeselaBuffer::empty();
        }
    };
    let kind_str = match unsafe { CStr::from_ptr(kind) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            ht.set_error("invalid kind string");
            return TeselaBuffer::empty();
        }
    };
    let diff_result = match kind_str {
        "object_type" => {
            let item: tesela_ir::ObjectType = match unsafe { decode_json(entity_json, entity_len) }
            {
                Ok(v) => v,
                Err(e) => {
                    ht.set_error(&e);
                    return TeselaBuffer::empty();
                }
            };
            rt.add_object_type(item)
        }
        "link_type" => {
            let item: tesela_ir::LinkType = match unsafe { decode_json(entity_json, entity_len) } {
                Ok(v) => v,
                Err(e) => {
                    ht.set_error(&e);
                    return TeselaBuffer::empty();
                }
            };
            rt.add_link_type(item)
        }
        "action" => {
            let item: tesela_ir::ActionType = match unsafe { decode_json(entity_json, entity_len) }
            {
                Ok(v) => v,
                Err(e) => {
                    ht.set_error(&e);
                    return TeselaBuffer::empty();
                }
            };
            rt.add_action(item)
        }
        "policy" => {
            let item: tesela_ir::PolicyRule = match unsafe { decode_json(entity_json, entity_len) }
            {
                Ok(v) => v,
                Err(e) => {
                    ht.set_error(&e);
                    return TeselaBuffer::empty();
                }
            };
            rt.add_policy(item)
        }
        "agent" => {
            let item: tesela_ir::Agent = match unsafe { decode_json(entity_json, entity_len) } {
                Ok(v) => v,
                Err(e) => {
                    ht.set_error(&e);
                    return TeselaBuffer::empty();
                }
            };
            rt.add_agent(item)
        }
        "trait" => {
            let item: tesela_ir::Trait = match unsafe { decode_json(entity_json, entity_len) } {
                Ok(v) => v,
                Err(e) => {
                    ht.set_error(&e);
                    return TeselaBuffer::empty();
                }
            };
            rt.add_trait_def(item)
        }
        "pipeline" => {
            let item: tesela_ir::TransformPipeline =
                match unsafe { decode_json(entity_json, entity_len) } {
                    Ok(v) => v,
                    Err(e) => {
                        ht.set_error(&e);
                        return TeselaBuffer::empty();
                    }
                };
            rt.add_pipeline(item)
        }
        other => {
            ht.set_error(&format!("unknown entity kind: '{}'", other));
            return TeselaBuffer::empty();
        }
    };
    match diff_result {
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

/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tesela_runtime_remove_entity_json(
    handle: u64,
    kind: *const c_char,
    api_name: *const c_char,
) -> TeselaBuffer {
    let mut ht = lock_handles!(or return TeselaBuffer::empty());
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid handle");
            return TeselaBuffer::empty();
        }
    };
    let kind_str = match unsafe { CStr::from_ptr(kind) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            ht.set_error("invalid kind string");
            return TeselaBuffer::empty();
        }
    };
    let name_str = match unsafe { CStr::from_ptr(api_name) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            ht.set_error("invalid api_name string");
            return TeselaBuffer::empty();
        }
    };
    let parsed_name = match name_str.parse::<tesela_core::ApiName>() {
        Ok(n) => n,
        Err(e) => {
            ht.set_error(&e.to_string());
            return TeselaBuffer::empty();
        }
    };
    match rt.remove_entity(kind_str, &parsed_name) {
        Ok(diff) => marshal_result(
            &serde_json::json!({
                "status": "removed",
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
