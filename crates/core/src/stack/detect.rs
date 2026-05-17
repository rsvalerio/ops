//! Filesystem walk that resolves a [`Stack`] from manifest presence.
//!
//! ARCH-1 / TASK-1185: extracted from the monolithic `stack.rs` so the
//! ancestor-walk and per-extension probe code lives separately from the
//! enum + embedded TOML metadata table.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use strum::IntoEnumIterator;

use super::Stack;

/// PERF-3 (TASK-1410): canonicalize is one stat per path component plus a
/// symlink dereference each level. `Stack::detect` runs once per CLI
/// dispatch and on a deep cwd / NFS / FUSE mount the syscall fan-out shows
/// up on the critical-path. Cache the resolved `(start -> canonical)`
/// mapping per process so repeat invocations from the same start path
/// skip the syscalls entirely. Fallback behaviour (lexical walk + tracing
/// debug breadcrumb on error) is preserved on the first miss; subsequent
/// hits replay the cached resolution.
static CANONICALIZE_CACHE: OnceLock<Mutex<HashMap<PathBuf, PathBuf>>> = OnceLock::new();

fn canonicalize_cached(start: &Path) -> PathBuf {
    let cache = CANONICALIZE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    // ERR-5 / TASK-1470 + DUP-3 / TASK-1477: poisoning here is recoverable
    // — the protected `HashMap<PathBuf, PathBuf>` has no invariant a
    // panicking caller could have broken. Route through the shared
    // `sync::lock_recover` so a single poison does not turn every later
    // `Stack::detect` into a hard panic for the rest of the process (which
    // is reachable from production CLI dispatch). Mirrors the policy used
    // in `expand.rs`.
    if let Some(p) = crate::sync::lock_recover(cache).get(start) {
        return p.clone();
    }
    let resolved = match std::fs::canonicalize(start) {
        Ok(p) => p,
        Err(e) => {
            tracing::debug!(
                path = ?start.display(),
                error = ?e,
                "Stack::detect could not canonicalize start; falling back to lexical walk"
            );
            start.to_path_buf()
        }
    };
    crate::sync::lock_recover(cache).insert(start.to_path_buf(), resolved.clone());
    resolved
}

/// Test seam (PERF-3 / TASK-1410 AC#3): returns `true` once `start` has
/// been resolved and cached by [`canonicalize_cached`]. The previous
/// counter-based seam was racy under parallel tests; querying the cache
/// directly is per-path and unaffected by other tests' detect() calls.
#[cfg(test)]
pub(super) fn canonicalize_cache_contains(start: &Path) -> bool {
    CANONICALIZE_CACHE
        .get()
        .map(|c| crate::sync::lock_recover(c).contains_key(start))
        .unwrap_or(false)
}

/// SEC-25: probe a manifest path with `try_exists` so transient errors are
/// logged rather than silently swallowed by `Path::exists`.
pub(super) fn manifest_present(path: &Path) -> bool {
    match path.try_exists() {
        Ok(present) => present,
        Err(err) => {
            // ERR-7 (TASK-0945): Debug-format path/error so a CWD-relative
            // ancestor probe path containing newlines / ANSI escapes cannot
            // forge log records.
            tracing::debug!(
                path = ?path.display(),
                error = ?err,
                "stack manifest probe failed; treating as not present",
            );
            false
        }
    }
}

/// File extensions used for extension-based detection (in addition to exact manifest files).
fn manifest_extensions(stack: Stack) -> &'static [&'static str] {
    match stack {
        Stack::Terraform => &["tf"],
        _ => &[],
    }
}

/// Whether `stack` has a manifest (exact filename or extension match) in `dir`.
pub(super) fn has_manifest_in_dir(stack: Stack, dir: &Path) -> bool {
    if stack
        .manifest_files()
        .iter()
        .any(|f| manifest_present(&dir.join(f)))
    {
        return true;
    }
    let extensions = manifest_extensions(stack);
    if !extensions.is_empty() {
        if let Ok(entries) = dir.read_dir() {
            // ERR-1 (TASK-0935): explicit match so a per-entry IO error
            // leaves a `tracing::debug` breadcrumb instead of silently
            // making the manifest "not found".
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::debug!(
                            parent = ?dir.display(),
                            error = ?e,
                            "stack manifest extension probe: read_dir entry failed; skipping",
                        );
                        continue;
                    }
                };
                if let Some(ext) = entry.path().extension() {
                    if extensions.iter().any(|e| ext == *e) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// ERR-5 / TASK-1470: regression — pollute the canonicalize cache via a
/// thread that panics while holding the lock, then assert the next
/// `detect()` call still resolves rather than hard-panicking. Pre-fix,
/// both lock sites in this module used `.expect("canonicalize cache
/// poisoned")` which would propagate the poison as a panic for the rest
/// of the process.
#[cfg(test)]
#[test]
fn detect_recovers_from_poisoned_canonicalize_cache() {
    use std::sync::Arc;

    // Force the cache to be initialised before we poison it so the
    // post-poison `detect` call observes a populated, then-recovered map.
    let dir = tempfile::tempdir().expect("tempdir");
    let _ = detect(dir.path());

    let cache: Arc<&'static Mutex<HashMap<PathBuf, PathBuf>>> = Arc::new(
        CANONICALIZE_CACHE
            .get()
            .expect("cache populated by detect() above"),
    );
    let poisoner = std::thread::spawn({
        let cache = Arc::clone(&cache);
        move || {
            let _guard = cache.lock().expect("lock");
            panic!("synthetic poison for TASK-1470");
        }
    });
    // Join must report Err — confirming the thread panicked and left the
    // mutex poisoned.
    assert!(
        poisoner.join().is_err(),
        "poisoner thread must have panicked to poison the lock"
    );

    // Production policy: poison must not propagate. `detect` should
    // continue to resolve.
    let _ = detect(dir.path());
}

/// Walk ancestors of `start` looking for a manifest match.
///
/// SEC-25 / TASK-0902: canonicalize once so the `pop()` walk operates on
/// the resolved chain. Reaching the cwd through a symlink would otherwise
/// let lexical `..` traversal yield ancestors outside the canonical
/// workspace, picking up a sibling project's manifests.
pub(super) fn detect(start: &Path) -> Option<Stack> {
    let mut current = canonicalize_cached(start);
    // READ-6 (TASK-1404): detection priority follows the `Stack` variant
    // declaration order; `Generic` has no manifest and is skipped via
    // `manifest_files().is_empty()`.
    for _ in 0..Stack::MAX_DETECT_DEPTH {
        if let Some(stack) = Stack::iter()
            .filter(|s| !s.manifest_files().is_empty())
            .find(|s| has_manifest_in_dir(*s, &current))
        {
            return Some(stack);
        }
        if !current.pop() {
            return None;
        }
    }
    None
}
