//! The fixing engine: discover candidates, read each one under a hard byte
//! cap, apply the fix, and write it back atomically.

use std::fs::Metadata;
use std::io::Write;
use std::path::Path;

use anyhow::Context;

use ops_core::bounded_read::{
    Rejected, read_candidate, record_failure, relative_to, report_walk_errors,
};

use crate::options::FixerOptions;
use crate::report::{FailedFile, FailureKind, FixerReport, SkipReason};
use crate::{atomic, binary, discovery, eof, trailing};

/// Strip trailing whitespace from every text file under `opts.root`.
///
/// [`FixerReport`] carries `#[must_use]` on the type itself, so a discarded
/// report warns even after `?` has unwrapped the `Result`.
///
/// # Errors
///
/// If the candidate file set cannot be discovered (including a failing
/// `git ls-files` when `tracked_only` is set), or if the writer fails.
/// Per-file read and write failures are recorded in the report, not returned;
/// see [`run_fixer`].
pub fn run_trailing_whitespace(
    opts: &FixerOptions,
    writer: &mut dyn Write,
) -> anyhow::Result<FixerReport> {
    run_fixer(opts, writer, "trailing-whitespace", trailing::fix_trailing)
}

/// Ensure every text file under `opts.root` ends with exactly one newline.
///
/// [`FixerReport`] carries `#[must_use]` on the type itself, so a discarded
/// report warns even after `?` has unwrapped the `Result`.
///
/// # Errors
///
/// If the candidate file set cannot be discovered (including a failing
/// `git ls-files` when `tracked_only` is set), or if the writer fails.
/// Per-file read and write failures are recorded in the report, not returned;
/// see [`run_fixer`].
pub fn run_end_of_file_fixer(
    opts: &FixerOptions,
    writer: &mut dyn Write,
) -> anyhow::Result<FixerReport> {
    run_fixer(opts, writer, "end-of-file-fixer", eof::fix_eof)
}

/// Apply `fix` to every candidate file under `opts.root`.
///
/// # Per-file failures do not abort the run
///
/// This is a deliberate policy. A batch rewriter that dies on the first
/// unwritable file (a read-only fixture, a root-owned file, a read-only
/// mount) leaves the
/// user with a half-fixed tree **and no record of what it already changed**,
/// because the report is dropped by the propagating error. Instead each read
/// or write failure is rendered on `writer` with the offending path and
/// recorded in [`FixerReport::files_failed`]; the run continues, and the
/// caller still receives every file that *was* fixed. A run with failures is
/// not a passing run — [`FixerReport::failed`] is what the CLI maps onto a
/// non-zero exit.
///
/// Only two things still abort: discovery failing outright (there is no
/// candidate set to work from) and the writer failing (a report nobody can
/// read is worse than an error).
///
/// # Errors
///
/// If discovery fails, or if `writer` fails.
fn run_fixer(
    opts: &FixerOptions,
    writer: &mut dyn Write,
    label: &str,
    fix: fn(&[u8]) -> Option<Vec<u8>>,
) -> anyhow::Result<FixerReport> {
    let mut discovered = discovery::discover(&opts.root, opts.tracked_only).with_context(|| {
        format!(
            "{label}: file discovery failed for root {} (tracked_only={})",
            opts.root.display(),
            opts.tracked_only
        )
    })?;

    let mut report = FixerReport {
        check_only: opts.check,
        ..FixerReport::default()
    };
    report_discovery_issues(&mut report, writer, label, &opts.root, &mut discovered)?;

    let mut ctx = RunContext {
        label,
        check: opts.check,
        root: &opts.root,
        writer,
    };
    // Every counter below tallies entries of `discovered.files`, an in-memory
    // `Vec` from one discovery pass, so the totals are bounded by its length
    // and the `saturating_add` guards can never actually saturate.
    for path in discovered.files {
        let outcome = classify_candidate(opts.max_bytes, fix, &path);
        record_outcome(&mut ctx, &mut report, &path, outcome)?;
    }

    Ok(report)
}

/// Everything the per-file stage needs to record one outcome: the run's label
/// and root for rendering paths, the check-mode flag that decides write-back,
/// and the writer the lines go to.
struct RunContext<'a> {
    label: &'a str,
    check: bool,
    root: &'a Path,
    writer: &'a mut dyn Write,
}

/// What the examining stage found for one discovered file.
///
/// The variants are the per-file accounting contract: each maps to exactly
/// one counter — `Unchanged` and a completed [`FileOutcome::NeedsWrite`] to
/// `files_scanned`, [`FileOutcome::Skipped`] to `files_skipped`,
/// [`FileOutcome::Failed`] to `files_failed` — so `scanned + skipped +
/// failed` accounts for every discovered path exactly once.
enum FileOutcome {
    /// Read and examined, but no rewrite is warranted: the fixer found
    /// nothing to change, or the fix produced identical bytes.
    Unchanged,
    /// Deliberately not examined; see [`SkipReason`].
    Skipped(SkipReason),
    /// Could not be read, with the failure kind and message to record.
    Failed { kind: FailureKind, message: String },
    /// The fix produced new bytes that must be written back — or, in check
    /// mode, reported without writing.
    NeedsWrite { fixed: Vec<u8>, metadata: Metadata },
}

/// Render the run-level discovery notices and fold the walk errors into the
/// report: the tracked-mode fallback warning, the untraversable directories,
/// then the undecodable-path count, in that order.
///
/// The walk-error accounting loop is shared with the config checkers; see
/// `ops_core::bounded_read::report_walk_errors` for why an untraversable
/// directory must fail the run, not just print.
///
/// # Errors
///
/// If `writer` fails while rendering a notice.
fn report_discovery_issues(
    report: &mut FixerReport,
    writer: &mut dyn Write,
    label: &str,
    root: &Path,
    discovered: &mut discovery::Discovery,
) -> anyhow::Result<()> {
    if let Some(fallback) = &discovered.fallback {
        // The user asked for the git index and is getting the filesystem
        // instead, which puts untracked files under a tool that rewrites in
        // place. Never silent.
        writeln!(
            writer,
            "{label}: --tracked unavailable ({fallback}); falling back to a full walk of {} — \
             untracked files are candidates too",
            root.display()
        )
        .with_context(|| format!("{label}: writing the discovery fallback notice failed"))?;
    }
    report_walk_errors(
        report,
        writer,
        label,
        std::mem::take(&mut discovered.walk_errors),
    )?;
    if discovered.undecodable_paths > 0 {
        writeln!(
            writer,
            "{label}: {} tracked path(s) skipped: filename is not valid UTF-8 on this platform",
            discovered.undecodable_paths
        )
        .with_context(|| format!("{label}: writing the undecodable-path notice failed"))?;
    }
    Ok(())
}

/// Read one discovered file and classify it, without touching the report or
/// the writer.
///
/// The bounded read pipeline is shared with the config checkers; one
/// implementation in `ops_core::bounded_read`, so its symlink/type guards and
/// read ceiling stay in one place.
fn classify_candidate(
    max_bytes: u64,
    fix: fn(&[u8]) -> Option<Vec<u8>>,
    path: &Path,
) -> FileOutcome {
    let (bytes, metadata) = match read_candidate(path, max_bytes) {
        Ok(candidate) => candidate,
        Err(Rejected::Skipped(reason)) => return FileOutcome::Skipped(reason),
        Err(Rejected::Failed(kind, message)) => return FileOutcome::Failed { kind, message },
    };
    if !binary::is_text(&bytes) {
        return FileOutcome::Skipped(SkipReason::NotText);
    }
    match fix(&bytes) {
        Some(fixed) if fixed != bytes => FileOutcome::NeedsWrite { fixed, metadata },
        _ => FileOutcome::Unchanged,
    }
}

/// Record one classified outcome: tally the counter it belongs to and render
/// its line.
///
/// # Errors
///
/// If `writer` fails while rendering a line.
fn record_outcome(
    ctx: &mut RunContext<'_>,
    report: &mut FixerReport,
    path: &Path,
    outcome: FileOutcome,
) -> anyhow::Result<()> {
    match outcome {
        FileOutcome::Unchanged => {
            report.files_scanned = report.files_scanned.saturating_add(1);
        }
        FileOutcome::Skipped(reason) => {
            report.files_skipped = report.files_skipped.saturating_add(1);
            let display = relative_to(path, ctx.root);
            write_skip(ctx.writer, ctx.label, &display, &reason)?;
        }
        FileOutcome::Failed { kind, message } => {
            let display = relative_to(path, ctx.root);
            let failure = FailedFile {
                path: display,
                kind,
                message,
            };
            record_failure(report, ctx.writer, ctx.label, failure)?;
        }
        FileOutcome::NeedsWrite { fixed, metadata } => {
            record_rewrite(ctx, report, path, &fixed, &metadata)?;
        }
    }
    Ok(())
}

/// Write back a file the fix changed — or, in check mode, report it without
/// writing — and tally the outcome.
///
/// # Errors
///
/// If `writer` fails while rendering a line.
fn record_rewrite(
    ctx: &mut RunContext<'_>,
    report: &mut FixerReport,
    path: &Path,
    fixed: &[u8],
    metadata: &Metadata,
) -> anyhow::Result<()> {
    let display = relative_to(path, ctx.root);
    let label = ctx.label;
    if ctx.check {
        // Check mode: the file needs fixing, which is the finding. It is
        // recorded exactly like a rewrite so the exit code is the same,
        // but the tree is never touched.
        report.files_scanned = report.files_scanned.saturating_add(1);
        writeln!(ctx.writer, "{label}: would fix {}", display.display())
            .with_context(|| format!("{label}: writing the would-fix line failed"))?;
        report.files_changed.push(display);
        return Ok(());
    }
    // The scanned tally is deliberately deferred past the write: a file
    // whose rewrite fails is recorded in `files_failed`, and counting it
    // as scanned too would put one discovered file in two buckets and
    // make `scanned + failed + skipped` overshoot the discovered total.
    if let Err(e) = atomic::replace(path, fixed, metadata) {
        record_failure(
            report,
            ctx.writer,
            label,
            FailedFile {
                path: display,
                kind: FailureKind::Write(e.kind()),
                message: format!("write: {e}"),
            },
        )?;
        return Ok(());
    }
    report.files_scanned = report.files_scanned.saturating_add(1);
    writeln!(ctx.writer, "{label}: fixed {}", display.display())
        .with_context(|| format!("{label}: writing the fixed-file line failed"))?;
    report.files_changed.push(display);
    Ok(())
}

/// Render a per-file skip line, except for the one skip that is routine.
///
/// Unlike the config checkers, which filter by extension first, every file in
/// the tree is a candidate here — so a repository with a few hundred images,
/// fonts and archives would drown the run in `skipped (not text)` lines and
/// bury the skips that actually mean something. `NotText` is therefore counted
/// in [`FixerReport::files_skipped`] and reported in the summary, but not
/// listed file by file. The unusual skips — over the cap, not a regular file,
/// gone from the worktree — are each named, because each is a file the user
/// might have expected to be checked.
fn write_skip(
    writer: &mut dyn Write,
    label: &str,
    display: &Path,
    reason: &SkipReason,
) -> anyhow::Result<()> {
    if matches!(reason, SkipReason::NotText) {
        return Ok(());
    }
    writeln!(writer, "{label}: {}: skipped ({reason})", display.display())
        .with_context(|| format!("{label}: writing skip notice failed"))
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::PathBuf;

    use super::*;

    thread_local! {
        /// The file [`edit_then_fix`] overwrites, standing in for whoever
        /// edits a candidate while the fixer is between read and write-back.
        static VICTIM: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    }

    const CONCURRENT_EDIT: &[u8] = b"saved by an editor mid-run\n";

    /// A fix that changes the file on disk before returning its rewrite: the
    /// exact interleaving of a concurrent edit landing after the read.
    #[expect(
        clippy::unnecessary_wraps,
        reason = "the signature is the one `run_fixer` takes for `fix`"
    )]
    fn edit_then_fix(_input: &[u8]) -> Option<Vec<u8>> {
        VICTIM.with_borrow(|victim| {
            let path = victim.as_ref().expect("the test sets the victim path");
            std::fs::write(path, CONCURRENT_EDIT).expect("the concurrent edit lands");
        });
        Some(b"rewritten from stale bytes\n".to_vec())
    }

    #[test]
    fn a_file_edited_between_read_and_write_back_is_reported_and_kept() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let path = root.join("a.txt");
        std::fs::write(&path, b"original  \n").unwrap();
        VICTIM.set(Some(path.clone()));

        let mut out = Vec::new();
        let opts = FixerOptions::new(root, false);
        let report = run_fixer(&opts, &mut out, "test-fixer", edit_then_fix).unwrap();

        assert_eq!(
            std::fs::read(&path).unwrap(),
            CONCURRENT_EDIT,
            "the concurrent edit must survive the run"
        );
        assert!(report.failed());
        assert!(report.files_changed.is_empty());
        let [failure] = report.files_failed.as_slice() else {
            panic!("expected one failure, got {:?}", report.files_failed);
        };
        assert_eq!(failure.path, PathBuf::from("a.txt"));
        assert!(matches!(failure.kind, FailureKind::Write(_)));
        let rendered = String::from_utf8(out).unwrap();
        assert!(
            rendered.contains(&format!("a.txt: write: {}", atomic::CHANGED_SINCE_READ)),
            "the refusal must be named in the output: {rendered}"
        );
    }
}
