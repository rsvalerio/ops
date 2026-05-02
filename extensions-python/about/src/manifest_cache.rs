//! Process-local cache for the parsed `pyproject.toml` value.
//!
//! DUP-3 / TASK-0816: the identity and units providers both read+parse the
//! same `pyproject.toml` per About invocation. `toml::from_str` is allocation-
//! heavy on real-world manifests (2--10 KB is typical) and shows up twice in
//! flamegraphs even though the file content is identical between calls.
//!
//! The cache is keyed by the joined manifest path (the providers call us with
//! the project root, which is constant within a single About run). Cache
//! entries are `Arc<toml::Value>` so both providers can deserialize their
//! private `Raw*` shapes from the shared parse without re-parsing the file.
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

static CACHE: OnceLock<Mutex<HashMap<PathBuf, Option<Arc<toml::Value>>>>> = OnceLock::new();

/// Read and parse `<root>/pyproject.toml` once per process, returning the
/// shared `toml::Value`. Subsequent calls with the same `root` reuse the
/// cached parse. Returns `None` when the file is missing or cannot be parsed
/// (the parse error is logged at the first call site, matching the pre-cache
/// behaviour).
pub(crate) fn pyproject_value(root: &Path) -> Option<Arc<toml::Value>> {
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let path = root.join("pyproject.toml");
    if let Some(entry) = cache.lock().expect("pyproject cache poisoned").get(&path) {
        return entry.clone();
    }
    let parsed = match ops_about::manifest_io::read_optional_text(&path, "pyproject.toml") {
        Some(content) => match toml::from_str::<toml::Value>(&content) {
            Ok(v) => Some(Arc::new(v)),
            Err(e) => {
                tracing::warn!(
                    path = ?path.display(),
                    error = %e,
                    recovery = "default-identity",
                    "failed to parse pyproject.toml"
                );
                None
            }
        },
        None => None,
    };
    let mut guard = cache.lock().expect("pyproject cache poisoned");
    if guard.len() >= CACHE_MAX_ENTRIES {
        // Bounded high-water mark: drop the whole table on overflow rather
        // than maintaining LRU metadata for a parse-result cache. Next
        // caller re-parses; correctness is unaffected.
        tracing::debug!(
            cap = CACHE_MAX_ENTRIES,
            "pyproject cache reached cap; clearing"
        );
        guard.clear();
    }
    guard.insert(path, parsed.clone());
    debug_assert!(
        guard.len() <= CACHE_MAX_ENTRIES,
        "pyproject cache exceeded cap of {CACHE_MAX_ENTRIES}"
    );
    parsed
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
        let a = pyproject_value(dir.path()).unwrap();
        let b = pyproject_value(dir.path()).unwrap();
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn missing_file_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(pyproject_value(dir.path()).is_none());
    }
}
