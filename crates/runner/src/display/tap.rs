//! Tap-file lifecycle for `ProgressDisplay`.
//!
//! ARCH-1 (TASK-0581): owns the tap-file state (open File handle, original
//! path for re-opening on failure, captured truncation kind) so the
//! progress-rendering surface no longer mixes tap concerns with indicatif
//! lifecycle. Behavior is preserved verbatim from the previous in-line
//! implementation in `display.rs`.

use std::fs::File;
use std::io::{ErrorKind, Write};
use std::path::PathBuf;

/// Tap-file writer with truncation tracking.
///
/// On the first write failure the inner `File` handle is dropped to suppress
/// the rest of the run's tap output (avoiding a noisy log per line). The
/// failure kind and the step id that triggered it are captured so
/// `on_run_finished` can emit a single user-visible "tap truncated" warning
/// and best-effort append a marker line at the end of the tap file itself.
pub(crate) struct TapWriter {
    file: Option<File>,
    path: Option<PathBuf>,
    truncation: Option<(String, String)>,
    /// CONC-7 / TASK-1176: capture the `io::ErrorKind` of the first write
    /// failure so [`Self::append_marker`] can short-circuit when re-opening
    /// the same path could only fail again (e.g. ENOSPC) or hang (e.g. a
    /// stale NFS mount). Pre-fix `append_marker` blindly re-opened on every
    /// truncation, producing two consecutive log lines under disk-full and
    /// re-issuing blocking IO on a hung mount on the synchronous display
    /// thread.
    truncation_kind: Option<ErrorKind>,
}

impl TapWriter {
    pub(crate) fn new(path: PathBuf) -> Self {
        let file = match File::create(&path) {
            Ok(f) => Some(f),
            Err(e) => {
                // ERR-7 (TASK-0940): Debug-format path/error so a tap path
                // configured in `.ops.toml` containing newlines or ANSI
                // escapes cannot forge log records.
                tracing::warn!(path = ?path.display(), error = ?e, "failed to open tap file");
                None
            }
        };
        Self {
            file,
            path: Some(path),
            truncation: None,
            truncation_kind: None,
        }
    }

    /// Append `line` plus a newline to the tap file. On the first IO error
    /// the file handle is dropped and the failure recorded in
    /// [`Self::truncation`]; subsequent calls become no-ops.
    pub(crate) fn write_line(&mut self, line: &str, step_id: Option<&str>) {
        // ERR-1: previously dropped the writeln Result silently; a broken tap
        // fd (disk full, NFS drop, closed underneath) would swallow every
        // subsequent line without the user seeing any diagnostic.
        // ERR-2 / TASK-0458: on first failure capture the kind + step id so
        // RunFinished can emit a user-visible "tap truncated" line. We do
        // not retry: the inner File is `std::fs::File` whose write
        // interface does not surface EAGAIN distinctly, and a retry-once
        // strategy is documented as optional in the task. Subsequent lines
        // no-op rather than spamming.
        if let Some(ref mut f) = self.file {
            if let Err(e) = writeln!(f, "{}", line) {
                tracing::debug!(error = %e, "tap file write failed; disabling further tap writes");
                self.truncation = Some((
                    step_id.unwrap_or("<unknown>").to_string(),
                    e.kind().to_string(),
                ));
                self.truncation_kind = Some(e.kind());
                self.file = None;
            }
        }
    }

    /// Drain a pending truncation record. Returns `Some((step_id, kind))`
    /// once if a write previously failed, `None` afterwards.
    pub(crate) fn take_truncation(&mut self) -> Option<(String, String)> {
        self.truncation.take()
    }

    /// Best-effort: re-open the tap file in append mode and write a final
    /// marker line. Used by `on_run_finished` so a downstream parser that
    /// only inspects the file (no stderr capture) still sees the truncation.
    /// If the open also fails, the caller's stderr warning is the only
    /// visible signal.
    pub(crate) fn append_marker(&self, line: &str) {
        let Some(path) = self.path.as_ref() else {
            return;
        };
        // CONC-7 / TASK-1176: skip the re-open when the prior failure kind
        // makes a successful retry impossible. Re-opening under disk-full
        // (`StorageFull`) only repeats the failure with an extra log line,
        // and re-opening under broken-pipe (`BrokenPipe`) on a hung NFS
        // mount can issue blocking IO on the display thread. Both
        // user-visible signals (the stderr warning + drained
        // `take_truncation`) already exist; the marker is best-effort and
        // not worth a second round of blocking IO. Other kinds (EACCES
        // recovered after a chmod, transient EIO) fall through to the
        // existing append attempt.
        if matches!(
            self.truncation_kind,
            Some(ErrorKind::StorageFull) | Some(ErrorKind::BrokenPipe)
        ) {
            tracing::debug!(
                target: "ops::tap",
                kind = ?self.truncation_kind,
                "skipping tap marker re-open: prior failure kind cannot succeed on retry"
            );
            return;
        }
        // ERR-2 / TASK-0775: log distinct breadcrumbs for the open vs write
        // failure modes. Function stays infallible (best-effort), but a
        // silent partial tap with no stderr capture had no postmortem
        // signal at all under the previous swallow-everything path.
        match std::fs::OpenOptions::new().append(true).open(path) {
            Ok(mut f) => {
                if let Err(e) = writeln!(f, "{line}") {
                    // ERR-7 (TASK-0940): Debug-format path/error so a tap
                    // path with embedded control characters cannot forge log
                    // lines.
                    tracing::warn!(
                        target: "ops::tap",
                        error = ?e,
                        path = ?path.display(),
                        "tap append-marker write failed",
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    target: "ops::tap",
                    error = ?e,
                    path = ?path.display(),
                    "tap append-marker open failed",
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// ERR-7 (TASK-0940): tracing fields for tap paths flow through the `?`
    /// formatter so a config-supplied path containing newlines or ANSI
    /// escapes cannot forge log records.
    /// CONC-7 / TASK-1176: when the prior write failure kind is
    /// `StorageFull` or `BrokenPipe`, `append_marker` must short-circuit
    /// rather than re-open the path. Re-opening can only repeat the
    /// failure (ENOSPC) or hang on a stale mount (BrokenPipe). We pin
    /// the short-circuit by setting the recorded truncation kind and
    /// asserting that the marker line is *not* written to the file.
    #[test]
    fn append_marker_skips_reopen_on_storage_full() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tap.txt");
        let mut tap = TapWriter::new(path.clone());
        // Simulate a prior failure with kind=StorageFull. Drop the file
        // handle so the marker path is the only writer.
        tap.file = None;
        tap.truncation_kind = Some(ErrorKind::StorageFull);

        // Write a known initial body to disk so the test can verify
        // `append_marker` did NOT touch the file.
        std::fs::write(&path, b"baseline\n").unwrap();
        tap.append_marker("MARKER");
        let after = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            after, "baseline\n",
            "append_marker must short-circuit on StorageFull and not re-open the file"
        );
    }

    #[test]
    fn append_marker_skips_reopen_on_broken_pipe() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tap.txt");
        let mut tap = TapWriter::new(path.clone());
        tap.file = None;
        tap.truncation_kind = Some(ErrorKind::BrokenPipe);

        std::fs::write(&path, b"baseline\n").unwrap();
        tap.append_marker("MARKER");
        let after = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            after, "baseline\n",
            "append_marker must short-circuit on BrokenPipe and not re-open the file"
        );
    }

    /// Other failure kinds still attempt the re-open (the prior
    /// best-effort behaviour) so a transient EACCES recovered between
    /// the failed write and the marker still has a chance to land.
    #[test]
    fn append_marker_still_attempts_reopen_on_other_kinds() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tap.txt");
        let mut tap = TapWriter::new(path.clone());
        tap.file = None;
        tap.truncation_kind = Some(ErrorKind::PermissionDenied);

        std::fs::write(&path, "baseline\n").unwrap();
        tap.append_marker("MARKER");
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(
            after.contains("MARKER"),
            "non-StorageFull/BrokenPipe kind should still attempt the marker append; got: {after}"
        );
    }

    #[test]
    fn tap_path_debug_escapes_control_characters() {
        let p = Path::new("a\nb\u{1b}[31mc/tap.txt");
        let rendered = format!("{:?}", p.display());
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains('\u{1b}'));
        assert!(rendered.contains("\\n"));
    }
}
