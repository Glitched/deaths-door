//! Poison-tolerant locking for `std::sync::Mutex`.
//!
//! The mutexes that guard hardware/IO bookkeeping (DMX universe state, APNS
//! token set and push throttle) are held across small, infallible critical
//! sections. If a thread ever *did* panic while holding one, a plain
//! `lock().unwrap()` would poison the mutex and turn every later access into a
//! cascading panic — silently wedging that subsystem for the rest of the run.
//! Recovering the guard instead keeps the subsystem alive; the data is still
//! memory-safe, just possibly mid-update.

use std::sync::{Mutex, MutexGuard};

pub trait LockExt<T> {
    /// Lock, recovering the guard even if the mutex was poisoned by a panic in
    /// another thread.
    fn lock_recover(&self) -> MutexGuard<'_, T>;
}

impl<T> LockExt<T> for Mutex<T> {
    fn lock_recover(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
