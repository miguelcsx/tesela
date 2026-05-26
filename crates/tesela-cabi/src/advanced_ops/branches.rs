use crate::handle::*;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

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
