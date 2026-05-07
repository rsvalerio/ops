//! Process-local cache for the raw `package.json` text.
//!
//! Thin wrapper over [`ops_about::manifest_cache::ArcTextCache`]
//! (DUP-1 / TASK-0973). Policy — cap, poison recovery, log wording — lives
//! in `ops-about` so the Node and Python copies cannot drift; this module
//! only names the filename and exposes the typed entry point used by the
//! identity and units providers.
//!
//! See `ops_about::manifest_cache` for the rationale (PERF-3 / TASK-0854,
//! ARCH-1 / TASK-0867, ERR-5 / TASK-0878).

use std::path::Path;
use std::sync::Arc;

use ops_about::manifest_cache::ArcTextCache;

static CACHE: ArcTextCache = ArcTextCache::new("package.json");

/// Read `<root>/package.json` once per process, returning the raw text as
/// a shared `Arc<str>`. Returns `None` when the file is missing or
/// unreadable.
pub(crate) fn package_json_text(root: &Path) -> Option<Arc<str>> {
    CACHE.read(root)
}

#[cfg(test)]
mod tests {
    //! Tests construct a fresh local [`ArcTextCache`] per case so the
    //! shared static never bleeds across the test binary
    //! (TEST-18 / TASK-0956). The poison-recovery scenario lives in
    //! `ops_about::manifest_cache` against a local instance there.

    use super::*;

    #[test]
    fn second_call_returns_same_arc() {
        let cache = ArcTextCache::new("package.json");
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"demo","version":"0.1.0"}"#,
        )
        .unwrap();
        let a = cache.read(dir.path()).unwrap();
        let b = cache.read(dir.path()).unwrap();
        assert!(Arc::ptr_eq(&a, &b));
    }

    /// PERF-3 / TASK-0854 sister: the second consumer of the cached text
    /// must share the Arc allocation — proves the cache deduplicates the
    /// IO without forcing a re-read or text clone per consumer.
    #[test]
    fn arc_is_shared_across_two_consumer_parses() {
        let cache = ArcTextCache::new("package.json");
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"demo","version":"0.1.0","workspaces":["packages/*"]}"#,
        )
        .unwrap();
        let a = cache.read(dir.path()).unwrap();
        let _: serde_json::Value =
            serde_json::from_str(&a).expect("identity-shape parse from shared text");
        let b = cache.read(dir.path()).unwrap();
        let _: serde_json::Value =
            serde_json::from_str(&b).expect("workspace-shape parse from shared text");
        assert!(
            Arc::ptr_eq(&a, &b),
            "both consumers must share the cached Arc<str>"
        );
        // Local cache + 2 captured Arcs in test = 3.
        assert_eq!(Arc::strong_count(&a), 3);
    }

    #[test]
    fn missing_file_returns_none() {
        let cache = ArcTextCache::new("package.json");
        let dir = tempfile::tempdir().unwrap();
        assert!(cache.read(dir.path()).is_none());
    }

    /// Smoke check that the production wrapper still routes through the
    /// shared cache. Uses a unique tempdir so it cannot collide with any
    /// other static-cache user in this binary.
    #[test]
    fn package_json_text_returns_arc() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"), r#"{"name":"smoke"}"#).unwrap();
        let text = package_json_text(dir.path()).expect("text");
        assert!(text.contains("smoke"));
    }
}
