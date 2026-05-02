//! Process-local cache for the parsed `pyproject.toml` value.
//!
//! DUP-3 / TASK-0816: the identity and units providers both read+parse the
//! same `pyproject.toml` per About invocation. `toml::from_str` is allocation-
//! heavy on real-world manifests (2--10 KB is typical) and shows up twice in
//! flamegraphs even though the file content is identical between calls.
//!
//! The cache is keyed by the joined manifest path (the providers call us with
//! the project root, which is constant within a single About run). Cache
//! entries are `Arc<str>` (the raw file text, validated as UTF-8) so each
//! consumer projects directly into its private `Raw*` shape via
//! `toml::from_str`.
//!
//! PERF-3 / TASK-0854: the cache previously stored `Arc<toml::Value>` so the
//! parse cost was paid once, but each consumer then `(*value).clone()`d the
//! 2-10 KB Value tree before `try_into`-ing. That clone partially undid the
//! deduplication. Storing the raw text and letting each consumer parse
//! directly into its target shape is cheaper than parse-once-then-deep-clone:
//! `toml::from_str::<RawX>` builds only the needed projection, which is far
//! smaller than a generic `toml::Value` tree.
//!
//! Bounded leak: in a one-shot CLI process the cache holds at most one entry
//! per project root probed; under `cargo test` parallelism every test creates
//! a fresh tempdir so paths never collide and stale data cannot mask a fix.
//!
//! ARCH-1 / TASK-0867: residency is hard-capped at [`CACHE_MAX_ENTRIES`].
//! When the table grows past the cap the entire map is cleared (cheap drop,
//! no LRU bookkeeping) — adequate because:
//!  - The cache value is a parse result, not authoritative state, so a
//!    cleared entry just means the next caller re-parses the manifest.
//!  - The cap is set far above any realistic single-CLI-run distinct-root
//!    count, so a one-shot process never trips it.
//!  - A long-running embedder that keeps probing distinct roots gets a
//!    bounded high-water mark instead of an unbounded leak.
//!
//! Embedders that drive `pyproject_value` from a long-running daemon, LSP
//! server, or `cargo test` reuse loop must treat this as a per-process
//! best-effort cache, not a durable store. If durability matters, scope the
//! cache via an explicit constructor parameter rather than the static.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Hard cap on cached parsed manifests. ARCH-1 / TASK-0867: bounds residency
/// in long-running hosts; far above the realistic distinct-root count of a
/// single `ops` invocation, so the cap never trips on the CLI happy path.
const CACHE_MAX_ENTRIES: usize = 1024;

static CACHE: OnceLock<Mutex<HashMap<PathBuf, Option<Arc<str>>>>> = OnceLock::new();

/// Read `<root>/pyproject.toml` once per process, returning the raw text
/// as a shared `Arc<str>`. Subsequent calls with the same `root` reuse the
/// cached file content. Returns `None` when the file is missing.
///
/// PERF-3 / TASK-0854: callers parse directly via
/// `toml::from_str::<RawX>(&text)` which projects straight into the
/// caller's typed shape — no `toml::Value` intermediate, no per-call deep
/// clone of a generic value tree.
pub(crate) fn pyproject_text(root: &Path) -> Option<Arc<str>> {
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let path = root.join("pyproject.toml");
    // ERR-5 / TASK-0878: recover from poisoning by inheriting the inner
    // map. The cache value is the raw file text, not authoritative state,
    // so a panic in a previous holder cannot leave a torn invariant;
    // treating poison as fatal would let one panic permanently brick the
    // cache for every other provider in the process.
    if let Some(entry) = cache
        .lock()
        .unwrap_or_else(|e| {
            tracing::warn!("pyproject cache mutex was poisoned by a prior panic; recovered");
            e.into_inner()
        })
        .get(&path)
    {
        return entry.clone();
    }
    let text =
        ops_about::manifest_io::read_optional_text(&path, "pyproject.toml").map(Arc::<str>::from);
    let mut guard = cache.lock().unwrap_or_else(|e| {
        tracing::warn!("pyproject cache mutex was poisoned by a prior panic; recovered");
        e.into_inner()
    });
    if guard.len() >= CACHE_MAX_ENTRIES {
        tracing::debug!(
            cap = CACHE_MAX_ENTRIES,
            "pyproject cache reached cap; clearing"
        );
        guard.clear();
    }
    guard.insert(path, text.clone());
    debug_assert!(
        guard.len() <= CACHE_MAX_ENTRIES,
        "pyproject cache exceeded cap of {CACHE_MAX_ENTRIES}"
    );
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_call_returns_same_arc() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"demo\"\n",
        )
        .unwrap();
        let a = pyproject_text(dir.path()).unwrap();
        let b = pyproject_text(dir.path()).unwrap();
        assert!(Arc::ptr_eq(&a, &b));
    }

    /// PERF-3 / TASK-0854: the second consumer of the cached text must
    /// share the Arc allocation — proves the cache deduplicates the IO
    /// without forcing a re-read or text clone per consumer.
    #[test]
    fn arc_is_shared_across_two_consumer_parses() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        // Two distinct consumers each ask for the cached text and parse
        // their own typed projection. Both must observe the same Arc.
        let a = pyproject_text(dir.path()).unwrap();
        let _: toml::Value = toml::from_str(&a).expect("identity-shape parse from shared text");
        let b = pyproject_text(dir.path()).unwrap();
        let _: toml::Value = toml::from_str(&b).expect("workspace-shape parse from shared text");
        assert!(
            Arc::ptr_eq(&a, &b),
            "both consumers must share the cached Arc<str>"
        );
        // strong_count = 1 in cache + 2 captured in test = 3; guard
        // against future regressions where the cache stops sharing.
        assert!(Arc::strong_count(&a) >= 3);
    }

    #[test]
    fn missing_file_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(pyproject_text(dir.path()).is_none());
    }

    /// ERR-5 / TASK-0878: a panic while holding the cache lock must not
    /// permanently brick the cache for every other provider in the
    /// process. Catching the panic across a thread boundary simulates the
    /// production scenario without aborting the test runner.
    #[test]
    fn poison_recovery_keeps_cache_usable() {
        let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));

        let h = std::thread::spawn(|| {
            let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
            let _g = cache.lock().expect("uncontended");
            panic!("simulated provider panic while holding the cache lock");
        });
        assert!(h.join().is_err(), "spawned thread should have panicked");
        assert!(cache.is_poisoned(), "lock must now be poisoned");

        // Subsequent caller must still succeed despite the poisoned mutex.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"recover\"\n",
        )
        .unwrap();
        let text = pyproject_text(dir.path()).expect("text after poison recovery");
        assert!(text.contains("name = \"recover\""));
    }
}
