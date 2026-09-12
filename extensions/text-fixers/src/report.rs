//! What a fixer run produces: per-file outcomes and the summary line.

use std::io::{self, Write};
use std::path::PathBuf;

// The per-file outcome vocabulary — why a candidate was
// skipped, why a file failed — is shared with the config checkers through one
// definition in `ops_core::bounded_read`, so a hardening fix applied there
// reaches both file-walking extensions at once.
pub use ops_core::bounded_read::{FailedFile, FailureKind, SkipReason};

/// Outcome of a fixer run.
///
/// The `#[must_use]` sits on the *type*, not on the
/// `run_*` functions, so it survives `?` — discarding the report after
/// unwrapping the `Result` is still a warning, because the report (via
/// [`FixerReport::changed`] and [`FixerReport::failed`]) is what drives the
/// process exit code.
#[must_use = "the report drives the process exit code; dropping it after `?` exits 0 on a dirty tree"]
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
    /// Directories the discovery walk could not traverse. Each one hides an
    /// unknown number of candidates, so a run that carries any of these did
    /// not see the whole tree and must not report "clean" — see
    /// [`FixerReport::failed`].
    pub walk_errors: Vec<String>,
}

impl ops_core::bounded_read::FileRunReport for FixerReport {
    fn push_failure(&mut self, failure: FailedFile) {
        self.files_failed.push(failure);
    }

    fn adopt_walk_errors(&mut self, errors: Vec<String>) {
        self.walk_errors = errors;
    }
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
    ///
    /// A walk error counts: the fixer completed every candidate it was given,
    /// but traversal silently omitted candidates it never learned about.
    /// Exiting 0 there is fail-open — the CLI would report a clean tree over
    /// directories it could not read.
    #[must_use]
    pub const fn failed(&self) -> bool {
        !self.files_failed.is_empty() || !self.walk_errors.is_empty()
    }
}

/// One-line summary for the CLI.
///
/// `scanned + skipped + failed` accounts for every path discovery returned, so
/// no file — an unreadable one included — can vanish from the summary.
///
/// # Errors
///
/// Propagates writer I/O errors so a broken pipe in CI is not silently hidden
/// from the caller.
pub fn write_summary(report: &FixerReport, label: &str, writer: &mut dyn Write) -> io::Result<()> {
    writeln!(
        writer,
        "{label}: scanned {} file(s), {} changed, {} skipped, {} failed, {} walk error(s)",
        report.files_scanned,
        report.files_changed.len(),
        report.files_skipped,
        report.files_failed.len(),
        report.walk_errors.len(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_summary_renders_every_counter() {
        let report = FixerReport {
            files_scanned: 7,
            files_changed: vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")],
            files_skipped: 3,
            files_failed: vec![FailedFile {
                path: PathBuf::from("c.txt"),
                kind: FailureKind::Read(io::ErrorKind::PermissionDenied),
                message: "read: permission denied".to_owned(),
            }],
            walk_errors: Vec::new(),
        };
        let mut buf = Vec::new();
        write_summary(&report, "trailing-whitespace", &mut buf).unwrap();
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "trailing-whitespace: scanned 7 file(s), 2 changed, 3 skipped, 1 failed, 0 walk \
             error(s)\n"
        );
    }

    #[test]
    fn write_summary_of_a_clean_run() {
        let mut buf = Vec::new();
        write_summary(&FixerReport::default(), "end-of-file-fixer", &mut buf).unwrap();
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "end-of-file-fixer: scanned 0 file(s), 0 changed, 0 skipped, 0 failed, 0 walk \
             error(s)\n"
        );
    }

    /// A walk error means traversal silently omitted candidates, so the run
    /// cannot honestly report "clean". Printing the error to the writer and
    /// dropping it would leave `failed()` false and exit the CLI 0 over
    /// directories it never read, so the error is carried in the report.
    #[test]
    fn a_walk_error_alone_fails_the_run_and_shows_in_the_summary() {
        let report = FixerReport {
            walk_errors: vec!["IO error for operation on /x: permission denied".to_owned()],
            ..FixerReport::default()
        };
        assert!(
            report.failed(),
            "a traversal that lost candidates must not report success"
        );
        assert!(!report.changed(), "a walk error is not a change");
        assert!(!FixerReport::default().failed());

        let mut buf = Vec::new();
        write_summary(&report, "trailing-whitespace", &mut buf).unwrap();
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "trailing-whitespace: scanned 0 file(s), 0 changed, 0 skipped, 0 failed, 1 walk \
             error(s)\n"
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
