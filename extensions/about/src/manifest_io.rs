//! Shared "read this manifest if it exists" helper for the about extensions.
//!
//! ERR-2 (TASK-0622): the per-stack about crates each had a near-identical
//! `match std::fs::read_to_string` block that downgraded NotFound to silence
//! and other IO errors to `tracing::debug!`. Six copies meant the next
//! copy/paste would silently drift the policy (TASK-0467 already filed one
//! such drift in the duckdb providers). This helper centralises the rule
//! so adding a stack inherits the consistent severity policy.

use std::path::Path;

/// Read a manifest's text content if the file exists.
///
/// Returns `Some(content)` on success, `None` when the file is absent or
/// when an unrelated IO error occurred. The classification matches the
/// six pre-existing call sites:
///
/// - `ErrorKind::NotFound` → silent `None` (a missing manifest is not an
///   error; the caller falls back to defaults).
/// - any other IO error → emits `tracing::debug!` with `path` and `error`,
///   returns `None`. Stack-specific callers can still log a `tracing::warn!`
///   on parse failure separately — this helper covers IO classification only.
///
/// `kind` is included in the log event so operators can grep by manifest
/// type ("package.json" vs "go.mod") without scraping paths.
pub fn read_optional_text(path: &Path, kind: &str) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(content) => Some(content),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            tracing::debug!(
                path = %path.display(),
                error = %e,
                kind = kind,
                "failed to read manifest"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_returns_none_silently() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("does-not-exist.toml");
        assert!(read_optional_text(&p, "test").is_none());
    }

    #[test]
    fn present_file_returns_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("a.txt");
        std::fs::write(&p, "hello").expect("write");
        assert_eq!(read_optional_text(&p, "test").as_deref(), Some("hello"));
    }

    #[cfg(unix)]
    #[test]
    fn other_io_error_returns_none_after_debug_log() {
        // Path is a directory, so read_to_string returns IsADirectory (not NotFound).
        let dir = tempfile::tempdir().expect("tempdir");
        let result = read_optional_text(dir.path(), "test");
        assert!(result.is_none());
    }
}
