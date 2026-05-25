//! Runtime lifecycle and spec management exports.

use crate::handle::*;
use std::os::raw::{c_char, c_int};

/// Create a new runtime from a JSON-encoded spec.
///
/// Returns an opaque `u64` handle (non-zero on success, 0 on failure).
///
/// # Safety
/// `spec_json` must be a valid UTF-8 buffer of at least `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn lattice_runtime_new_from_spec_json(
    spec_json: *const c_char,
    len: c_int,
) -> u64 {
    let spec = match parse_spec(spec_json, len) {
        Ok(s) => s,
        Err(e) => {
            lock_handles!(or return 0).set_error(&e);
            return 0;
        }
    };
    let callbacks = {
        let ht = lock_handles!(or return 0);
        ht.backend_callbacks.clone()
    };
    match build_runtime(spec, &callbacks) {
        Ok(rt) => lock_handles!(or return 0).insert(rt),
        Err(e) => {
            lock_handles!(or return 0).set_error(&e);
            0
        }
    }
}

/// Release a runtime handle and free associated resources.
///
/// # Safety
/// `handle` must be a value previously returned by
/// [`lattice_runtime_new_from_spec_json`].
#[no_mangle]
pub unsafe extern "C" fn lattice_runtime_release(handle: u64) {
    lock_handles!(or return).remove(handle);
}

/// Gracefully shut down the runtime.
///
/// # Safety
/// `handle` must be valid.
#[no_mangle]
pub unsafe extern "C" fn lattice_runtime_shutdown(handle: u64) -> *mut c_char {
    lock_handles!(or return std::ptr::null_mut()).remove(handle);
    std::ptr::null_mut()
}

/// Return the current spec as a JSON buffer.
///
/// # Safety
/// `handle` must be valid.
#[no_mangle]
pub unsafe extern "C" fn lattice_runtime_spec_json(handle: u64) -> LatticeBuffer {
    let mut ht = lock_handles!(or return LatticeBuffer::empty());
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid runtime handle");
            return LatticeBuffer::empty();
        }
    };
    let spec = match rt.spec() {
        Ok(s) => s,
        Err(e) => {
            ht.set_error(&e.to_string());
            return LatticeBuffer::empty();
        }
    };
    match serde_json::to_vec(&spec) {
        Ok(b) => LatticeBuffer::from_bytes(b),
        Err(e) => {
            ht.set_error(&e.to_string());
            LatticeBuffer::empty()
        }
    }
}

/// Apply a new spec JSON to the runtime, returning a diff JSON buffer.
///
/// # Safety
/// All pointers must be valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn lattice_runtime_apply_spec_json(
    handle: u64,
    spec_json: *const c_char,
    len: c_int,
) -> LatticeBuffer {
    let mut ht = lock_handles!(or return LatticeBuffer::empty());
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid runtime handle");
            return LatticeBuffer::empty();
        }
    };
    let new_spec = match parse_spec(spec_json, len) {
        Ok(s) => s,
        Err(e) => {
            ht.set_error(&e);
            return LatticeBuffer::empty();
        }
    };
    match rt.apply_spec(new_spec) {
        Ok(diff) => marshal_result(&diff, &mut ht),
        Err(e) => {
            ht.set_error(&e.to_string());
            LatticeBuffer::empty()
        }
    }
}
