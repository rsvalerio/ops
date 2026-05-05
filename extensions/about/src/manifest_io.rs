//! Shared "read this manifest if it exists" helper for the about extensions.
//!
//! ERR-2 (TASK-0622): the per-stack about crates each had a near-identical
//! `match std::fs::read_to_string` block that downgraded NotFound to silence
//! and other IO errors to `tracing::debug!`. Six copies meant the next
//! copy/paste would silently drift the policy (TASK-0467 already filed one
//! such drift in the duckdb providers). This helper centralises the rule
//! so adding a stack inherits the consistent severity policy.
//!
//! TASK-0649: non-NotFound IO errors now log at `tracing::warn!` matching
//! the sibling `try_read_manifest` / `resolve_member_globs` policy so that
//! operators at default log levels see unreadable-manifest diagnostics.

use std::io::Read;
use std::path::Path;

/// SEC-33 (TASK-0831): hard cap on manifest size. `ops about` runs in
/// user-controlled working directories where an adversarial repository
/// (or a `/dev/zero` symlink) could otherwise force an unbounded
/// allocation. 4 MiB is well above any real `package.json` / `pom.xml`
/// / `pnpm-workspace.yaml` while keeping a single oversize read bounded.
pub const MAX_MANIFEST_BYTES: u64 = 4 * 1024 * 1024;

/// Read a manifest's text content if the file exists.
///
/// Returns `Some(content)` on success, `None` when the file is absent or
/// when an unrelated IO error occurred. The classification matches the
/// six pre-existing call sites:
///
/// - `ErrorKind::NotFound` → silent `None` (a missing manifest is not an
///   error; the caller falls back to defaults).
/// - any other IO error → emits `tracing::warn!` with `path` and `error`,
///   returns `None`. This matches the policy established by
///   `try_read_manifest` (TASK-0548) and `resolve_member_globs` (TASK-0517):
///   a permission-denied or EIO manifest read is a real environment problem
///   that the user needs to be told about.
///
/// `kind` is included in the log event so operators can grep by manifest
/// type ("package.json" vs "go.mod") without scraping paths.
pub fn read_optional_text(path: &Path, kind: &str) -> Option<String> {
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            tracing::warn!(
                path = ?path.display(),
                error = ?e,
                kind = kind,
                "failed to read manifest"
            );
            return None;
        }
    };

    // PERF-1 / TASK-0971: pre-size the read buffer from file metadata
    // (clamped to MAX_MANIFEST_BYTES) so a single allocation covers the
    // whole manifest instead of paying the doubling-resize cost on every
    // read. The metadata-unknown branch falls back to `String::new()` so
    // the SEC-33 cap and oversize-bail policy stay identical.
    let preallocate = file
        .metadata()
        .ok()
        .map(|m| m.len().min(MAX_MANIFEST_BYTES))
        .unwrap_or(0);
    let mut buf = String::with_capacity(preallocate as usize);
    let limit = MAX_MANIFEST_BYTES.saturating_add(1);
    match (&mut file).take(limit).read_to_string(&mut buf) {
        Ok(_) => {}
        Err(e) => {
            tracing::warn!(
                path = ?path.display(),
                error = ?e,
                kind = kind,
                "failed to read manifest"
            );
            return None;
        }
    }

    if buf.len() as u64 > MAX_MANIFEST_BYTES {
        tracing::warn!(
            path = ?path.display(),
            kind = kind,
            cap = MAX_MANIFEST_BYTES,
            "manifest exceeds size cap; skipping"
        );
        return None;
    }

    Some(buf)
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
    fn other_io_error_returns_none_after_warn_log() {
        // Path is a directory, so read_to_string returns IsADirectory (not NotFound).
        let dir = tempfile::tempdir().expect("tempdir");
        let result = read_optional_text(dir.path(), "test");
        assert!(result.is_none());
    }

    /// SEC-33 (TASK-0831): files larger than MAX_MANIFEST_BYTES must not be
    /// slurped into memory. Use a sentinel-byte content larger than the cap
    /// and assert the helper bails to None.
    #[test]
    fn oversize_file_returns_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("huge.toml");
        let oversize = (MAX_MANIFEST_BYTES + 1) as usize;
        let content = vec![b'a'; oversize];
        std::fs::write(&p, &content).expect("write");
        assert!(read_optional_text(&p, "test").is_none());
    }

    #[test]
    fn at_cap_file_returns_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("at_cap.toml");
        let content = vec![b'a'; MAX_MANIFEST_BYTES as usize];
        std::fs::write(&p, &content).expect("write");
        let got = read_optional_text(&p, "test").expect("Some");
        assert_eq!(got.len(), MAX_MANIFEST_BYTES as usize);
    }

    /// ERR-7 (TASK-0665): paths must be Debug-formatted in log fields so
    /// embedded newlines/ANSI escapes cannot forge log lines. This test
    /// pins the formatting choice without requiring a tracing-subscriber
    /// dependency: the same `?` formatter used in the `tracing::warn!` call
    /// site escapes control characters at the value layer.
    /// ERR-7 / TASK-0999: `io::Error` messages flowing through the Debug
    /// formatter must escape control characters so a hostile filename or
    /// symlink-target whose error message contains `\n` or `\u{1b}[31m`
    /// cannot forge log lines.
    #[test]
    fn io_error_debug_escapes_control_characters() {
        let e = std::io::Error::other("rogue\nINJECTED line\u{1b}[31m");
        let rendered = format!("{e:?}");
        assert!(
            !rendered.contains('\n'),
            "raw newline leaked into Debug rendering: {rendered}"
        );
        assert!(
            !rendered.contains('\u{1b}'),
            "raw ANSI ESC leaked into Debug rendering: {rendered}"
        );
        assert!(
            rendered.contains("\\n"),
            "expected escaped newline in Debug rendering: {rendered}"
        );
    }

    #[test]
    fn path_display_debug_escapes_control_characters() {
        let p = Path::new("a\nb\u{1b}[31mc");
        let rendered = format!("{:?}", p.display());
        assert!(
            !rendered.contains('\n'),
            "raw newline leaked into log value: {rendered}"
        );
        assert!(
            !rendered.contains('\u{1b}'),
            "raw ANSI ESC leaked into log value: {rendered}"
        );
        assert!(
            rendered.contains("\\n"),
            "expected escaped newline in {rendered}"
        );
    }
}
