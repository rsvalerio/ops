//! Shared `Mutex` poison policy: propagate, never recover.
//!
//! A poisoned `Mutex` means a thread panicked while holding the guard, so
//! the protected value may be mid-update (a cache `map` and its eviction
//! `order` queue out of step, a dedup set half-inserted). Nothing in the
//! type tells the next reader which half was applied, so the workspace
//! never reaches inside a poisoned lock: no `PoisonError::into_inner`, no
//! `get_ref`/`get_mut`, no `clear_poison()`. The poisoning panic
//! is re-raised in every later holder instead, and surfaces through the
//! normal failure path (a `spawn_blocking` `JoinError`, a failed step, or
//! the process exit).
//!
//! "This lock guards no multi-step invariant" is deliberately *not* an
//! exemption: it is a claim about today's critical sections that nobody
//! re-checks when one grows a second statement — several of the caches
//! this module serves already do evict-then-insert under one guard.
//!
//! [`lock`] is the workspace's single statement of that policy. It is
//! public so extension crates use it instead of growing per-crate copies.

use std::sync::{Mutex, MutexGuard};

/// Acquire `m`, propagating poison as a panic naming the lock.
///
/// `name` identifies the lock in the panic message so a poisoned-lock
/// failure points at the seam that observed it, not at a later
/// value-mismatch.
///
/// # Panics
///
/// Panics when `m` is poisoned, i.e. when a previous holder panicked while
/// holding the guard. That panic *is* the handling: the protected
/// state may be torn, so continuing with it is never correct.
#[track_caller]
#[expect(
    clippy::panic,
    reason = "CONC-16: a poisoned lock is propagated, never recovered; the panic is the handling"
)]
pub fn lock<'a, T: ?Sized>(m: &'a Mutex<T>, name: &'static str) -> MutexGuard<'a, T> {
    m.lock().unwrap_or_else(|_| panic!("{name} mutex poisoned by a panicking holder"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Poison `m` from a scoped thread that panics while holding the guard,
    /// then return so the caller observes a poisoned lock. Scoped rather than
    /// `thread::spawn` so the mutex can live on the stack — the tests touch no
    /// process-global state.
    fn poison(m: &Mutex<u32>) {
        let joined = std::thread::scope(|s| {
            s.spawn(|| {
                let _guard = m.lock().expect("uncontended lock");
                panic!("synthetic poison");
            })
            .join()
        });
        assert!(joined.is_err(), "the poisoning thread must have panicked");
        assert!(m.is_poisoned(), "lock must be poisoned for this test to mean anything");
    }

    #[test]
    fn lock_hands_out_the_guard_on_a_healthy_lock() {
        let m = Mutex::new(7u32);
        assert_eq!(*lock(&m, "test_seam"), 7);
    }

    /// A poisoned lock is propagated as a panic naming the seam, and the
    /// poison flag is left in place so every later holder fails too.
    #[test]
    fn lock_propagates_poison_as_a_panic_naming_the_seam() {
        let m = Mutex::new(7u32);
        poison(&m);

        let err = std::panic::catch_unwind(|| *lock(&m, "test_seam"))
            .expect_err("a poisoned lock must not hand out its guard");
        let msg = err
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| err.downcast_ref::<&str>().copied())
            .unwrap_or_default();
        assert!(msg.contains("test_seam"), "panic must name the seam, got: {msg}");
        assert!(msg.contains("poisoned"), "panic must say the lock was poisoned, got: {msg}");
        assert!(m.is_poisoned(), "poison must not be cleared");
    }
}
