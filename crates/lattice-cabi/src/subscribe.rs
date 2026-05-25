//! Subscription CABI exports — event bus and change-stream polling.

use crate::handle::{lock_handles, parse_api_name, LatticeBuffer, Subscription};
use std::os::raw::{c_char, c_int};
use std::time::Duration;

/// Subscribe to real-time domain events for `object_type`.
///
/// Returns a subscription handle (opaque u64). Use
/// [`lattice_runtime_subscribe_poll`] to read events and
/// [`lattice_runtime_subscribe_close`] to release.
///
/// # Safety
/// `object_name` must be a valid null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lattice_runtime_subscribe_json(
    handle: u64,
    _actor_json: *const c_char,
    _actor_len: c_int,
    object_name: *const c_char,
) -> u64 {
    let mut ht = lock_handles!(or return 0);
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid handle");
            return 0;
        }
    };
    let obj = if object_name.is_null() {
        None
    } else {
        match unsafe { parse_api_name(object_name) } {
            Ok(n) => Some(n),
            Err(e) => {
                ht.set_error(&e);
                return 0;
            }
        }
    };
    match rt.subscribe(obj.as_ref()) {
        Ok(rx) => ht.insert_sub(Subscription::Event(rx)),
        Err(e) => {
            ht.set_error(&e.to_string());
            0
        }
    }
}

/// Subscribe to CDC change events for `object_type`.
///
/// Returns a subscription handle. Same polling pattern as above.
///
/// # Safety
/// `object_name` must be a valid null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lattice_runtime_subscribe_changes_json(
    handle: u64,
    _actor_json: *const c_char,
    _actor_len: c_int,
    object_name: *const c_char,
) -> u64 {
    let mut ht = lock_handles!(or return 0);
    let rt = match ht.get(handle).cloned() {
        Some(r) => r,
        None => {
            ht.set_error("invalid handle");
            return 0;
        }
    };
    let obj = match unsafe { parse_api_name(object_name) } {
        Ok(n) => n,
        Err(e) => {
            ht.set_error(&e);
            return 0;
        }
    };
    match rt.subscribe_changes(&obj) {
        Ok(rx) => ht.insert_sub(Subscription::Change(rx)),
        Err(e) => {
            ht.set_error(&e.to_string());
            0
        }
    }
}

/// Poll a subscription for the next event.
///
/// `timeout_ms` controls blocking behaviour:
/// * `0`   — non-blocking (returns empty buffer immediately if no event).
/// * `>0`  — block up to `timeout_ms` milliseconds.
/// * `-1`  — block indefinitely until an event arrives.
///
/// An empty buffer with no error means timeout / no event.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lattice_runtime_subscribe_poll(
    sub_handle: u64,
    timeout_ms: c_int,
) -> LatticeBuffer {
    let mut ht = lock_handles!(or return LatticeBuffer::empty());
    let sub = match ht.get_sub(sub_handle) {
        Some(s) => s,
        None => {
            ht.set_error("invalid subscription handle");
            return LatticeBuffer::empty();
        }
    };
    let dur = if timeout_ms < 0 {
        None
    } else if timeout_ms == 0 {
        Some(Duration::ZERO)
    } else {
        Some(Duration::from_millis(timeout_ms as u64))
    };

    let json_bytes: Option<Vec<u8>> = match sub {
        Subscription::Event(rx) => {
            let ev = if let Some(d) = dur {
                if d == Duration::ZERO {
                    rx.try_recv().ok()
                } else {
                    rx.recv_timeout(d).ok()
                }
            } else {
                rx.recv().ok()
            };
            ev.and_then(|e| serde_json::to_vec(&e).ok())
        }
        Subscription::Change(rx) => {
            let ev = if let Some(d) = dur {
                if d == Duration::ZERO {
                    rx.try_recv().ok()
                } else {
                    rx.recv_timeout(d).ok()
                }
            } else {
                rx.recv().ok()
            };
            ev.and_then(|e| serde_json::to_vec(&e).ok())
        }
    };

    match json_bytes {
        Some(bytes) => LatticeBuffer::from_bytes(bytes),
        None => LatticeBuffer::empty(),
    }
}

/// Close and drop a subscription.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lattice_runtime_subscribe_close(sub_handle: u64) {
    let mut ht = lock_handles!(or return);
    ht.remove_sub(sub_handle);
}
