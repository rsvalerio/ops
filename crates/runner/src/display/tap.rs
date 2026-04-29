//! Tap-file lifecycle for `ProgressDisplay`.
//!
//! ARCH-1 (TASK-0581): owns the tap-file state (open File handle, original
//! path for re-opening on failure, captured truncation kind) so the
//! progress-rendering surface no longer mixes tap concerns with indicatif
//! lifecycle. Behavior is preserved verbatim from the previous in-line
//! implementation in `display.rs`.

use std::fs::File;
use std::io::Write;
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
}

impl TapWriter {
    pub(crate) fn new(path: PathBuf) -> Self {
        let file = match File::create(&path) {
            Ok(f) => Some(f),
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "failed to open tap file");
                None
            }
        };
        Self {
            file,
            path: Some(path),
            truncation: None,
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
        if let Some(path) = self.path.as_ref() {
            if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(path) {
                let _ = writeln!(f, "{line}");
            }
        }
    }
}
