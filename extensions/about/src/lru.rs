//! Shared LRU primitives for manifest-style caches.
//!
//! DUP-1 / TASK-1145: the typed-manifest cache in
//! `extensions-rust/about/src/query.rs` and the raw-text manifest cache in
//! [`crate::manifest_cache`] both needed identical bookkeeping: a monotonic
//! access-tick stamp and an `O(log n)` LRU victim queue with lazy
//! invalidation. Each was reimplemented from scratch and the doc comments
//! warned that the policies "must be kept in lockstep ... or the two caches
//! will silently drift". Lifting the bookkeeping into one place pins the
//! eviction policy at the code level — a future tweak (different staleness
//! check, batch eviction, etc.) lands in one source location.
//!
//! Caches still own their own value type (a typed `LoadedManifest` pairs with
//! freshness metadata; raw text pairs with a per-key `OnceLock`) and their
//! own cap; only the *policy shape* is shared.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Strictly-increasing per-process access tick. Stamped on every cache hit
/// or insert. The smallest tick recorded against a live entry is the
/// least-recently-used.
///
/// `Relaxed` is sufficient: cross-thread ordering is irrelevant for victim
/// selection. We only need each access to receive a strictly increasing
/// stamp under whatever lock the caller already holds.
#[must_use]
pub fn next_lru_tick() -> u64 {
    static LRU_TICK: AtomicU64 = AtomicU64::new(0);
    LRU_TICK.fetch_add(1, Ordering::Relaxed)
}

/// Min-heap of `(last_accessed_tick, key)` pairs with lazy invalidation.
///
/// Cache hits push a fresh `(tick, key)` entry without removing the older
/// stamp; the eviction loop discards stale heads by comparing the popped
/// tick against the caller-supplied freshness check. This keeps `push`
/// `O(log n)` and amortises eviction at `O(log n)` for the common case.
pub struct LruVictimQueue<K: Ord> {
    heap: BinaryHeap<Reverse<(u64, K)>>,
}

impl<K: Ord> Default for LruVictimQueue<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Ord> LruVictimQueue<K> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
        }
    }

    /// Drop every queued entry. Intended for test-only resets where the
    /// caller is also clearing the authoritative entry map.
    pub fn clear(&mut self) {
        self.heap.clear();
    }

    /// Stamp a fresh `(tick, key)` access. The previous stamp is left in the
    /// heap and discarded as stale on the next eviction sweep.
    pub fn push(&mut self, tick: u64, key: K) {
        self.heap.push(Reverse((tick, key)));
    }

    /// Pop the least-recently-used key. `is_fresh(&key, tick)` returns
    /// `true` when the popped stamp still matches the caller's authoritative
    /// record (i.e. the entry has not been re-stamped on a later access).
    /// Stale heads are skipped silently.
    pub fn pop_lru<F>(&mut self, mut is_fresh: F) -> Option<K>
    where
        F: FnMut(&K, u64) -> bool,
    {
        while let Some(Reverse((tick, key))) = self.heap.pop() {
            if is_fresh(&key, tick) {
                return Some(key);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_strictly_increase() {
        let a = next_lru_tick();
        let b = next_lru_tick();
        let c = next_lru_tick();
        assert!(a < b && b < c);
    }

    #[test]
    fn lru_pop_returns_smallest_fresh_tick() {
        let mut q = LruVictimQueue::<&'static str>::new();
        q.push(10, "a");
        q.push(5, "b");
        q.push(7, "c");
        // All entries fresh — smallest tick wins.
        let first = q.pop_lru(|_, _| true);
        assert_eq!(first, Some("b"));
        let second = q.pop_lru(|_, _| true);
        assert_eq!(second, Some("c"));
    }

    #[test]
    fn lru_skips_stale_heads() {
        let mut q = LruVictimQueue::<&'static str>::new();
        q.push(1, "a");
        q.push(3, "b");
        q.push(5, "a"); // refreshed: the (1,"a") head is now stale.
                        // Caller's authoritative state: a@5, b@3.
        let popped = q.pop_lru(|k, t| match *k {
            "a" => t == 5,
            "b" => t == 3,
            _ => false,
        });
        assert_eq!(popped, Some("b"), "smallest fresh tick wins, stale skipped");
    }

    #[test]
    fn lru_returns_none_when_all_stale() {
        let mut q = LruVictimQueue::<&'static str>::new();
        q.push(1, "a");
        q.push(2, "b");
        let popped = q.pop_lru(|_, _| false);
        assert!(popped.is_none());
    }
}
