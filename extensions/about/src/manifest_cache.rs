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
//! ARCH-1 / TASK-0867: residency is hard-capped at [`CACHE_MAX_ENTRIES`].
//! When the table grows past the cap the entire map is cleared (cheap drop,
//! no LRU bookkeeping) — adequate because the cache value is a parse input,
//! not authoritative state, so a cleared entry just means the next caller
//! re-reads the file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

type CacheMap = HashMap<PathBuf, Option<Arc<str>>>;

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
        // CONC-1 / TASK-1051: hold the lock across the file read so racing
        // readers for the same uncached path observe a single Arc and
        // preserve the Arc::ptr_eq dedup contract that PERF-3 / TASK-0854
        // relies on. This serialises distinct paths through one cache
        // instance, but the cache value is just raw manifest text bounded
        // by manifest_io's size cap, and the warm path returns without IO.
        //
        // ERR-5 / TASK-0878: recover from poisoning by inheriting the
        // inner map. The cache value is the raw file text, not
        // authoritative state, so a panic in a previous holder cannot
        // leave a torn invariant; treating poison as fatal would let one
        // panic permanently brick the cache for every other provider in
        // the process.
        let mut guard = cache.lock().unwrap_or_else(|e| {
            tracing::warn!(
                filename = self.filename,
                "manifest cache mutex was poisoned by a prior panic; recovered"
            );
            e.into_inner()
        });
        if let Some(entry) = guard.get(&path) {
            return entry.clone();
        }
        if guard.len() >= CACHE_MAX_ENTRIES {
            tracing::debug!(
                filename = self.filename,
                cap = CACHE_MAX_ENTRIES,
                "manifest cache reached cap; clearing"
            );
            guard.clear();
        }
        let text =
            crate::manifest_io::read_optional_text(&path, self.filename).map(Arc::<str>::from);
        guard.insert(path, text.clone());
        debug_assert!(
            guard.len() <= CACHE_MAX_ENTRIES,
            "manifest cache exceeded cap of {CACHE_MAX_ENTRIES}"
        );
        text
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
