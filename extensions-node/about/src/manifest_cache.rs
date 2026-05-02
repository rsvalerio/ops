//! Process-local cache for the raw `package.json` text.
//!
//! DUP-3 / TASK-0931: the identity provider (`parse_package_json`) and the
//! units provider (`workspace_member_globs`) both read+parse the same
//! `package.json` per About invocation. Mirrors the Python `manifest_cache`
//! pattern from TASK-0816 — Node was the structural-consistency gap.
//!
//! Cache entries are `Arc<str>` (the raw file text, validated as UTF-8) so
//! each consumer projects directly into its private `Raw*` shape via
//! `serde_json::from_str` — no `serde_json::Value` intermediate, no
//! per-call deep clone of a generic value tree (PERF-3 / TASK-0854).
//!
//! ARCH-1 / TASK-0867: residency is hard-capped at [`CACHE_MAX_ENTRIES`].
//! When the table grows past the cap the entire map is cleared (cheap drop,
//! no LRU bookkeeping) — adequate because the cache value is a parse input,
//! not authoritative state, so a cleared entry just means the next caller
//! re-reads the file.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

const CACHE_MAX_ENTRIES: usize = 1024;

static CACHE: OnceLock<Mutex<HashMap<PathBuf, Option<Arc<str>>>>> = OnceLock::new();

/// Read `<root>/package.json` once per process, returning the raw text as a
/// shared `Arc<str>`. Subsequent calls with the same `root` reuse the
/// cached file content. Returns `None` when the file is missing or
/// unreadable (the read goes through `manifest_io::read_optional_text`
/// which logs non-NotFound IO errors at warn).
pub(crate) fn package_json_text(root: &Path) -> Option<Arc<str>> {
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let path = root.join("package.json");
    // ERR-5 / TASK-0878: recover from poisoning by inheriting the inner
    // map. The cache value is the raw file text, not authoritative state,
    // so a panic in a previous holder cannot leave a torn invariant.
    if let Some(entry) = cache
        .lock()
        .unwrap_or_else(|e| {
            tracing::warn!("package.json cache mutex was poisoned by a prior panic; recovered");
            e.into_inner()
        })
        .get(&path)
    {
        return entry.clone();
    }
    let text =
        ops_about::manifest_io::read_optional_text(&path, "package.json").map(Arc::<str>::from);
    let mut guard = cache.lock().unwrap_or_else(|e| {
        tracing::warn!("package.json cache mutex was poisoned by a prior panic; recovered");
        e.into_inner()
    });
    if guard.len() >= CACHE_MAX_ENTRIES {
        tracing::debug!(
            cap = CACHE_MAX_ENTRIES,
            "package.json cache reached cap; clearing"
        );
        guard.clear();
    }
    guard.insert(path, text.clone());
    debug_assert!(
        guard.len() <= CACHE_MAX_ENTRIES,
        "package.json cache exceeded cap of {CACHE_MAX_ENTRIES}"
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
            dir.path().join("package.json"),
            r#"{"name":"demo","version":"0.1.0"}"#,
        )
        .unwrap();
        let a = package_json_text(dir.path()).unwrap();
        let b = package_json_text(dir.path()).unwrap();
        assert!(Arc::ptr_eq(&a, &b));
    }

    /// PERF-3 / TASK-0854 sister: the second consumer of the cached text
    /// must share the Arc allocation — proves the cache deduplicates the
    /// IO without forcing a re-read or text clone per consumer.
    #[test]
    fn arc_is_shared_across_two_consumer_parses() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"demo","version":"0.1.0","workspaces":["packages/*"]}"#,
        )
        .unwrap();
        let a = package_json_text(dir.path()).unwrap();
        let _: serde_json::Value =
            serde_json::from_str(&a).expect("identity-shape parse from shared text");
        let b = package_json_text(dir.path()).unwrap();
        let _: serde_json::Value =
            serde_json::from_str(&b).expect("workspace-shape parse from shared text");
        assert!(
            Arc::ptr_eq(&a, &b),
            "both consumers must share the cached Arc<str>"
        );
        assert!(Arc::strong_count(&a) >= 3);
    }

    #[test]
    fn missing_file_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(package_json_text(dir.path()).is_none());
    }

    /// ERR-5 / TASK-0878: a panic while holding the cache lock must not
    /// permanently brick the cache for every other provider in the
    /// process.
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

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"), r#"{"name":"recover"}"#).unwrap();
        let text = package_json_text(dir.path()).expect("text after poison recovery");
        assert!(text.contains("\"recover\""));
    }
}
