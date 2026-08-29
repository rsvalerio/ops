//! What a checker run produces: per-file failures and the summary line.

use std::io::{self, Write};
use std::path::PathBuf;

/// Why a file is in [`CheckerReport::files_failed`].
///
/// Only [`FailureKind::Parse`] is a *check* failure. The I/O kinds mean the
/// checker never got to look at the content, which is a different verdict
/// even though both are rendered on the same line — the CLI maps
/// [`CheckerReport::failed`] onto a non-zero exit that is documented to mean
/// "a file did not parse", so the distinction has to survive in the type
/// rather than in a message prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// `metadata()` on the path failed.
    Metadata(io::ErrorKind),
    /// The file could not be opened or read.
    Read(io::ErrorKind),
    /// The parser rejected the content, or a checker bound was exceeded.
    Parse,
}

/// A file that failed during a checker run.
#[derive(Debug, Clone)]
pub struct FailedFile {
    pub path: PathBuf,
    pub kind: FailureKind,
    pub message: String,
}

/// Outcome of a checker run.
#[derive(Debug, Default)]
pub struct CheckerReport {
    /// Files that were actually read *and* handed to the parser. A file whose
    /// metadata or read failed was not scanned in any sense and is not
    /// counted here.
    pub files_scanned: usize,
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
