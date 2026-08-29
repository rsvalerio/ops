//! The fixing engine: discover candidates, read each one under a hard byte
//! cap, apply the fix, and write it back atomically.

use std::fs::{File, Metadata};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;

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
    let discovered = discovery::discover(&opts.root, opts.tracked_only).with_context(|| {
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
    for error in &discovered.walk_errors {
        // An entry the walk could not traverse hides an unknown number of
        // files, and a gate that reports "clean" over them is fail-open.
        writeln!(writer, "{label}: walk error: {error}")
            .with_context(|| format!("{label}: writing the walk-error notice failed"))?;
    }
    if discovered.undecodable_paths > 0 {
        writeln!(
            writer,
            "{label}: {} tracked path(s) skipped: filename is not valid UTF-8 on this platform",
            discovered.undecodable_paths
        )
        .with_context(|| format!("{label}: writing the undecodable-path notice failed"))?;
    }

    let mut report = FixerReport::default();
    // Every counter below tallies entries of `discovered.files`, an in-memory
    // `Vec` from one discovery pass, so the totals are bounded by its length
    // and the `saturating_add` guards can never actually saturate.
    for path in discovered.files {
        let display = relative_to(&path, &opts.root);
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

/// Emit one failure line and record it, so the failure sources cannot drift
/// apart in either wording or bookkeeping.
///
/// The path is on the line and in the record. That is the whole point: the
/// predecessor propagated a bare `io::Error`, so a repository-wide run could
/// fail with `Permission denied (os error 13)` and nothing anywhere naming
/// which of thousands of files it meant.
fn record_failure(
    report: &mut FixerReport,
    writer: &mut dyn Write,
    label: &str,
    failure: FailedFile,
) -> anyhow::Result<()> {
    writeln!(
        writer,
        "{label}: {}: {}",
        failure.path.display(),
        failure.message
    )
    .with_context(|| format!("{label}: writing failure line failed"))?;
    report.files_failed.push(failure);
    Ok(())
}

/// Why a path did not become bytes to fix.
enum Rejected {
    /// Deliberately not fixed.
    Skipped(SkipReason),
    /// I/O failure, with the line to render for it.
    Failed(FailureKind, String),
}

/// Bytes read in full within the cap, with the metadata of the handle they
/// came from — the same metadata the rewrite restores onto the new inode.
type Content = (Vec<u8>, Metadata);

/// Read one candidate file under a hard byte ceiling.
///
/// The cap is a property of the *read* (`Read::take`), not of a preceding
/// `metadata()` call: `metadata()` describes a path at one instant and the
/// file it described can grow before the read, so a stat-then-read pair
/// bounds nothing. Opening once and measuring the handle also removes the
/// window in which the path could be swapped between the type check and the
/// read.
fn read_candidate(path: &Path, max_bytes: u64) -> Result<Content, Rejected> {
    let (file, metadata) = open_regular_file(path, max_bytes)?;
    read_bounded(file, metadata, max_bytes)
}

/// Open `path` if — and only if — it is a regular file within the cap.
fn open_regular_file(path: &Path, max_bytes: u64) -> Result<(File, Metadata), Rejected> {
    // Type guard *before* `File::open`, and unavoidably by path: opening a
    // FIFO blocks in `open(2)` until a writer appears, so a symlink to one
    // would hang the fixer before any handle-based check could run.
    // `symlink_metadata` rather than `metadata`, so a symlink is rejected as
    // itself instead of being judged by its target. It authorises nothing —
    // the checks that gate the read are taken from the handle below.
    match std::fs::symlink_metadata(path) {
        Ok(md) if !md.file_type().is_file() => {
            return Err(Rejected::Skipped(SkipReason::NotRegularFile))
        }
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return Err(Rejected::Skipped(SkipReason::Vanished))
        }
        Err(e) => return Err(metadata_failure(&e)),
    }

    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return Err(Rejected::Skipped(SkipReason::Vanished))
        }
        Err(e) => return Err(read_failure(&e)),
    };
    // From the handle, so it describes the file that will actually be read
    // rather than whatever the path resolves to on a second lookup.
    let md = match file.metadata() {
        Ok(md) => md,
        Err(e) => return Err(metadata_failure(&e)),
    };
    if !md.is_file() {
        return Err(Rejected::Skipped(SkipReason::NotRegularFile));
    }
    if md.len() > max_bytes {
        return Err(Rejected::Skipped(SkipReason::TooLarge {
            len: md.len(),
            cap: max_bytes,
        }));
    }
    Ok((file, md))
}

/// Read `file` with the cap enforced by the reader itself.
fn read_bounded(file: File, metadata: Metadata, max_bytes: u64) -> Result<Content, Rejected> {
    // `max_bytes + 1` rather than `max_bytes`: reading one byte past the cap
    // is what makes an over-cap file *detectable* instead of silently
    // truncated and then rewritten as if it were the whole file — which for a
    // fixer that writes its input back would destroy everything past the cap.
    let ceiling = max_bytes.saturating_add(1);
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len().min(max_bytes)).unwrap_or(0));
    if let Err(e) = file.take(ceiling).read_to_end(&mut bytes) {
        return Err(read_failure(&e));
    }
    // The stat above was a snapshot; the file may have grown since. This is
    // the check that holds, because it measures what was read.
    let read = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if read > max_bytes {
        return Err(Rejected::Skipped(SkipReason::TooLarge {
            len: read,
            cap: max_bytes,
        }));
    }
    Ok((bytes, metadata))
}

fn metadata_failure(e: &std::io::Error) -> Rejected {
    Rejected::Failed(FailureKind::Metadata(e.kind()), format!("metadata: {e}"))
}

fn read_failure(e: &std::io::Error) -> Rejected {
    Rejected::Failed(FailureKind::Read(e.kind()), format!("read: {e}"))
}

fn relative_to(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}
