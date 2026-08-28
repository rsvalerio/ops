//! What a fixer run produces: per-file outcomes and the summary line.

use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;

/// Why a discovered file was deliberately not fixed.
///
/// A skip is a decision, not a malfunction: the file was reachable and the
/// fixer chose to leave it alone. Contrast [`FailureKind`], which means the
/// fixer wanted to look and could not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// Over [`crate::FixerOptions::max_bytes`].
    TooLarge { len: u64, cap: u64 },
    /// A directory, device, FIFO, socket or symlink: never something to
    /// rewrite, and reading one can block forever or never reach EOF.
    NotRegularFile,
    /// Listed by discovery but absent when the fixer reached it — a staged
    /// deletion under `--tracked`, a sparse checkout, or a plain race.
    Vanished,
    /// Not text: contains a NUL byte, or is not valid UTF-8. See
    /// [`crate::binary::is_text`].
    NotText,
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { len, cap } => write!(f, "size {len} exceeds cap {cap}"),
            Self::NotRegularFile => f.write_str("not a regular file"),
            Self::Vanished => f.write_str("not present in the worktree"),
            Self::NotText => f.write_str("not text"),
        }
    }
}

/// Why the fixer could not complete a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// `symlink_metadata` or `File::metadata` failed.
    Metadata(io::ErrorKind),
    /// The file could not be opened or read.
    Read(io::ErrorKind),
    /// The fix was computed but could not be written back.
    Write(io::ErrorKind),
}

/// A file the fixer could not complete.
#[derive(Debug, Clone)]
pub struct FailedFile {
    /// Relative to the run's root where possible.
    pub path: PathBuf,
    pub kind: FailureKind,
    pub message: String,
}

/// Outcome of a fixer run.
#[derive(Debug, Default)]
pub struct FixerReport {
    /// Files read in full and examined as text. A file that was skipped or
    /// failed was not examined in any sense and is not counted here.
    pub files_scanned: usize,
    /// Files rewritten, relative to the run's root where possible.
    pub files_changed: Vec<PathBuf>,
    /// Files deliberately left alone; see [`SkipReason`].
    pub files_skipped: usize,
    /// Files the fixer could not read or could not write back.
    pub files_failed: Vec<FailedFile>,
}

impl FixerReport {
    /// Whether at least one file was rewritten.
    #[must_use]
    pub const fn changed(&self) -> bool {
        !self.files_changed.is_empty()
    }

    /// Whether at least one file could not be checked or could not be written.
    ///
    /// Separate from [`changed`](Self::changed) because the two mean different
    /// things to a hook driver even though both produce a non-zero exit:
    /// "I fixed something, re-stage it" versus "I could not look".
    #[must_use]
    pub const fn failed(&self) -> bool {
        !self.files_failed.is_empty()
    }
}

/// One-line summary for the CLI.
///
/// `scanned + skipped + failed` accounts for every path discovery returned, so
/// a file can never vanish from the summary the way a silently-skipped
/// unreadable file used to.
///
/// # Errors
///
/// Propagates writer I/O errors so a broken pipe in CI is not silently hidden
/// from the caller.
pub fn write_summary(report: &FixerReport, label: &str, writer: &mut dyn Write) -> io::Result<()> {
    writeln!(
        writer,
        "{label}: scanned {} file(s), {} changed, {} skipped, {} failed",
        report.files_scanned,
        report.files_changed.len(),
        report.files_skipped,
        report.files_failed.len(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_summary_renders_all_four_counters() {
        let report = FixerReport {
            files_scanned: 7,
            files_changed: vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")],
            files_skipped: 3,
            files_failed: vec![FailedFile {
                path: PathBuf::from("c.txt"),
                kind: FailureKind::Read(io::ErrorKind::PermissionDenied),
                message: "read: permission denied".to_owned(),
            }],
        };
        let mut buf = Vec::new();
        write_summary(&report, "trailing-whitespace", &mut buf).unwrap();
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "trailing-whitespace: scanned 7 file(s), 2 changed, 3 skipped, 1 failed\n"
        );
    }

    #[test]
    fn write_summary_of_a_clean_run() {
        let mut buf = Vec::new();
        write_summary(&FixerReport::default(), "end-of-file-fixer", &mut buf).unwrap();
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "end-of-file-fixer: scanned 0 file(s), 0 changed, 0 skipped, 0 failed\n"
        );
    }

    #[test]
    fn changed_and_failed_are_independent() {
        let mut report = FixerReport::default();
        assert!(!report.changed());
        assert!(!report.failed());

        report.files_failed.push(FailedFile {
            path: PathBuf::from("a.txt"),
            kind: FailureKind::Write(io::ErrorKind::PermissionDenied),
            message: "write: permission denied".to_owned(),
        });
        assert!(!report.changed(), "a failure is not a change");
        assert!(report.failed());
    }
}
