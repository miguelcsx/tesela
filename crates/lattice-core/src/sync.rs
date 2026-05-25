//! Lock acquisition helpers that return [`Error`] instead of panicking.

use crate::Error;
use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Acquire a [`Mutex`] read lock, returning an error if poisoned.
pub fn lock_mutex<T>(lock: &Mutex<T>) -> Result<MutexGuard<'_, T>, Error> {
    lock.lock()
        .map_err(|_| Error::internal("mutex lock poisoned"))
}

/// Acquire an [`RwLock`] read lock, returning an error if poisoned.
pub fn lock_read<T>(lock: &RwLock<T>) -> Result<RwLockReadGuard<'_, T>, Error> {
    lock.read()
        .map_err(|_| Error::internal("rwlock read lock poisoned"))
}

/// Acquire an [`RwLock`] write lock, returning an error if poisoned.
pub fn lock_write<T>(lock: &RwLock<T>) -> Result<RwLockWriteGuard<'_, T>, Error> {
    lock.write()
        .map_err(|_| Error::internal("rwlock write lock poisoned"))
}
