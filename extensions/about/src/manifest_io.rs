//! Shared "read this manifest if it exists" helper for the about extensions.
//!
//! Every per-stack about crate reads its manifests through this one helper,
//! so they all share the same policy: a missing manifest is silent, a
//! manifest that resolves outside the workspace root — a root-level manifest
//! that is itself a symlink (`package.json -> /some/other/file`), or one
//! reached through a symlink that escapes — is refused with a
//! `tracing::warn!`, any other failure logs at `tracing::warn!` (visible at
//! default log levels), the read is size-capped, and only regular files are
//! read.

use std::path::Path;

/// Read a manifest's text content if it exists and stays inside `root`.
///
/// The manifest path is canonicalized (which follows symlinks) and must sit
/// under the canonicalized `root`, mirroring the workspace-member read in
/// `ops_about::workspace`: a root manifest that is a symlink out of the
/// workspace is skipped with a `tracing::warn!` rather than having its
/// fields surfaced, while a symlink that stays inside the root is followed.
/// The read itself goes through [`ops_core::text::read_capped_to_string`] on
/// the canonical path: that open refuses a symlink at every component and any
/// non-regular file, so an entry swapped in after the containment check
/// cannot redirect the read, and a FIFO named like a manifest cannot block
/// `ops about` waiting for a writer.
///
/// The size cap is the process-wide [`ops_core::text::manifest_max_bytes`]
/// (default 4 MiB, override via `OPS_MANIFEST_MAX_BYTES`) — the same cap the
/// workspace-member read uses, so root and member manifests share one
/// policy.
///
/// Returns `Some(content)` on success, `None` when the file is absent,
/// resolves outside `root`, is not a regular file, exceeds the cap, or could
/// not be read:
///
/// - `ErrorKind::NotFound` → silent `None` (a missing manifest is not an
///   error; the caller falls back to defaults).
/// - resolves outside the workspace root → `tracing::warn!`, returns `None`.
/// - a non-regular file (FIFO, device, directory) → `tracing::warn!`,
///   returns `None`.
/// - any other IO error → emits `tracing::warn!` with `path` and `error`,
///   returns `None`: a permission-denied or EIO manifest read is a real
///   environment problem that the user needs to be told about.
///
/// `kind` is included in the log event so operators can grep by manifest
/// type ("package.json" vs "go.mod") without scraping paths.
pub fn read_optional_text(path: &Path, root: &Path, kind: &str) -> Option<String> {
    let canonical = match std::fs::canonicalize(path) {
        Ok(canonical) => canonical,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            warn_failed_to_read(path, kind, &e);
            return None;
        }
    };
    // Fail closed: without a canonical root, containment cannot be shown.
    // Both sides are canonicalized so a symlinked operator prefix (`/var` on
    // macOS) resolves the same way on each side of the comparison.
    let contained = std::fs::canonicalize(root).is_ok_and(|root| canonical.starts_with(root));
    if !contained {
        tracing::warn!(
            path = ?path.display(),
            kind = kind,
            "manifest resolves outside the workspace root; skipping"
        );
        return None;
    }
    match ops_core::text::read_capped_to_string(&canonical) {
        Ok(content) => Some(content),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            warn_failed_to_read(path, kind, &e);
            None
        }
    }
}

/// Log a manifest that exists but could not be read.
///
/// The path and error are Debug-formatted so embedded newlines / ANSI
/// escapes cannot forge log lines.
fn warn_failed_to_read(path: &Path, kind: &str, error: &std::io::Error) {
    tracing::warn!(
        path = ?path.display(),
        error = ?error,
        kind = kind,
        "failed to read manifest"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_returns_none_silently() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("does-not-exist.toml");
        assert!(read_optional_text(&p, dir.path(), "test").is_none());
    }

    #[test]
    fn present_file_returns_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("a.txt");
        std::fs::write(&p, "hello").expect("write");
        assert_eq!(
            read_optional_text(&p, dir.path(), "test").as_deref(),
            Some("hello")
        );
    }

    #[cfg(unix)]
    #[test]
    fn other_io_error_returns_none_after_warn_log() {
        // Path is a directory: not a regular file, and not NotFound either.
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("subdir");
        std::fs::create_dir(&p).expect("mkdir");
        let result = read_optional_text(&p, dir.path(), "test");
        assert!(result.is_none());
    }

    /// Files larger than the effective cap must not be slurped into memory.
    /// Use a sentinel-byte content larger than the cap and assert the helper
    /// bails to None.
    #[test]
    fn oversize_file_returns_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("huge.toml");
        let oversize = usize::try_from(ops_core::text::manifest_max_bytes().saturating_add(1))
            .unwrap_or(usize::MAX);
        let content = vec![b'a'; oversize];
        std::fs::write(&p, &content).expect("write");
        assert!(read_optional_text(&p, dir.path(), "test").is_none());
    }

    #[test]
    fn at_cap_file_returns_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("at_cap.toml");
        let at_cap = usize::try_from(ops_core::text::manifest_max_bytes()).unwrap_or(usize::MAX);
        let content = vec![b'a'; at_cap];
        std::fs::write(&p, &content).expect("write");
        let got = read_optional_text(&p, dir.path(), "test").expect("Some");
        assert_eq!(got.len(), at_cap);
    }

    /// SEC-14 / TASK-2433: a root-level manifest that is itself a symlink
    /// pointing outside the workspace root must be refused (with a warn),
    /// not read and surfaced in `ops about` output.
    #[cfg(unix)]
    #[test]
    fn root_manifest_symlinked_outside_the_root_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("tempdir");
        std::fs::write(outside.path().join("real.json"), "{}").expect("write");

        let link = dir.path().join("package.json");
        std::os::unix::fs::symlink(outside.path().join("real.json"), &link).expect("symlink");

        assert!(
            read_optional_text(&link, dir.path(), "package.json").is_none(),
            "a manifest symlinked out of the workspace root must not be read"
        );
    }

    /// The inside-root counterpart: a symlink that stays inside the workspace
    /// is followed, matching the workspace-member read policy.
    #[cfg(unix)]
    #[test]
    fn root_manifest_symlinked_inside_the_root_is_followed() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("real.txt"), "hello").expect("write");

        let link = dir.path().join("a.txt");
        std::os::unix::fs::symlink(dir.path().join("real.txt"), &link).expect("symlink");

        assert_eq!(
            read_optional_text(&link, dir.path(), "test").as_deref(),
            Some("hello")
        );
    }

    /// A FIFO named like a manifest must be rejected promptly instead of
    /// blocking on a writer that never appears. The read runs on its own
    /// thread so a regression fails the test rather than hanging the suite.
    #[cfg(unix)]
    #[test]
    fn fifo_returns_none_without_blocking() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("package.json");
        let status = std::process::Command::new("mkfifo")
            .arg(&p)
            .status()
            .expect("run mkfifo");
        assert!(status.success(), "mkfifo failed");

        let root = dir.path().to_path_buf();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(read_optional_text(&p, &root, "package.json"));
        });
        let result = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("read_optional_text blocked on a FIFO");
        assert!(result.is_none(), "a FIFO must not be read as a manifest");
    }

    /// Paths must be Debug-formatted in log fields so
    /// embedded newlines/ANSI escapes cannot forge log lines. This test
    /// pins the formatting choice without requiring a tracing-subscriber
    /// dependency: the same `?` formatter used in the `tracing::warn!` call
    /// site escapes control characters at the value layer.
    /// `io::Error` messages flowing through the Debug
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
