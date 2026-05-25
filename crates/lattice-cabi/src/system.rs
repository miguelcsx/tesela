//! Agent, system introspection, and registration exports.

use crate::handle::*;
use lattice_core::Value;
use serde_json::json;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_void};

/// Start an agent run.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lattice_runtime_agent_start_json(
    handle: u64,
    actor_json: *const c_char,
    actor_len: c_int,
    agent_name: *const c_char,
    input_json: *const c_char,
    input_len: c_int,
) -> LatticeBuffer {
    let mut ht = lock_handles!(or return LatticeBuffer::empty());
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid handle");
            return LatticeBuffer::empty();
        }
    };
    let actor = unsafe { extract_actor(actor_json, actor_len) };
    let name = match unsafe { parse_api_name(agent_name) } {
        Ok(n) => n,
        Err(e) => {
            ht.set_error(&e);
            return LatticeBuffer::empty();
        }
    };
    let input: serde_json::Value = if input_len > 0 {
        match unsafe { decode_json(input_json, input_len) } {
            Ok(v) => v,
            Err(e) => {
                ht.set_error(&e);
                return LatticeBuffer::empty();
            }
        }
    } else {
        json!({})
    };
    match rt.start_agent_run(&actor, &name, Value::new(input)) {
        Ok(run_id) => marshal_result(&json!({"run_id": run_id}), &mut ht),
        Err(e) => {
            ht.set_error(&e.to_string());
            LatticeBuffer::empty()
        }
    }
}

/// Get the current state of an agent run.
///
/// # Safety
/// `run_id` must be a null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lattice_runtime_agent_get_run_json(
    handle: u64,
    run_id: *const c_char,
) -> LatticeBuffer {
    let mut ht = lock_handles!(or return LatticeBuffer::empty());
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid handle");
            return LatticeBuffer::empty();
        }
    };
    let rid = match unsafe { CStr::from_ptr(run_id) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            ht.set_error(&e.to_string());
            return LatticeBuffer::empty();
        }
    };
    let actor = default_actor();
    match rt.get_agent_run(&actor, &rid) {
        Ok(run) => marshal_result(&run, &mut ht),
        Err(e) => {
            ht.set_error(&e.to_string());
            LatticeBuffer::empty()
        }
    }
}

/// Return health status JSON.
///
/// # Safety
/// `handle` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lattice_runtime_health_json(handle: u64) -> LatticeBuffer {
    let mut ht = lock_handles!(or return LatticeBuffer::empty());
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid handle");
            return LatticeBuffer::empty();
        }
    };
    match rt.health() {
        Ok(h) => marshal_result(&h, &mut ht),
        Err(e) => {
            ht.set_error(&e.to_string());
            LatticeBuffer::empty()
        }
    }
}

/// Return capabilities JSON.
///
/// # Safety
/// `handle` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lattice_runtime_capabilities_json(handle: u64) -> LatticeBuffer {
    let mut ht = lock_handles!(or return LatticeBuffer::empty());
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid handle");
            return LatticeBuffer::empty();
        }
    };
    marshal_result(&rt.capabilities(), &mut ht)
}

/// Free a pointer previously returned by a user-registered callback.
///
/// # Safety
/// `ptr` must have been returned by a callback registered via
/// [`lattice_runtime_register_backend`], [`lattice_runtime_register_action_handler`],
/// or [`lattice_runtime_register_custom_tool`], and not already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lattice_callback_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { libc::free(ptr as *mut c_void) };
    }
}

/// Register a user-implemented backend for a given adapter type.
///
/// The callback receives JSON requests like
/// `{"op":"search","object_type":"user","query":{...}}`
/// and must return JSON responses. Use [`lattice_callback_free`] to free the
/// returned pointer.
///
/// # Safety
/// `handle` must be valid; `adapter_type` a null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lattice_runtime_register_backend(
    handle: u64,
    adapter_type: *const c_char,
    callback: CallbackFn,
    user_data: *mut c_void,
) -> *mut c_char {
    let mut ht = lock_handles!(or return c_error_string("handle table lock poisoned"));
    if ht.get(handle).is_none() {
        return c_error_string("invalid runtime handle");
    }
    let at = match unsafe { CStr::from_ptr(adapter_type) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => return c_error_string(&e.to_string()),
    };
    ht.backend_callbacks.insert(
        at,
        CCallback {
            callback,
            user_data,
        },
    );
    std::ptr::null_mut()
}

/// Register an action handler callback for the given action kind.
///
/// # Safety
/// `handle` must be valid; `kind` a null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lattice_runtime_register_action_handler(
    handle: u64,
    kind: *const c_char,
    callback: CallbackFn,
    user_data: *mut c_void,
) -> *mut c_char {
    let mut ht = lock_handles!(or return c_error_string("handle table lock poisoned"));
    if ht.get(handle).is_none() {
        return c_error_string("invalid runtime handle");
    }
    let k = match unsafe { CStr::from_ptr(kind) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => return c_error_string(&e.to_string()),
    };
    ht.action_callbacks.insert(
        k,
        CCallback {
            callback,
            user_data,
        },
    );
    std::ptr::null_mut()
}

/// Register a custom tool callback.
///
/// # Safety
/// `handle` must be valid; `name` a null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lattice_runtime_register_custom_tool(
    handle: u64,
    name: *const c_char,
    callback: CallbackFn,
    user_data: *mut c_void,
) -> *mut c_char {
    let mut ht = lock_handles!(or return c_error_string("handle table lock poisoned"));
    if ht.get(handle).is_none() {
        return c_error_string("invalid runtime handle");
    }
    let n = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => return c_error_string(&e.to_string()),
    };
    ht.custom_tool_callbacks.insert(
        n,
        CCallback {
            callback,
            user_data,
        },
    );
    std::ptr::null_mut()
}

/// Register an object-store callback.
///
/// # Safety
/// `handle` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lattice_runtime_register_object_store(
    handle: u64,
    name: *const c_char,
    callback: CallbackFn,
    user_data: *mut c_void,
) -> *mut c_char {
    let mut ht = lock_handles!(or return c_error_string("handle table lock poisoned"));
    if ht.get(handle).is_none() {
        return c_error_string("invalid runtime handle");
    }
    let key = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok("") => "default".to_string(),
        Ok(s) => s.to_string(),
        Err(e) => return c_error_string(&e.to_string()),
    };
    ht.object_store_callbacks.insert(
        key,
        CCallback {
            callback,
            user_data,
        },
    );
    std::ptr::null_mut()
}

/// Register a message-bus callback.
///
/// # Safety
/// `handle` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lattice_runtime_register_message_bus(
    handle: u64,
    name: *const c_char,
    callback: CallbackFn,
    user_data: *mut c_void,
) -> *mut c_char {
    let mut ht = lock_handles!(or return c_error_string("handle table lock poisoned"));
    if ht.get(handle).is_none() {
        return c_error_string("invalid runtime handle");
    }
    let key = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok("") => "default".to_string(),
        Ok(s) => s.to_string(),
        Err(e) => return c_error_string(&e.to_string()),
    };
    ht.message_bus_callbacks.insert(
        key,
        CCallback {
            callback,
            user_data,
        },
    );
    std::ptr::null_mut()
}

/// Register a run-store callback.
///
/// # Safety
/// `handle` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lattice_runtime_register_run_store(
    handle: u64,
    name: *const c_char,
    callback: CallbackFn,
    user_data: *mut c_void,
) -> *mut c_char {
    let mut ht = lock_handles!(or return c_error_string("handle table lock poisoned"));
    if ht.get(handle).is_none() {
        return c_error_string("invalid runtime handle");
    }
    let key = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok("") => "default".to_string(),
        Ok(s) => s.to_string(),
        Err(e) => return c_error_string(&e.to_string()),
    };
    ht.run_store_callbacks.insert(
        key,
        CCallback {
            callback,
            user_data,
        },
    );
    std::ptr::null_mut()
}

/// Register a capability issuer callback.
///
/// # Safety
/// `handle` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lattice_runtime_register_capability_issuer(
    handle: u64,
    name: *const c_char,
    callback: CallbackFn,
    user_data: *mut c_void,
) -> *mut c_char {
    let mut ht = lock_handles!(or return c_error_string("handle table lock poisoned"));
    if ht.get(handle).is_none() {
        return c_error_string("invalid runtime handle");
    }
    let key = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok("") => "default".to_string(),
        Ok(s) => s.to_string(),
        Err(e) => return c_error_string(&e.to_string()),
    };
    ht.capability_callbacks.insert(
        key,
        CCallback {
            callback,
            user_data,
        },
    );
    std::ptr::null_mut()
}

/// Register an action handler callback for a specific action name.
///
/// # Safety
/// `handle` must be valid; `action_name` a null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lattice_runtime_register_action(
    handle: u64,
    action_name: *const c_char,
    callback: CallbackFn,
    user_data: *mut c_void,
) -> *mut c_char {
    let mut ht = lock_handles!(or return c_error_string("handle table lock poisoned"));
    if ht.get(handle).is_none() {
        return c_error_string("invalid runtime handle");
    }
    let n = match unsafe { CStr::from_ptr(action_name) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => return c_error_string(&e.to_string()),
    };
    ht.action_by_name_callbacks.insert(
        n,
        CCallback {
            callback,
            user_data,
        },
    );
    std::ptr::null_mut()
}
