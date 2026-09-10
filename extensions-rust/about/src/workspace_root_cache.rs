//! Bounded process cache of `cwd → resolved workspace root`.
//!
//! PERF-1 / TASK-2028: CL-3 / TASK-1762 keyed the typed-manifest cache by the
//! *resolved* workspace root so two cwds inside one workspace share one entry.
//! That forced the resolution to happen **before** the cache probe, so
//! `find_workspace_root_strict` — an ancestor walk that `fs::canonicalize`s
//! each candidate's parent under the SEC-25 / TASK-1204 hardening — ran on
//! every `load_workspace_manifest` call, cache hits included. Four providers
//! hit the cache per `ops about` run against the same cwd, so the walk ran four
//! times over for one answer that cannot change between them.
//!
//! Memoizing the walk restores the cheap hit path (`HashMap` probe plus the
//! freshness `stat`) while leaving the typed-manifest cache keyed by the root.
//!
//! # Cache contract
//!
//! - **Key**: the cwd exactly as handed to the resolver
//!   ([`ops_extension::Context::working_directory`]), un-canonicalised. It is
//!   the resolver's own input, so two spellings of one directory memoize twice
//!   and each still answers correctly — the alternative (canonicalising the
//!   key) would reintroduce the syscall this cache exists to remove.
//! - **Value**: the canonical workspace root, shared as `Arc<PathBuf>` so the
//!   hit path hands out an atomic bump rather than a fresh `PathBuf`
//!   allocation — the same `Arc` [`crate::manifest::LoadedManifest`] carries.
//! - **Maximum size**: [`MAX_WORKSPACE_ROOT_CACHE_ENTRIES`], enforced on insert
//!   with LRU eviction via the shared [`ops_about::lru`] primitives, so a
//!   long-running host visiting an unbounded set of directories cannot
//!   accumulate one entry per cwd (PERF-16 / SEC-33).
//! - **Invalidation**: none by elapsed time. A cwd's workspace root changes
//!   only if a `Cargo.toml` is created or removed in its ancestor chain while
//!   the process runs; `ctx.refresh` is the escape hatch and re-resolves,
//!   replacing the entry. Only *successful* resolutions are stored — a
//!   `NotFound` must stay re-checkable, because writing the missing manifest is
//!   precisely what a user does next.
//!
//! # Concurrency
//!
//! One `Mutex<HashMap>`, held for the probe or the insert and never across the
//! ancestor walk, mirroring the CONC-7 contract spelled out in
//! [`crate::manifest_cache`]. The same migration note applies: a daemon host
//! running providers in parallel against many distinct roots must move both
//! caches to a sharded map before this single mutex becomes the bottleneck.

use ops_about::lru::BoundedLruCache;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Soft upper bound on memoized cwds. Matches
/// [`crate::manifest_cache::MAX_TYPED_MANIFEST_CACHE_ENTRIES`]: a single
/// workspace is reached from at most a handful of cwds in practice, and the
/// value is two `Arc<PathBuf>`s, so the cap is about bounding a pathological
/// host rather than trimming a realistic working set.
pub const MAX_WORKSPACE_ROOT_CACHE_ENTRIES: usize = 64;

/// DUP-1 / TASK-2150: the LRU scaffold (victim queue, compaction slack,
/// record/evict loop, cap preamble) lives in
/// [`ops_about::lru::BoundedLruCache`]; this struct names the key, value and
/// cap for the cwd→root memoization. Queue keys are `Arc<PathBuf>` shared
/// with the entry, so a hit-path restamp is an atomic bump (PERF-3 /
/// TASK-1572).
struct WorkspaceRootCache {
    cache: BoundedLruCache<PathBuf, Arc<PathBuf>, Arc<PathBuf>>,
}

impl WorkspaceRootCache {
    fn new() -> Self {
        Self {
            cache: BoundedLruCache::new(MAX_WORKSPACE_ROOT_CACHE_ENTRIES),
        }
    }

    fn probe(&mut self, cwd: &Path) -> Option<Arc<PathBuf>> {
        self.cache.touch(cwd).cloned()
    }

    fn insert(&mut self, cwd: &Path, root: &Arc<PathBuf>) {
        self.cache.insert(cwd.to_path_buf(), Arc::clone(root));
    }
}

fn workspace_root_cache() -> &'static Mutex<WorkspaceRootCache> {
    static CACHE: OnceLock<Mutex<WorkspaceRootCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(WorkspaceRootCache::new()))
}

/// Acquire the cache lock, recovering from a `PoisonError` instead of
/// panicking: the value is plain data (two `Arc<PathBuf>`s and a counter), so a
/// panic in another provider cannot leave it torn, and propagating the poison
/// would turn one unrelated panic into an `ops about` crash. The sibling
/// typed-manifest cache warns on recovery because a poisoned lock there
/// degrades a correctness-relevant freshness check; here the worst outcome is
/// an extra ancestor walk, so recovery is silent. DUP-1 / TASK-2150: the
/// shared helper carries the recovery scaffold.
fn lock() -> std::sync::MutexGuard<'static, WorkspaceRootCache> {
    ops_about::lru::lock_recovering(workspace_root_cache(), || {})
}

/// The memoized workspace root for `cwd`, if one has been resolved.
pub fn probe(cwd: &Path) -> Option<Arc<PathBuf>> {
    lock().probe(cwd)
}

/// Memoize `root` as the resolved workspace root for `cwd`, evicting the
/// least-recently-used entry when the cap is reached.
pub fn insert(cwd: &Path, root: &Arc<PathBuf>) {
    lock().insert(cwd, root);
}

/// Drop the memoized root for `cwd`, if any. Test-facing: production code
/// replaces an entry through [`insert`] on the `ctx.refresh` path rather than
/// removing it, so eviction and re-resolution stay one step.
#[cfg(test)]
pub fn evict(cwd: &Path) {
    lock().cache.remove(cwd);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_about::lru::VICTIM_QUEUE_SLACK;

    fn clear_cache() {
        lock().cache.clear();
    }

    fn cache_len() -> usize {
        lock().cache.len()
    }

    fn victim_queue_len() -> usize {
        lock().cache.victim_queue_len()
    }

    fn root_of(name: &str) -> Arc<PathBuf> {
        Arc::new(PathBuf::from(format!("/ws/{name}")))
    }

    // These tests `clear_cache()` and assert exact entry counts against the
    // process-global cache, so nothing else may be populating it. The
    // populating callers are `load_workspace_manifest`'s tests, which run
    // under the `typed_manifest_cache` key — holding both keys is what
    // actually excludes them. Naming only `workspace_root_cache` left the two
    // groups free to run concurrently, and a manifest test's insert would then
    // break a `cache_len()` assertion here.

    /// The point of the cache: a second lookup for the same cwd answers from
    /// memory, so the caller never repeats the canonicalizing ancestor walk.
    #[serial_test::serial(typed_manifest_cache, workspace_root_cache)]
    #[test]
    fn probe_returns_the_memoized_root() {
        clear_cache();
        let cwd = Path::new("/ws/alpha/crates/foo");
        assert!(probe(cwd).is_none(), "cold cache must miss");

        let root = root_of("alpha");
        insert(cwd, &root);

        let hit = probe(cwd).expect("warm cache must hit");
        assert!(
            Arc::ptr_eq(&hit, &root),
            "the hit must share the memoized allocation, not clone the path"
        );
        assert!(
            probe(Path::new("/ws/beta/crates/foo")).is_none(),
            "an unrelated cwd must not hit"
        );
        clear_cache();
    }

    /// Re-inserting a known cwd replaces the memoized root rather than adding
    /// a second entry — this is the `ctx.refresh` path, where the ancestor
    /// walk is redone and may legitimately land on a different root.
    #[serial_test::serial(typed_manifest_cache, workspace_root_cache)]
    #[test]
    fn insert_replaces_an_existing_entry() {
        clear_cache();
        let cwd = Path::new("/ws/alpha/crates/foo");
        insert(cwd, &root_of("alpha"));
        insert(cwd, &root_of("alpha/crates/foo"));

        assert_eq!(cache_len(), 1, "re-insert must not add an entry");
        assert_eq!(
            probe(cwd).as_deref(),
            Some(&PathBuf::from("/ws/alpha/crates/foo")),
            "re-insert must replace the memoized root"
        );
        clear_cache();
    }

    /// PERF-16 / SEC-33: the cap holds and the least-recently-used cwd is the
    /// one that goes, so a host cycling through directories cannot grow this
    /// map without bound and cannot evict the entry it keeps touching.
    #[serial_test::serial(typed_manifest_cache, workspace_root_cache)]
    #[test]
    fn cap_evicts_the_least_recently_used_cwd() {
        clear_cache();
        let hot = PathBuf::from("/ws/hot");
        insert(&hot, &root_of("hot"));

        for i in 0..MAX_WORKSPACE_ROOT_CACHE_ENTRIES {
            // Keep `hot` the most-recently-used entry throughout.
            assert!(
                probe(&hot).is_some(),
                "hot entry must survive iteration {i}"
            );
            insert(&PathBuf::from(format!("/ws/cold-{i}")), &root_of("cold"));
        }

        assert_eq!(
            cache_len(),
            MAX_WORKSPACE_ROOT_CACHE_ENTRIES,
            "the cap must hold"
        );
        assert!(
            probe(&hot).is_some(),
            "LRU eviction must spare the repeatedly touched cwd"
        );
        assert!(
            probe(Path::new("/ws/cold-0")).is_none(),
            "the least-recently-used cwd must be the evicted one"
        );
        clear_cache();
    }

    /// PERF-16 / TASK-1723: stamping on every hit must not leak stamps while
    /// the map sits below the cap — the shape every CLI run has.
    #[serial_test::serial(typed_manifest_cache, workspace_root_cache)]
    #[test]
    fn victim_queue_stays_bounded_below_the_cap() {
        clear_cache();
        let cwd = PathBuf::from("/ws/alpha/crates/foo");
        insert(&cwd, &root_of("alpha"));
        for _ in 0..500 {
            assert!(probe(&cwd).is_some());
        }
        let bound = cache_len()
            .saturating_mul(2)
            .saturating_add(VICTIM_QUEUE_SLACK);
        assert!(
            victim_queue_len() <= bound,
            "victim queue must compact instead of growing per hit: {}",
            victim_queue_len()
        );
        clear_cache();
    }

    #[serial_test::serial(typed_manifest_cache, workspace_root_cache)]
    #[test]
    fn evict_drops_only_the_named_cwd() {
        clear_cache();
        let kept = PathBuf::from("/ws/kept");
        let dropped = PathBuf::from("/ws/dropped");
        insert(&kept, &root_of("kept"));
        insert(&dropped, &root_of("dropped"));

        evict(&dropped);

        assert!(probe(&kept).is_some(), "unrelated entry must survive");
        assert!(probe(&dropped).is_none(), "named entry must be gone");
        clear_cache();
    }
}
