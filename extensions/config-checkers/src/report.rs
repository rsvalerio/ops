//! What a checker run produces: per-file failures and the summary line.

use std::io::Write;

// DUP-2 / TASK-2162: the per-file outcome vocabulary — failure kinds and the
// failed-file record — is shared with the text fixers through one definition
// in `ops_core::bounded_read`, so the two file-walking extensions cannot
// drift on what a failure means. `FailureKind::Write` is the fixers' kind;
// this crate never constructs it, and `Parse` is the one only checkers use.
pub use ops_core::bounded_read::{FailedFile, FailureKind};

/// Outcome of a checker run.
///
/// API-5 / TASK-2135: the `#[must_use]` sits on the *type*, not on the
/// `run_check_*` functions, so it survives `?` — discarding the report after
/// unwrapping the `Result` is still a warning, because the report (via
/// [`CheckerReport::failed`]) is what drives the process exit code.
#[must_use = "the report drives the process exit code; dropping it after `?` exits 0 on a failed run"]
#[derive(Debug, Default)]
pub struct CheckerReport {
    /// Files that were actually read *and* handed to the parser. A file whose
    /// metadata or read failed was not scanned in any sense and is not
    /// counted here.
    pub files_scanned: usize,
    /// Files the run could not complete, with the failure kind and message.
    /// Paths are relative to [`crate::CheckerOptions::root`]. Any entry at
    /// all means the run failed — see [`CheckerReport::failed`].
    pub files_failed: Vec<FailedFile>,
    /// Files that were not validated: over [`crate::CheckerOptions::max_bytes`],
    /// not a regular file, or gone from the worktree by the time the checker
    /// reached them. Counted separately from `files_scanned` so callers can
    /// distinguish "validated and OK" from "not validated at all".
    pub files_skipped: usize,
    /// Directories the discovery walk could not traverse. Each one hides an
    /// unknown number of candidates, so a run that carries any of these did
    /// not see the whole tree and must not report "clean" — see
    /// [`CheckerReport::failed`].
    pub walk_errors: Vec<String>,
}

impl ops_core::bounded_read::FileRunReport for CheckerReport {
    fn push_failure(&mut self, failure: FailedFile) {
        self.files_failed.push(failure);
    }

    fn adopt_walk_errors(&mut self, errors: Vec<String>) {
        self.walk_errors = errors;
    }
}

impl CheckerReport {
    /// Whether the run should be treated as a failure by the caller.
    ///
    /// A walk error counts: the checker validated every candidate it was
    /// given, but traversal silently omitted candidates it never learned
    /// about. Exiting 0 there is fail-open — the CLI would report a clean
    /// tree over directories it could not read.
    #[must_use]
    pub const fn failed(&self) -> bool {
        !self.files_failed.is_empty() || !self.walk_errors.is_empty()
    }
}

/// One-line summary for the CLI.
///
/// # Errors
/// Propagates writer I/O errors so a broken pipe in CI is not silently
/// hidden from the caller.
pub fn write_summary(
    report: &CheckerReport,
    label: &str,
    writer: &mut dyn Write,
) -> std::io::Result<()> {
    writeln!(
        writer,
        "{label}: scanned {} file(s), {} failed, {} skipped, {} walk error(s)",
        report.files_scanned,
        report.files_failed.len(),
        report.files_skipped,
        report.walk_errors.len(),
    )
}
