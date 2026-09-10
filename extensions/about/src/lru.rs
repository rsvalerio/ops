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
//! DUP-1 / TASK-2150: [`BoundedLruCache`] completes that lift. Three caches
//! in `extensions-rust/about` (typed manifests, project coverage,
//! workspace-root memoization) had each rebuilt the same scaffold on top of
//! [`LruVictimQueue`] — the record/compact/evict loop, the slack constant,
//! the cap-check-then-evict insert preamble — and every past fix to the
//! construct (TASK-1723 compaction, TASK-1572 `Arc` queue keys, TASK-1240
//! heap eviction, TASK-1023 tick-on-hit) had to be applied three times.
//! The scaffold now lives here once; caches keep only their key type,
//! value type and cap.
//!
//! Caches still own their own value type (a typed `LoadedManifest` pairs with
//! freshness metadata; raw text pairs with a per-key `OnceLock`) and their
//! own cap; only the *policy shape* is shared.

use std::borrow::Borrow;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

/// Strictly-increasing per-process access tick. Stamped on every cache hit
/// or insert. The smallest tick recorded against a live entry is the
/// least-recently-used.
///
/// `Relaxed` is sufficient: cross-thread ordering is irrelevant for victim
/// selection. We only need each access to receive a strictly increasing
/// stamp under whatever lock the caller already holds.
#[must_use = "stamp the returned tick on the entry; calling again yields a different tick"]
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
    #[must_use = "store the returned queue; each call allocates a fresh, empty one"]
    pub const fn new() -> Self {
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
    ///
    /// PERF-16 / TASK-1723: pushing alone never shrinks the queue, and the
    /// eviction sweep only runs when the caller's map is at its cap. A cache
    /// that stamps on every access therefore grows this heap without bound
    /// for as long as it sits *below* the cap. Callers must pair `push` with
    /// [`Self::retain_fresh`], triggered off [`Self::len`], to keep the queue
    /// proportional to the live entry count.
    pub fn push(&mut self, tick: u64, key: K) {
        self.heap.push(Reverse((tick, key)));
    }

    /// Number of queued stamps, **including** stale ones. This is the
    /// queue's memory footprint, not the live entry count — the caller's
    /// authoritative map owns that.
    #[must_use = "use the returned count; it includes stale stamps, not just live entries"]
    pub fn len(&self) -> usize {
        self.heap.len()
    }

    /// Whether the queue holds no stamps at all, stale ones included.
    #[must_use = "branch on the verdict; it reflects the heap, not the live entry count"]
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    /// Drop every stamp the caller no longer considers current, leaving one
    /// entry per live key.
    ///
    /// `is_fresh` carries the same contract as in [`Self::pop_lru`]: it
    /// returns `true` when the `(key, tick)` pair still matches the caller's
    /// authoritative record. Runs in `O(n)` over the queue (a `retain` plus
    /// one `O(n)` heapify), so trigger it on a growth threshold rather than
    /// on every push.
    pub fn retain_fresh<F>(&mut self, mut is_fresh: F)
    where
        F: FnMut(&K, u64) -> bool,
    {
        let mut stamps = std::mem::take(&mut self.heap).into_vec();
        stamps.retain(|Reverse((tick, key))| is_fresh(key, *tick));
        self.heap = BinaryHeap::from(stamps);
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

/// Slack added to the victim-queue compaction threshold of every
/// [`BoundedLruCache`].
///
/// PERF-16 / TASK-1723: without it a cache holding a single key would
/// compact on every other access. Sixteen stale stamps is a few hundred
/// bytes and buys amortisation for the small-key-count shape the CLI
/// actually runs.
pub const VICTIM_QUEUE_SLACK: usize = 16;

/// One live cache entry: the caller's value plus the bookkeeping the LRU
/// policy needs.
struct BoundedEntry<V, Q> {
    value: V,
    last_accessed: u64,
    /// The queue-key form of the map key, so a hit-path restamp is a cheap
    /// `Q::clone` (an `Arc` bump when `Q = Arc<K>`) rather than a fresh key
    /// allocation (PERF-3 / TASK-1572).
    queue_key: Q,
}

/// A bounded LRU cache: the map + victim-queue scaffold three
/// `extensions-rust/about` caches used to duplicate (DUP-1 / TASK-2150).
///
/// - `K` is the map key (`PathBuf`, `u64`, …); lookups accept any `QL`
///   where `K: Borrow<QL>`, so a `PathBuf`-keyed cache is probed with a
///   `&Path`.
/// - `Q` is the victim-queue key. It defaults to `K`, and for allocation
///   -heavy keys it should be the shared form (`Arc<PathBuf>`), making the
///   hit-path restamp an atomic bump instead of a `PathBuf` clone
///   (PERF-3 / TASK-1572).
///
/// The type owns the whole policy: the record/compact loop
/// (PERF-16 / TASK-1723), LRU eviction at the cap (PERF-1 / TASK-1240,
/// CONC-2 / TASK-0843), the tick-on-hit refresh (CONC-2 / TASK-1023) and
/// the cap-check-then-evict insert preamble.
pub struct BoundedLruCache<K, V, Q = K>
where
    K: Eq + Hash,
    Q: Ord,
{
    map: HashMap<K, BoundedEntry<V, Q>>,
    victim_queue: LruVictimQueue<Q>,
    cap: usize,
}

impl<K, V, Q> BoundedLruCache<K, V, Q>
where
    K: Eq + Hash + Clone,
    Q: Ord + Clone + Borrow<K> + From<K>,
{
    /// An empty cache that holds at most `cap` live entries, evicting the
    /// least-recently-used one when a *new* key would exceed the cap.
    #[must_use]
    pub fn new(cap: usize) -> Self {
        Self {
            map: HashMap::new(),
            victim_queue: LruVictimQueue::new(),
            cap,
        }
    }

    /// The soft cap this cache was built with.
    #[must_use]
    pub const fn cap(&self) -> usize {
        self.cap
    }

    /// Number of live entries (the map, not the victim queue).
    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether the cache holds no live entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Live keys, in map order (diagnostics; eviction order is the victim
    /// queue's business).
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.map.keys()
    }

    /// Whether `key` has a live entry. Does not stamp an access.
    #[must_use]
    pub fn contains_key<QL>(&self, key: &QL) -> bool
    where
        K: Borrow<QL>,
        QL: Hash + Eq + ?Sized,
    {
        self.map.contains_key(key)
    }

    /// The value for `key`, without stamping an access.
    #[must_use]
    pub fn get<QL>(&self, key: &QL) -> Option<&V>
    where
        K: Borrow<QL>,
        QL: Hash + Eq + ?Sized,
    {
        self.map.get(key).map(|entry| &entry.value)
    }

    /// Mutable access to the value for `key`, without stamping an access —
    /// for callers that adjust bookkeeping fields of the value in place
    /// (e.g. a test splicing a freshness key).
    pub fn get_mut<QL>(&mut self, key: &QL) -> Option<&mut V>
    where
        K: Borrow<QL>,
        QL: Hash + Eq + ?Sized,
    {
        self.map.get_mut(key).map(|entry| &mut entry.value)
    }

    /// Whether the queue holds a stamp for `key` whose tick still matches
    /// the entry's — the freshness closure shared by compaction and
    /// eviction.
    fn stamp_is_fresh(map: &HashMap<K, BoundedEntry<V, Q>>, key: &Q, tick: u64) -> bool {
        map.get(key.borrow())
            .is_some_and(|entry| entry.last_accessed == tick)
    }

    /// Push a fresh `(tick, queue_key)` stamp and compact the queue once it
    /// outgrows `2 * live + VICTIM_QUEUE_SLACK` (PERF-16 / TASK-1723:
    /// stamping on every hit must not grow the queue without bound while
    /// the map sits below its cap — the shape every CLI run has).
    ///
    /// Must run *after* the map reflects the access: compaction validates
    /// each stamp against `map[key].last_accessed`, so a pre-update call
    /// would compact away the stamp it just pushed.
    fn push_stamp(&mut self, queue_key: Q, tick: u64) {
        self.victim_queue.push(tick, queue_key);
        let threshold = self
            .map
            .len()
            .saturating_mul(2)
            .saturating_add(VICTIM_QUEUE_SLACK);
        if self.victim_queue.len() > threshold {
            let map = &self.map;
            self.victim_queue
                .retain_fresh(|key, tick| Self::stamp_is_fresh(map, key, tick));
        }
    }

    /// Look `key` up; when present and `accept` approves the value, stamp
    /// the entry most-recently-used (CONC-2 / TASK-1023: a hit refreshes
    /// the tick so hot entries survive eviction) and return the value.
    ///
    /// A value `accept` rejects is returned as `None` and left *unstamped*
    /// — the caller's staleness check runs before the restamp, so a stale
    /// entry keeps its old tick for the insert path to overwrite.
    pub fn touch_if<QL>(&mut self, key: &QL, accept: impl FnOnce(&V) -> bool) -> Option<&V>
    where
        K: Borrow<QL>,
        QL: Hash + Eq + ?Sized,
    {
        let Self {
            map, victim_queue, ..
        } = self;
        let entry = map.get_mut(key)?;
        if !accept(&entry.value) {
            return None;
        }
        let tick = next_lru_tick();
        entry.last_accessed = tick;
        let queue_key = Q::clone(&entry.queue_key);
        // The mutable borrow of the entry ends here; compaction and the
        // returned shared reference borrow the map immutably.
        victim_queue.push(tick, queue_key);
        let threshold = map
            .len()
            .saturating_mul(2)
            .saturating_add(VICTIM_QUEUE_SLACK);
        if victim_queue.len() > threshold {
            victim_queue.retain_fresh(|key, tick| Self::stamp_is_fresh(map, key, tick));
        }
        map.get(key).map(|entry| &entry.value)
    }

    /// Look `key` up and stamp it most-recently-used; [`Self::touch_if`]
    /// with an always-accepting check.
    pub fn touch<QL>(&mut self, key: &QL) -> Option<&V>
    where
        K: Borrow<QL>,
        QL: Hash + Eq + ?Sized,
    {
        self.touch_if(key, |_| true)
    }

    /// Drop the least-recently-used entry (smallest live tick), if any.
    fn evict_lru(&mut self) {
        let map = &mut self.map;
        if let Some(victim) = self
            .victim_queue
            .pop_lru(|key, tick| Self::stamp_is_fresh(map, key, tick))
        {
            map.remove(victim.borrow());
        }
    }

    /// Insert or replace `key`'s entry, first evicting the LRU entry when a
    /// *new* key would exceed the cap (CONC-2 / TASK-0843: the soft cap,
    /// with the hot working set surviving).
    ///
    /// Replacing an existing key never evicts and resets the entry's tick.
    pub fn insert(&mut self, key: K, value: V) {
        // PERF-1 / TASK-1240: O(log n) eviction via the lazy-invalidation
        // min-heap, replacing the previous O(n) `min_by_key` scan.
        if !self.map.contains_key(&key) && self.map.len() >= self.cap {
            self.evict_lru();
        }
        let queue_key = Q::from(key.clone());
        let tick = next_lru_tick();
        self.map.insert(
            key,
            BoundedEntry {
                value,
                last_accessed: tick,
                queue_key: Q::clone(&queue_key),
            },
        );
        // Stamp after the map update — `push_stamp`'s compaction validates
        // stamps against the live entry.
        self.push_stamp(queue_key, tick);
    }

    /// Drop `key`'s entry (ctx.refresh semantics). The victim queue keeps
    /// its now-stale stamps; compaction and eviction skip them.
    pub fn remove<QL>(&mut self, key: &QL) -> Option<V>
    where
        K: Borrow<QL>,
        QL: Hash + Eq + ?Sized,
    {
        self.map.remove(key).map(|entry| entry.value)
    }

    /// Drop every entry and every stamp (test resets).
    pub fn clear(&mut self) {
        self.map.clear();
        self.victim_queue.clear();
    }

    /// Number of queued stamps including stale ones — the queue's memory
    /// footprint, bounded by the compaction threshold.
    #[must_use]
    pub fn victim_queue_len(&self) -> usize {
        self.victim_queue.len()
    }
}

/// Acquire `lock`, recovering from poisoning instead of propagating it.
///
/// DUP-1 / TASK-2150: the poison-recovering `lock()` helpers were one copy
/// per cache module; the scaffold (and its `on_poison` hook) lives here now.
///
/// A poisoned mutex means some thread panicked while holding it. For the
/// plain-data caches this module serves, the guarded value cannot be torn by
/// the panic, so `PoisonError::into_inner` recovery is safe and propagating
/// the poison would turn one unrelated panic into a crash. `on_poison` runs
/// once per *observed* poisoning, before the guard is handed out — pass a
/// warn (with a monotonic counter, as the typed-manifest cache does) when a
/// poisoned lock guards correctness-relevant state, or a no-op when the
/// worst outcome is a recomputation. The sticky poison flag is cleared
/// afterwards, so later callers see a healthy mutex; a fresh panic
/// re-poisons and re-runs `on_poison`.
pub fn lock_recovering<T: ?Sized>(lock: &Mutex<T>, on_poison: impl FnOnce()) -> MutexGuard<'_, T> {
    match lock.lock() {
        Ok(guard) => guard,
        Err(poison) => {
            on_poison();
            lock.clear_poison();
            poison.into_inner()
        }
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

    /// PERF-16 / TASK-1723: compaction must drop every stale stamp, keep the
    /// queue at one entry per live key, and leave pop order intact.
    #[test]
    fn retain_fresh_drops_stale_stamps_and_preserves_order() {
        let mut q = LruVictimQueue::<&'static str>::new();
        // Three keys, each re-stamped twice: six pushes, three live stamps.
        for (tick, key) in [(1, "a"), (2, "b"), (3, "c"), (4, "b"), (5, "a"), (6, "c")] {
            q.push(tick, key);
        }
        assert_eq!(q.len(), 6);

        // Authoritative state: a@5, b@4, c@6.
        let live = |k: &&'static str, t: u64| matches!((*k, t), ("a", 5) | ("b", 4) | ("c", 6));
        q.retain_fresh(live);
        assert_eq!(q.len(), 3, "one stamp per live key survives compaction");
        assert!(!q.is_empty());

        // Min-heap order still holds after the rebuild: b@4, a@5, c@6.
        assert_eq!(q.pop_lru(live), Some("b"));
        assert_eq!(q.pop_lru(live), Some("a"));
        assert_eq!(q.pop_lru(live), Some("c"));
        assert!(q.is_empty());
    }

    #[test]
    fn retain_fresh_can_empty_the_queue() {
        let mut q = LruVictimQueue::<&'static str>::new();
        q.push(1, "a");
        q.push(2, "b");
        q.retain_fresh(|_, _| false);
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn lru_returns_none_when_all_stale() {
        let mut q = LruVictimQueue::<&'static str>::new();
        q.push(1, "a");
        q.push(2, "b");
        let popped = q.pop_lru(|_, _| false);
        assert!(popped.is_none());
    }

    /// DUP-1 / TASK-2150: the per-cache LRU tests the three
    /// `extensions-rust/about` caches used to carry, against the shared
    /// type directly.
    mod bounded_cache {
        use super::super::{lock_recovering, BoundedLruCache, VICTIM_QUEUE_SLACK};

        #[test]
        fn cap_evicts_the_least_recently_used_key() {
            let mut cache = BoundedLruCache::<&'static str, u8>::new(3);
            cache.insert("a", 1);
            cache.insert("b", 2);
            cache.insert("c", 3);
            assert_eq!(cache.len(), 3);

            // Touch `a`: the LRU victim must now be `b`, never the hot key.
            assert_eq!(cache.touch(&"a"), Some(&1));
            cache.insert("d", 4);

            assert_eq!(cache.len(), 3, "the cap must hold");
            assert!(cache.contains_key(&"a"), "hot key must survive eviction");
            assert!(cache.contains_key(&"d"), "newly inserted key must remain");
            assert!(
                !cache.contains_key(&"b"),
                "the coldest key must be the victim"
            );
        }

        /// PERF-16 / TASK-1723: stamping on every hit must not leak stamps
        /// while the map sits below its cap — the shape every CLI run has.
        #[test]
        fn victim_queue_stays_bounded_below_the_cap() {
            let mut cache = BoundedLruCache::<u32, u8>::new(64);
            cache.insert(1, 1);
            for _ in 0..500 {
                assert!(cache.touch(&1).is_some());
            }
            let bound = cache
                .len()
                .saturating_mul(2)
                .saturating_add(VICTIM_QUEUE_SLACK);
            assert!(
                cache.victim_queue_len() <= bound,
                "victim queue holds {} stamps after 500 hits on one key; expected at most {bound}",
                cache.victim_queue_len()
            );
        }

        /// CONC-2 / TASK-1023 plus the stale-probe rule: `touch_if` rejects
        /// without restamping, and a rejected entry is later overwritten by
        /// `insert`, not evicted as a phantom.
        #[test]
        fn touch_if_rejects_without_restamping() {
            let mut cache = BoundedLruCache::<&'static str, bool>::new(2);
            cache.insert("stale", false);
            assert_eq!(
                cache.touch_if(&"stale", |fresh| *fresh),
                None,
                "a rejected value must not be served or restamped"
            );
            cache.insert("stale", true);
            assert_eq!(
                cache.touch_if(&"stale", |fresh| *fresh),
                Some(&true),
                "the replacement must be servable"
            );
        }

        /// `remove` drops the entry but leaves the queue self-healing: the
        /// stale stamp must never evict a live key later.
        #[test]
        fn remove_leaves_no_phantom_victim() {
            let mut cache = BoundedLruCache::<&'static str, u8>::new(2);
            cache.insert("gone", 1);
            cache.insert("kept", 2);
            assert_eq!(cache.remove(&"gone"), Some(1));
            // Fill the freed slot, then make `kept` the hot key, then force
            // an eviction: the stale `gone` stamp must be skipped and the
            // live `kept` entry must survive.
            cache.insert("x", 3);
            assert!(cache.touch(&"kept").is_some());
            cache.insert("y", 4);
            assert!(cache.contains_key(&"kept"));
            assert!(cache.contains_key(&"y"));
            assert!(
                !cache.contains_key(&"x"),
                "the coldest live key is the victim"
            );
        }

        #[test]
        fn lock_recovering_recovers_and_runs_the_hook_once_per_poisoning() {
            let poisoned = std::sync::Mutex::new(0u8);
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _g = poisoned.lock().unwrap();
                panic!("intentional poison");
            }));
            assert!(poisoned.lock().is_err(), "premise: mutex is poisoned");

            let mut hook_ran = false;
            let guard = lock_recovering(&poisoned, || hook_ran = true);
            assert!(hook_ran, "the hook must run on the observed poisoning");
            assert_eq!(*guard, 0, "recovery hands out the guarded value");

            // The sticky flag was cleared: a later acquisition is a plain
            // healthy lock and must not re-run the hook.
            drop(guard);
            let mut ran_again = false;
            let _guard = lock_recovering(&poisoned, || ran_again = true);
            assert!(!ran_again, "a healthy lock must not re-run the hook");
        }
    }
}
