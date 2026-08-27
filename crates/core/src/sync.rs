//! DUP-3 / TASK-1477 + CONC-9: shared `Mutex` poison-recover policy.
//!
//! Every `Mutex` in this crate protects a cache or deduplication set whose
//! every possible state is a valid map / set — no invariant the panicking
//! caller could have broken. Calling `.lock().expect(...)` on those would
//! turn a single panic-inside-lock into a hard panic for the rest of the
//! process; the project-wide policy is therefore to `clear_poison()` and
//! continue with the recovered guard. This module factors that pattern
//! into a single helper so the four (and counting) callsites cannot drift.
//!
//! Two helpers are exposed:
//!
//! - [`lock_recover`] — silent recovery, used by production hot paths whose
//!   protected state is documented as "every state is valid" (the
//!   workspace-root cache, the warn-seen set, the canonicalize cache).
//! - [`lock_recover_warn`] — surfaces a `tracing::warn!` breadcrumb with the
//!   supplied label when the lock was poisoned. Used by test-support seams
//!   (`ops_root_cache_len`, `expand_warn_seen_count`, …) so a flake stemming
//!   from a sibling panic is visible at the right level instead of being
//!   swallowed.

use std::sync::{Mutex, MutexGuard};

/// Acquire `m`, recovering silently from poisoning.
///
/// Use in production hot paths whose protected state is a cache or
/// dedup set with no broken invariant. Tests and seams that need a
/// breadcrumb when poison was observed should use [`lock_recover_warn`]
/// instead.
pub fn lock_recover<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| {
        m.clear_poison();
        e.into_inner()
    })
}

/// Acquire `m`, recovering from poisoning and emitting a
/// `tracing::warn!` tagged with `label` so the recovery event is visible.
///
/// Use in test-support seams (or any callsite where a future flake would
/// otherwise look like a value-mismatch failure rather than the poison
/// that actually caused it). Gated to `#[cfg(test)]` because all current
/// callers live behind that gate; production code uses
/// [`lock_recover`] for the silent-recovery policy.
#[cfg(test)]
pub fn lock_recover_warn<'a, T>(m: &'a Mutex<T>, label: &'static str) -> MutexGuard<'a, T> {
    match m.lock() {
        Ok(g) => g,
        Err(e) => {
            m.clear_poison();
            tracing::warn!(
                site = label,
                "mutex was poisoned by a previous panicking holder; recovered guard returned"
            );
            e.into_inner()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Poison `m` from a scoped thread that panics while holding the guard,
    /// then return so the caller observes a poisoned lock. Scoped rather than
    /// `thread::spawn` so the mutex can live on the stack — the whole point of
    /// these tests is that they touch no process-global state.
    fn poison(m: &Mutex<u32>) {
        let joined = std::thread::scope(|s| {
            s.spawn(|| {
                let _guard = m.lock().expect("uncontended lock");
                panic!("synthetic poison");
            })
            .join()
        });
        assert!(joined.is_err(), "the poisoning thread must have panicked");
        assert!(
            m.is_poisoned(),
            "lock must be poisoned for this test to mean anything"
        );
    }

    /// TEST-15 / TASK-1664: this is the deterministic replacement for
    /// `expand::tests::ops_root_cache_len_surfaces_poison_breadcrumb`, which
    /// poisoned the *process-global* `OPS_ROOT` cache and asserted on the
    /// breadcrumb. `lock_recover_warn` calls `clear_poison`, so a poisoned
    /// lock yields exactly one breadcrumb and whichever caller recovers first
    /// consumes it. Fifteen unserialised tests in `expand` reach that cache
    /// through `test_vars` → `from_env` → `cached_ops_root_arc`, so the old
    /// test was racing all of them: it passed on an idle workstation and
    /// failed reliably on a 2-core CI runner. Serialising the cache tests
    /// against each other did not help, because the fifteen are not
    /// serialised at all.
    ///
    /// Testing the seam against a stack-local mutex removes the coupling
    /// entirely: nothing else can reach this lock, so the assertion cannot
    /// race.
    #[test]
    fn lock_recover_warn_emits_breadcrumb_naming_the_seam() {
        let m = Mutex::new(7u32);
        poison(&m);

        let (logs, value) = crate::test_utils::capture_tracing(tracing::Level::WARN, || {
            *lock_recover_warn(&m, "test_seam")
        });

        assert_eq!(value, 7, "recovered guard must still expose the value");
        assert!(
            logs.contains("test_seam"),
            "warn breadcrumb must name the seam, got: {logs}"
        );
        assert!(
            logs.contains("poisoned"),
            "warn breadcrumb must mention the poison recovery, got: {logs}"
        );
    }

    /// The breadcrumb is for the poison path only — a healthy lock must stay
    /// silent, or every seam call would spam the warn channel.
    #[test]
    fn lock_recover_warn_is_silent_on_a_healthy_lock() {
        let m = Mutex::new(7u32);

        let (logs, value) = crate::test_utils::capture_tracing(tracing::Level::WARN, || {
            *lock_recover_warn(&m, "test_seam")
        });

        assert_eq!(value, 7);
        assert!(logs.is_empty(), "healthy lock must not warn, got: {logs}");
    }

    /// `lock_recover` is the silent sibling: it must recover the value and
    /// clear the poison without emitting anything.
    #[test]
    fn lock_recover_recovers_silently_and_clears_poison() {
        let m = Mutex::new(7u32);
        poison(&m);

        let (logs, value) =
            crate::test_utils::capture_tracing(tracing::Level::WARN, || *lock_recover(&m));

        assert_eq!(value, 7, "recovered guard must still expose the value");
        assert!(
            logs.is_empty(),
            "lock_recover must stay silent, got: {logs}"
        );
        assert!(
            !m.is_poisoned(),
            "poison must be cleared so later callers take the fast path"
        );
    }
}
