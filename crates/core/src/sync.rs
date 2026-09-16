//! DUP-3 / TASK-1477 + CONC-9: shared `Mutex` poison-recover policy.
//!
//! Every `Mutex` this module serves protects a cache or deduplication set
//! whose every possible state is a valid map / set — no invariant the
//! panicking caller could have broken. Calling `.lock().expect(...)` on
//! those would turn a single panic-inside-lock into a hard panic for the
//! rest of the process; the project-wide policy is therefore to
//! `clear_poison()` and continue with the recovered guard. This module
//! factors that pattern into a single helper so the callsites cannot drift.
//!
//! DUP-1 / TASK-2258: this is the workspace's *only* statement of that
//! policy. It is public so extension crates reach the helpers directly
//! instead of growing per-crate copies (`ops_about::lru::lock_recovering`
//! was one such copy, since deleted).
//!
//! Three helpers are exposed:
//!
//! - [`lock_recover`] — silent recovery, used by production hot paths whose
//!   protected state is documented as "every state is valid" (the
//!   workspace-root cache, the warn-seen set, the canonicalize cache).
//! - [`lock_recover_warn`] — surfaces a `tracing::warn!` breadcrumb with
//!   the supplied label when the lock was poisoned. Used by test-support
//!   seams (`ops_root_cache_len`, `expand_warn_seen_count`, …) and by
//!   callers whose worst outcome deserves a visible breadcrumb.
//! - [`lock_recover_with`] — the general hook form the other two delegate
//!   to; the hook runs once per *observed* poisoning.

use std::sync::{Mutex, MutexGuard};

/// Acquire `m`, recovering from poisoning; `on_poison` runs once per
/// *observed* poisoning, before the guard is handed out.
///
/// A poisoned mutex means some thread panicked while holding it. For the
/// plain-data caches this policy serves, the guarded value cannot be torn
/// by the panic, so `PoisonError::into_inner` recovery is safe and
/// propagating the poison would turn one unrelated panic into a crash.
/// `on_poison` runs before the guard is handed out — pass a warn (with a
/// monotonic counter, as the typed-manifest cache does) when a poisoned
/// lock guards correctness-relevant state, or a no-op when the worst
/// outcome is a recomputation. The sticky poison flag is cleared
/// afterwards, so later callers see a healthy mutex; a fresh panic
/// re-poisons and re-runs `on_poison`.
pub fn lock_recover_with<T: ?Sized>(m: &Mutex<T>, on_poison: impl FnOnce()) -> MutexGuard<'_, T> {
    match m.lock() {
        Ok(guard) => guard,
        Err(poison) => {
            on_poison();
            m.clear_poison();
            poison.into_inner()
        }
    }
}

/// Acquire `m`, recovering silently from poisoning.
///
/// Use in production hot paths whose protected state is a cache or
/// dedup set with no broken invariant. Tests and seams that need a
/// breadcrumb when poison was observed should use `lock_recover_warn`
/// instead.
pub fn lock_recover<T: ?Sized>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    lock_recover_with(m, || {})
}

/// Acquire `m`, recovering from poisoning and emitting a
/// `tracing::warn!` tagged with `label` so the recovery event is visible.
///
/// Use in test-support seams (or any callsite where a future flake would
/// otherwise look like a value-mismatch failure rather than the poison
/// that actually caused it).
pub fn lock_recover_warn<'a, T: ?Sized>(m: &'a Mutex<T>, label: &'static str) -> MutexGuard<'a, T> {
    lock_recover_with(m, || {
        tracing::warn!(
            site = label,
            "mutex was poisoned by a previous panicking holder; recovered guard returned"
        );
    })
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

    /// DUP-1 / TASK-2258 (ported from the deleted
    /// `ops_about::lru::lock_recovering` test): the hook runs once per
    /// *observed* poisoning — `clear_poison` means a later acquisition is a
    /// plain healthy lock and must not re-run it.
    #[test]
    fn lock_recover_with_runs_the_hook_once_per_poisoning() {
        let m = Mutex::new(0u32);
        poison(&m);

        let mut hook_ran = false;
        let guard = lock_recover_with(&m, || hook_ran = true);
        assert!(hook_ran, "the hook must run on the observed poisoning");
        assert_eq!(*guard, 0, "recovery hands out the guarded value");
        drop(guard);

        let mut ran_again = false;
        let _guard = lock_recover_with(&m, || ran_again = true);
        assert!(!ran_again, "a healthy lock must not re-run the hook");
    }
}
