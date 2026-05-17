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
pub(crate) fn lock_recover<'a, T>(m: &'a Mutex<T>) -> MutexGuard<'a, T> {
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
pub(crate) fn lock_recover_warn<'a, T>(m: &'a Mutex<T>, label: &'static str) -> MutexGuard<'a, T> {
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
