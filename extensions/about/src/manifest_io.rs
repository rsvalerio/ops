//! Shared "read this manifest if it exists" helper for the about extensions.
//!
//! Every per-stack about crate reads its manifests through this one helper,
//! so they all share the same policy: a missing manifest is silent, any
//! other failure logs at `tracing::warn!` (visible at default log levels),
//! the read is size-capped, and only regular files are read.

use std::io::Read;
use std::path::Path;

/// Hard cap on manifest size.
///
/// `ops about` runs in user-controlled working directories where an
/// adversarial repository could otherwise force an unbounded allocation.
/// 4 MiB is well above any real
/// `package.json` / `pom.xml` / `pnpm-workspace.yaml` while keeping a
/// single oversize read bounded.
pub const MAX_MANIFEST_BYTES: u64 = 4 * 1024 * 1024;

/// Read a manifest's text content if the file exists.
///
/// Returns `Some(content)` on success, `None` when the file is absent, is
/// not a regular file, is larger than [`MAX_MANIFEST_BYTES`], or could not
/// be read:
///
/// - `ErrorKind::NotFound` → silent `None` (a missing manifest is not an
///   error; the caller falls back to defaults).
/// - a non-regular file (FIFO, device, directory) → `tracing::warn!`,
///   returns `None`. The open never blocks, so a FIFO named like a manifest
///   in a hostile checkout cannot hang `ops about` waiting for a writer.
/// - any other IO error → emits `tracing::warn!` with `path` and `error`,
///   returns `None`: a permission-denied or EIO manifest read is a real
///   environment problem that the user needs to be told about.
///
/// `kind` is included in the log event so operators can grep by manifest
/// type ("package.json" vs "go.mod") without scraping paths.
pub fn read_optional_text(path: &Path, kind: &str) -> Option<String> {
    let mut file = match open_nonblocking(path) {
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

    // The type check is made on the open handle, so it describes the file
    // that will actually be read rather than whatever the path resolves to
    // on a second lookup. Only a regular file is read: a FIFO or device
    // could block the read or never reach end-of-file.
    let metadata = match file.metadata() {
        Ok(metadata) => metadata,
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
    if !metadata.is_file() {
        tracing::warn!(
            path = ?path.display(),
            kind = kind,
            "failed to read manifest: not a regular file"
        );
        return None;
    }

    // Pre-size the read buffer from the file length (clamped to
    // MAX_MANIFEST_BYTES) so a single allocation covers the whole manifest
    // instead of paying the doubling-resize cost on every read.
    let preallocate = metadata.len().min(MAX_MANIFEST_BYTES);
    // `preallocate` is already clamped to MAX_MANIFEST_BYTES, so the only
    // platform where this could truncate is one whose usize cannot hold the
    // cap; saturating there just means a smaller preallocation.
    let mut buf = String::with_capacity(usize::try_from(preallocate).unwrap_or(usize::MAX));
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

    // A length that does not fit in a `u64` is necessarily far above the
    // 4 MiB cap, so saturating to `u64::MAX` keeps this comparison exact for
    // every value the check can actually distinguish.
    if u64::try_from(buf.len()).unwrap_or(u64::MAX) > MAX_MANIFEST_BYTES {
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

/// Open `path` for reading without blocking.
///
/// `O_NONBLOCK` makes opening a FIFO return immediately instead of waiting
/// for a writer; it has no effect on reads from a regular file, which is
/// the only kind [`read_optional_text`] goes on to read.
#[cfg(unix)]
fn open_nonblocking(path: &Path) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt as _;
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
}

/// Open `path` for reading. Non-Unix targets have no FIFO that blocks
/// `open`; the regular-file check in [`read_optional_text`] still applies.
#[cfg(not(unix))]
fn open_nonblocking(path: &Path) -> std::io::Result<std::fs::File> {
    std::fs::File::open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `MAX_MANIFEST_BYTES` (plus `extra`) as a `usize`, for sizing test
    /// buffers.
    ///
    /// The cap is 4 MiB, which fits every `usize` these tests run on. On a
    /// hypothetical platform whose `usize` were narrower, `usize::MAX` would
    /// itself be below the cap, so the saturating fallback still yields a
    /// buffer the tests can allocate rather than an unwrap or a panic.
    fn cap_bytes_as_usize(extra: u64) -> usize {
        usize::try_from(MAX_MANIFEST_BYTES.saturating_add(extra)).unwrap_or(usize::MAX)
    }

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
        // Path is a directory: not a regular file, and not NotFound either.
        let dir = tempfile::tempdir().expect("tempdir");
        let result = read_optional_text(dir.path(), "test");
        assert!(result.is_none());
    }

    /// Files larger than `MAX_MANIFEST_BYTES` must not be
    /// slurped into memory. Use a sentinel-byte content larger than the cap
    /// and assert the helper bails to None.
    #[test]
    fn oversize_file_returns_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("huge.toml");
        let oversize = cap_bytes_as_usize(1);
        let content = vec![b'a'; oversize];
        std::fs::write(&p, &content).expect("write");
        assert!(read_optional_text(&p, "test").is_none());
    }

    #[test]
    fn at_cap_file_returns_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("at_cap.toml");
        let content = vec![b'a'; cap_bytes_as_usize(0)];
        std::fs::write(&p, &content).expect("write");
        let got = read_optional_text(&p, "test").expect("Some");
        assert_eq!(got.len(), cap_bytes_as_usize(0));
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

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(read_optional_text(&p, "package.json"));
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
