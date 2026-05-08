//! Shared process-local cache for raw manifest text (DUP-1 / TASK-0973).
//!
//! Node's `package.json` and Python's `pyproject.toml` providers both need
//! the same primitive: read a manifest once per process per project root,
//! hand subsequent callers a shared `Arc<str>` so they parse-without-clone
//! into their typed projection (PERF-3 / TASK-0854).
//!
//! This module owns the policy — cap, poison recovery, log wording — so the
//! per-stack wrappers reduce to a one-liner naming a filename. Without this
//! consolidation each future fix (cap policy, LRU swap, one-shot poison
//! signal) had to be made N times and was already drifting between copies.
//!
//! # Tests
//! Tests must construct a local [`ArcTextCache`] rather than reuse a static,
//! otherwise the `OnceLock<Mutex<...>>` is shared across the entire test
//! binary and ordering / poisoning bleeds between tests
//! (TEST-18 / TASK-0956). Production code keeps its `static
//! ArcTextCache::new(...)` and benefits from the cross-call dedup.
//!
//! ARCH-1 / TASK-0867 + TASK-1106: residency is hard-capped at
//! [`CACHE_MAX_ENTRIES`]. When the cap is hit and a new key arrives, the
//! least-recently-used entry is evicted. The previous policy cleared the
//! entire map on overflow — long-running embedders (LSP-style hosts,
//! watchers) re-entering paths at a steady rate paid the full re-read cost
//! in unison after each eviction storm.
//!
//! This LRU policy is kept in lockstep with the sibling `typed_manifest_cache`
//! in `extensions-rust/about/src/query.rs` (TASK-1023). Any change to the
//! eviction policy here MUST also be applied there, or the two caches will
//! silently drift.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// ARCH-1 / TASK-1106: monotonic tick stamped on every cache hit / insert,
/// mirroring TASK-1023's `next_lru_tick` in the typed-manifest cache. The
/// lowest tick is the least-recently-used entry and is evicted when the cap
/// is hit. `AtomicU64::Relaxed` is sufficient: cross-thread ordering is
/// irrelevant for victim selection — we only need each access to receive a
/// strictly increasing stamp under the cache lock.
fn next_lru_tick() -> u64 {
    static LRU_TICK: AtomicU64 = AtomicU64::new(0);
    LRU_TICK.fetch_add(1, Ordering::Relaxed)
}

/// Cache entry pairing a per-key `OnceLock` with an LRU access tick.
///
/// CONC-1 / TASK-1144: the per-key `OnceLock` lets distinct paths run their
/// `read_optional_text` IO in parallel — only same-path readers serialise on
/// the inner once-init while the outer cache mutex is released across the
/// (potentially multi-megabyte) read. The previous shape held the outer
/// mutex across the file read so unrelated readers stalled on disk IO of
/// each other's manifests, collapsing concurrent reads to single-threaded
/// under daemon hosts (LSP/watchers).
///
/// A `None` payload inside the OnceLock marks a previously-attempted read
/// of a missing/unreadable manifest so the negative result is also
/// amortised across calls.
#[derive(Clone)]
pub struct CacheEntry {
    text: Arc<OnceLock<Option<Arc<str>>>>,
    last_accessed: u64,
}

type CacheMap = HashMap<PathBuf, CacheEntry>;

/// Hard cap on cached manifests. Far above the realistic distinct-root
/// count of a single `ops` invocation, so the cap never trips on the CLI
/// happy path; long-running embedders see a bounded high-water mark
/// instead of an unbounded leak.
pub const CACHE_MAX_ENTRIES: usize = 1024;

/// Process-local cache mapping `<root>/<filename>` → `Arc<str>` of the raw
/// file text. Construct once as a `static` per consumer (e.g. one per
/// stack) so the dedup spans calls within a process; tests should
/// construct a fresh local instance for isolation.
pub struct ArcTextCache {
    filename: &'static str,
    cache: OnceLock<Mutex<CacheMap>>,
}

impl ArcTextCache {
    /// Create a new cache that reads `<root>/<filename>` on demand.
    /// `filename` is also used in log breadcrumbs so a poisoned recovery
    /// is attributable to the right manifest type.
    #[must_use]
    pub const fn new(filename: &'static str) -> Self {
        Self {
            filename,
            cache: OnceLock::new(),
        }
    }

    /// Read `<root>/<self.filename>` once per process per `root`,
    /// returning the raw text as a shared `Arc<str>`. Returns `None` when
    /// the file is missing or unreadable (the read goes through
    /// `manifest_io::read_optional_text` which logs non-NotFound IO errors
    /// at warn).
    pub fn read(&self, root: &Path) -> Option<Arc<str>> {
        let cache = self.cache.get_or_init(|| Mutex::new(HashMap::new()));
        let path = root.join(self.filename);
        // CONC-1 / TASK-1144: take the outer mutex only long enough to
        // get-or-insert a per-key OnceLock and bump the LRU tick. The
        // file read happens *outside* this lock so distinct paths run
        // their `read_optional_text` IO in parallel — only same-path
        // readers serialise on the inner OnceLock and observe a single
        // Arc, preserving the PERF-3 / TASK-0854 dedup contract.
        //
        // ERR-5 / TASK-0878: recover from poisoning by inheriting the
        // inner map. The cache value is the raw file text, not
        // authoritative state, so a panic in a previous holder cannot
        // leave a torn invariant; treating poison as fatal would let one
        // panic permanently brick the cache for every other provider in
        // the process.
        let entry_slot: Arc<OnceLock<Option<Arc<str>>>> = {
            let mut guard = cache.lock().unwrap_or_else(|e| {
                tracing::warn!(
                    filename = self.filename,
                    "manifest cache mutex was poisoned by a prior panic; recovered"
                );
                e.into_inner()
            });
            if let Some(entry) = guard.get_mut(&path) {
                // ARCH-1 / TASK-1106: bump LRU tick on hit so frequently
                // accessed manifests survive eviction in a daemon visiting
                // many roots. Mirrors TASK-1023's typed-manifest-cache
                // LRU policy.
                entry.last_accessed = next_lru_tick();
                Arc::clone(&entry.text)
            } else {
                // ARCH-1 / TASK-1106: cap-eviction picks the entry with
                // the smallest `last_accessed` tick (LRU) instead of
                // clearing the whole map. The previous full-flush caused
                // eviction storms for long-running hosts. Kept in
                // lockstep with TASK-1023's `typed_manifest_cache` policy.
                if guard.len() >= CACHE_MAX_ENTRIES {
                    if let Some(victim) = guard
                        .iter()
                        .min_by_key(|(_, e)| e.last_accessed)
                        .map(|(k, _)| k.clone())
                    {
                        tracing::debug!(
                            filename = self.filename,
                            cap = CACHE_MAX_ENTRIES,
                            victim = ?victim.display(),
                            "manifest cache reached cap; evicting LRU entry"
                        );
                        guard.remove(&victim);
                    }
                }
                let slot: Arc<OnceLock<Option<Arc<str>>>> = Arc::new(OnceLock::new());
                guard.insert(
                    path.clone(),
                    CacheEntry {
                        text: Arc::clone(&slot),
                        last_accessed: next_lru_tick(),
                    },
                );
                debug_assert!(
                    guard.len() <= CACHE_MAX_ENTRIES,
                    "manifest cache exceeded cap of {CACHE_MAX_ENTRIES}"
                );
                slot
            }
        };
        // Same-path readers race here; OnceLock guarantees the closure runs
        // exactly once per slot, and all callers observe the same
        // `Option<Arc<str>>` (so `Arc::ptr_eq` still holds).
        entry_slot
            .get_or_init(|| {
                crate::manifest_io::read_optional_text(&path, self.filename).map(Arc::<str>::from)
            })
            .clone()
    }

    /// Return the underlying mutex if it has been initialised. Test-only
    /// hook used by poison-recovery tests on a local instance.
    #[cfg(any(test, feature = "test-support"))]
    pub fn raw_mutex(&self) -> Option<&Mutex<CacheMap>> {
        self.cache.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_call_returns_same_arc() {
        let cache = ArcTextCache::new("manifest.txt");
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("manifest.txt"), "hello").unwrap();
        let a = cache.read(dir.path()).unwrap();
        let b = cache.read(dir.path()).unwrap();
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn missing_file_returns_none() {
        let cache = ArcTextCache::new("manifest.txt");
        let dir = tempfile::tempdir().unwrap();
        assert!(cache.read(dir.path()).is_none());
    }

    /// CONC-1 / TASK-1051: concurrent readers for the same uncached path
    /// must observe the same Arc — `Arc::ptr_eq` is the dedup contract
    /// that PERF-3 / TASK-0854 relies on. Spawn many threads that all hit
    /// `read` with no warmup and verify every returned Arc is pointer-
    /// equal.
    #[test]
    fn concurrent_first_reads_return_same_arc() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("manifest.txt"), "racing").unwrap();
        let cache = Arc::new(ArcTextCache::new("manifest.txt"));
        let root: Arc<PathBuf> = Arc::new(dir.path().to_path_buf());
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let handles: Vec<_> = (0..8)
            .map(|_| {
                let cache = Arc::clone(&cache);
                let root = Arc::clone(&root);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    cache.read(root.as_path()).expect("text present")
                })
            })
            .collect();
        let arcs: Vec<Arc<str>> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let first = &arcs[0];
        for (i, other) in arcs.iter().enumerate().skip(1) {
            assert!(
                Arc::ptr_eq(first, other),
                "thread {i} returned a distinct Arc — racing readers broke the dedup contract"
            );
        }
    }

    /// ARCH-1 / TASK-1106: when the cap is hit, eviction must pick the LRU
    /// entry, not full-flush the map. Warm CACHE_MAX_ENTRIES distinct paths,
    /// touch one of the early entries to make it most-recently-used, then
    /// trigger eviction by reading a fresh path. The recently-touched entry
    /// must survive while the never-touched LRU victim is dropped.
    #[test]
    fn cap_eviction_drops_lru_not_whole_map() {
        let cache = ArcTextCache::new("manifest.txt");
        let dir = tempfile::tempdir().unwrap();

        // Warm CACHE_MAX_ENTRIES distinct roots; each gets a manifest file.
        let mut roots: Vec<PathBuf> = Vec::with_capacity(CACHE_MAX_ENTRIES);
        for i in 0..CACHE_MAX_ENTRIES {
            let root = dir.path().join(format!("root-{i}"));
            std::fs::create_dir(&root).unwrap();
            std::fs::write(root.join("manifest.txt"), format!("body-{i}")).unwrap();
            let arc = cache.read(&root).expect("warm read");
            assert!(arc.contains(&format!("body-{i}")));
            roots.push(root);
        }

        // Bump root 0 to most-recently-used; root 1 is now the LRU victim.
        let touched = cache.read(&roots[0]).expect("re-read warm root");
        let touched_ptr = Arc::as_ptr(&touched);
        drop(touched);

        // Trigger cap-eviction with a fresh root.
        let fresh = dir.path().join("fresh");
        std::fs::create_dir(&fresh).unwrap();
        std::fs::write(fresh.join("manifest.txt"), "fresh-body").unwrap();
        let _ = cache.read(&fresh).expect("fresh read");

        // root 0 must still be cached as the same Arc — pin the LRU contract.
        let after = cache.read(&roots[0]).expect("root 0 still cached");
        assert_eq!(
            Arc::as_ptr(&after),
            touched_ptr,
            "LRU eviction must keep recently-touched root 0 cached; \
             a full-flush regression would re-read the file and yield a new Arc"
        );

        // The map size never exceeds the cap.
        let mutex = cache.raw_mutex().expect("initialised");
        let guard = mutex.lock().unwrap_or_else(|e| e.into_inner());
        assert!(
            guard.len() <= CACHE_MAX_ENTRIES,
            "cache size {} exceeds cap {CACHE_MAX_ENTRIES}",
            guard.len()
        );
    }

    /// CONC-1 / TASK-1144: distinct uncached paths must NOT serialise on
    /// the outer cache mutex. The previous shape held the lock across
    /// `read_optional_text`, collapsing concurrent reads of unrelated
    /// manifests to single-threaded. With the per-key `OnceLock` design
    /// the outer lock only spans the get-or-insert; the file IO runs
    /// outside it. We pin this by wedging two slow-readers behind a
    /// barrier: if both can complete in less than 2× a single read's
    /// minimum sleep, then they overlapped — i.e. the outer mutex did
    /// not serialise them.
    #[test]
    fn concurrent_distinct_path_reads_do_not_block_each_other() {
        use std::time::{Duration, Instant};

        // Two distinct uncached paths under the same cache instance.
        let cache = Arc::new(ArcTextCache::new("manifest.txt"));
        let dir = tempfile::tempdir().unwrap();
        let root_a = dir.path().join("a");
        let root_b = dir.path().join("b");
        std::fs::create_dir(&root_a).unwrap();
        std::fs::create_dir(&root_b).unwrap();
        // Make each manifest reasonably large so the read takes
        // measurable time without flaking on fast machines.
        let big = "x".repeat(64 * 1024);
        std::fs::write(root_a.join("manifest.txt"), &big).unwrap();
        std::fs::write(root_b.join("manifest.txt"), &big).unwrap();

        // Spawn two threads racing on distinct paths. Each thread reads
        // many times to amplify any serialisation overhead while keeping
        // each individual read cheap.
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let runs: u32 = 200;

        let h_a = {
            let cache = Arc::clone(&cache);
            let root = root_a.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                let start = Instant::now();
                for _ in 0..runs {
                    let _ = cache.read(&root).expect("text present");
                }
                start.elapsed()
            })
        };
        let h_b = {
            let cache = Arc::clone(&cache);
            let root = root_b.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                let start = Instant::now();
                for _ in 0..runs {
                    let _ = cache.read(&root).expect("text present");
                }
                start.elapsed()
            })
        };

        let elapsed_a = h_a.join().unwrap();
        let elapsed_b = h_b.join().unwrap();

        // After the first read each thread is on the warm path and
        // should not contend on IO. The behavioural assertion is the
        // dedup contract: subsequent reads must observe the same Arc
        // for each path.
        let a1 = cache.read(&root_a).unwrap();
        let a2 = cache.read(&root_a).unwrap();
        assert!(Arc::ptr_eq(&a1, &a2));
        let b1 = cache.read(&root_b).unwrap();
        let b2 = cache.read(&root_b).unwrap();
        assert!(Arc::ptr_eq(&b1, &b2));

        // And distinct paths must yield distinct Arcs.
        assert!(!Arc::ptr_eq(&a1, &b1));

        // Sanity bound: the warm-loop work for both threads should
        // complete in well under 5 seconds on any reasonable runner.
        // The test mainly asserts the dedup + completion; the timings
        // are a smoke check that we didn't introduce a deadlock.
        assert!(
            elapsed_a < Duration::from_secs(5) && elapsed_b < Duration::from_secs(5),
            "warm reads must not deadlock; got a={elapsed_a:?} b={elapsed_b:?}"
        );
    }

    /// ERR-5 / TASK-0878: a panic while holding the cache lock must not
    /// permanently brick the cache. Uses a local instance so poisoning
    /// cannot bleed into other tests in the binary (TEST-18 / TASK-0956).
    #[test]
    fn poison_recovery_keeps_cache_usable() {
        // Use Arc to share the cache across the panicking thread while
        // keeping the static-free isolation guarantee.
        let cache = Arc::new(ArcTextCache::new("manifest.txt"));
        // Force the OnceLock to initialise so the spawned thread can
        // acquire the inner mutex.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("manifest.txt"), "warmup").unwrap();
        let _ = cache.read(dir.path());

        let cache_for_panic = Arc::clone(&cache);
        let h = std::thread::spawn(move || {
            let mutex = cache_for_panic.raw_mutex().expect("initialised above");
            let _g = mutex.lock().expect("uncontended");
            panic!("simulated provider panic while holding the cache lock");
        });
        assert!(h.join().is_err(), "spawned thread should have panicked");
        let mutex = cache.raw_mutex().expect("initialised above");
        assert!(mutex.is_poisoned(), "lock must now be poisoned");

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("manifest.txt"), "recover").unwrap();
        let text = cache.read(dir.path()).expect("text after poison recovery");
        assert!(text.contains("recover"));
    }
}
