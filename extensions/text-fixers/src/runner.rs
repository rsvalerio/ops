//! The fixing engine: discover candidates, read each one under a hard byte
//! cap, apply the fix, and write it back atomically.

use std::io::Write;
use std::path::Path;

use anyhow::Context;

use ops_core::bounded_read::{
    read_candidate, record_failure, relative_to, report_walk_errors, Rejected,
};

use crate::options::FixerOptions;
use crate::report::{FailedFile, FailureKind, FixerReport, SkipReason};
use crate::{atomic, binary, discovery, eof, trailing};

/// Strip trailing whitespace from every text file under `opts.root`.
///
/// # Errors
///
/// If the candidate file set cannot be discovered (including a failing
/// `git ls-files` when `tracked_only` is set), or if the writer fails.
/// Per-file read and write failures are recorded in the report, not returned;
/// see [`run_fixer`].
#[must_use = "the FixerReport drives the process exit code; ignoring it defeats the fixer"]
pub fn run_trailing_whitespace(
    opts: &FixerOptions,
    writer: &mut dyn Write,
) -> anyhow::Result<FixerReport> {
    run_fixer(opts, writer, "trailing-whitespace", trailing::fix_trailing)
}

/// Ensure every text file under `opts.root` ends with exactly one newline.
///
/// # Errors
///
/// If the candidate file set cannot be discovered (including a failing
/// `git ls-files` when `tracked_only` is set), or if the writer fails.
/// Per-file read and write failures are recorded in the report, not returned;
/// see [`run_fixer`].
#[must_use = "the FixerReport drives the process exit code; ignoring it defeats the fixer"]
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
/// This is a deliberate policy, and it is the opposite of what the `?` on the
/// old `fs::write` did. A batch rewriter that dies on the first unwritable
/// file (a read-only fixture, a root-owned file, a read-only mount) leaves the
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

    if let Some(fallback) = discovered.fallback {
        // The user asked for the git index and is getting the filesystem
        // instead, which puts untracked files under a tool that rewrites in
        // place. Never silent.
        writeln!(
            writer,
            "{label}: --tracked unavailable ({fallback}); falling back to a full walk of {} — \
             untracked files are candidates too",
            opts.root.display()
        )
        .with_context(|| format!("{label}: writing the discovery fallback notice failed"))?;
    }
    let mut report = FixerReport::default();
    // DUP-2 / TASK-2162: the walk-error accounting loop is shared with the
    // config checkers; see `ops_core::bounded_read::report_walk_errors` for
    // why an untraversable directory must fail the run, not just print.
    report_walk_errors(
        &mut report,
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

    // Every counter below tallies entries of `discovered.files`, an in-memory
    // `Vec` from one discovery pass, so the totals are bounded by its length
    // and the `saturating_add` guards can never actually saturate.
    for path in discovered.files {
        let display = relative_to(&path, &opts.root);
        // DUP-2 / TASK-2162: the bounded read pipeline is shared with the
        // config checkers; one implementation in `ops_core::bounded_read`,
        // so its symlink/type guards and read ceiling cannot diverge again.
        let (bytes, metadata) = match read_candidate(&path, opts.max_bytes) {
            Ok(candidate) => candidate,
            Err(Rejected::Skipped(reason)) => {
                report.files_skipped = report.files_skipped.saturating_add(1);
                write_skip(writer, label, &display, &reason)?;
                continue;
            }
            Err(Rejected::Failed(kind, message)) => {
                record_failure(
                    &mut report,
                    writer,
                    label,
                    FailedFile {
                        path: display,
                        kind,
                        message,
                    },
                )?;
                continue;
            }
        };

        if !binary::is_text(&bytes) {
            report.files_skipped = report.files_skipped.saturating_add(1);
            write_skip(writer, label, &display, &SkipReason::NotText)?;
            continue;
        }
        let Some(fixed) = fix(&bytes) else {
            report.files_scanned = report.files_scanned.saturating_add(1);
            continue;
        };
        if fixed == bytes {
            report.files_scanned = report.files_scanned.saturating_add(1);
            continue;
        }

        // The scanned tally is deliberately deferred past this point: a file
        // whose rewrite fails is recorded in `files_failed`, and counting it
        // as scanned too would put one discovered file in two buckets and
        // make `scanned + failed + skipped` overshoot the discovered total.
        if let Err(e) = atomic::replace(&path, &fixed, &metadata) {
            record_failure(
                &mut report,
                writer,
                label,
                FailedFile {
                    path: display,
                    kind: FailureKind::Write(e.kind()),
                    message: format!("write: {e}"),
                },
            )?;
            continue;
        }
        report.files_scanned = report.files_scanned.saturating_add(1);
        writeln!(writer, "{label}: fixed {}", display.display())
            .with_context(|| format!("{label}: writing the fixed-file line failed"))?;
        report.files_changed.push(display);
    }

    Ok(report)
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
