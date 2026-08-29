//! The checking engine: discover candidates, read them under a hard byte
//! cap, hand the bytes to a parser, and record what happened.

use std::ffi::OsStr;
use std::fmt;
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::error::CheckError;
use crate::options::CheckerOptions;
use crate::report::{CheckerReport, FailedFile, FailureKind};
use crate::{json, yaml};

/// Validate every `*.json` file under `opts.root`.
///
/// # Errors
/// Propagates discovery failures and writer I/O errors with the checker
/// label and root attached for diagnosis.
#[must_use = "the CheckerReport drives the process exit code; ignoring it defeats the validator"]
pub fn run_check_json(
    opts: &CheckerOptions,
    writer: &mut dyn Write,
) -> anyhow::Result<CheckerReport> {
    let allow_json5 = opts.allow_json5;
    run_checker(
        opts,
        writer,
        "check-json",
        |ext| matches_ext(ext, &["json"]),
        move |bytes| json::check_json(bytes, allow_json5),
    )
}

/// Validate every `*.yaml` / `*.yml` file under `opts.root`.
///
/// # Errors
/// Propagates discovery failures and writer I/O errors with the checker
/// label and root attached for diagnosis.
#[must_use = "the CheckerReport drives the process exit code; ignoring it defeats the validator"]
pub fn run_check_yaml(
    opts: &CheckerOptions,
    writer: &mut dyn Write,
) -> anyhow::Result<CheckerReport> {
    run_checker(
        opts,
        writer,
        "check-yaml",
        |ext| matches_ext(ext, &["yaml", "yml"]),
        yaml::check_yaml,
    )
}

fn matches_ext(ext: Option<&OsStr>, allowed: &[&str]) -> bool {
    ext.and_then(OsStr::to_str)
        .is_some_and(|e| allowed.iter().any(|a| a.eq_ignore_ascii_case(e)))
}

fn run_checker<E, C>(
    opts: &CheckerOptions,
    writer: &mut dyn Write,
    label: &str,
    ext_ok: E,
    check: C,
) -> anyhow::Result<CheckerReport>
where
    E: Fn(Option<&OsStr>) -> bool,
    C: Fn(&[u8]) -> Result<(), CheckError>,
{
    let discovered = ops_text_fixers::discovery::discover(&opts.root, opts.tracked_only)
        .with_context(|| {
            format!(
                "{label}: file discovery failed for root {} (tracked_only={})",
                opts.root.display(),
                opts.tracked_only
            )
        })?;
    if let Some(fallback) = discovered.fallback {
        // `--tracked` could not be honoured, so the candidate set silently
        // widened from the git index to every non-ignored file. Say so.
        writeln!(
            writer,
            "{label}: --tracked unavailable ({fallback}); falling back to a full walk of {}",
            opts.root.display()
        )
        .with_context(|| format!("{label}: writing the discovery fallback notice failed"))?;
    }
    let mut report = CheckerReport::default();
    for error in &discovered.walk_errors {
        // A directory the walk could not traverse hides an unknown number of
        // candidates; a checker that reports "clean" over them is fail-open.
        // Printing the notice is not enough — the error has to reach the
        // report so `CheckerReport::failed` can drive a non-zero exit.
        writeln!(writer, "{label}: walk error: {error}")
            .with_context(|| format!("{label}: writing the walk-error notice failed"))?;
    }
    report.walk_errors = discovered.walk_errors;

    // The counters below tally entries of `discovered.files`, an in-memory
    // `Vec` produced by one discovery walk, so their totals are bounded by its
    // length and the `saturating_add` guards can never actually saturate.
    for path in discovered.files {
        if !ext_ok(path.extension()) {
            continue;
        }
        let display = relative_to(&path, &opts.root);
        match read_candidate(&path, opts.max_bytes) {
            Candidate::Skipped(reason) => {
                report.files_skipped = report.files_skipped.saturating_add(1);
                writeln!(writer, "{label}: {}: skipped ({reason})", display.display())
                    .with_context(|| format!("{label}: writing skip notice failed"))?;
            }
            Candidate::Failed(kind, message) => {
                let failure = FailedFile {
                    path: display,
                    kind,
                    message,
                };
                record_failure(&mut report, writer, label, failure)?;
            }
            Candidate::Bytes(bytes) => {
                report.files_scanned = report.files_scanned.saturating_add(1);
                if let Err(err) = check(&bytes) {
                    let failure = FailedFile {
                        path: display,
                        kind: FailureKind::Parse,
                        message: err.to_string(),
                    };
                    record_failure(&mut report, writer, label, failure)?;
                }
            }
        }
    }

    Ok(report)
}

/// Emit one failure line and record it, so the three failure sources cannot
/// drift apart in either wording or bookkeeping.
fn record_failure(
    report: &mut CheckerReport,
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

/// Why a candidate was not validated.
enum SkipReason {
    /// Over the configured byte cap.
    TooLarge { len: u64, cap: u64 },
    /// A device, FIFO, socket or directory — never a config file, and
    /// reading one can block forever or never reach EOF.
    NotRegularFile,
    /// Listed by discovery but absent when the checker reached it: an
    /// unstaged deletion or a sparse checkout under `--tracked`, or a plain
    /// race under the walk. Not a parse failure.
    Vanished,
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { len, cap } => write!(f, "size {len} exceeds cap {cap}"),
            Self::NotRegularFile => f.write_str("not a regular file"),
            Self::Vanished => f.write_str("not present in the worktree"),
        }
    }
}

/// What [`read_candidate`] found at a path.
enum Candidate {
    /// Content read in full, within the cap.
    Bytes(Vec<u8>),
    /// Deliberately not validated.
    Skipped(SkipReason),
    /// I/O failure, with the line to render for it.
    Failed(FailureKind, String),
}

/// Read one candidate file under a hard byte ceiling.
///
/// `SEC-25`: the byte cap is a property of the *read*, not of a preceding
/// `metadata()` call. `metadata()` is a snapshot of a path, and the file it
/// described can be replaced or extended before the read; `Read::take` bounds
/// the read itself, so the ceiling holds regardless.
fn read_candidate(path: &Path, max_bytes: u64) -> Candidate {
    let (file, len) = match open_regular_file(path, max_bytes) {
        Ok(opened) => opened,
        Err(candidate) => return candidate,
    };
    read_bounded(file, len, max_bytes)
}

/// Open `path` if — and only if — it is a regular file within the cap.
///
/// Returns the open handle and the size its own metadata reported, or the
/// [`Candidate`] the caller should record instead.
fn open_regular_file(path: &Path, max_bytes: u64) -> Result<(File, u64), Candidate> {
    // Type guard *before* `File::open`. This one is unavoidably by-path:
    // opening a FIFO blocks in `open(2)` until a writer appears, so a tracked
    // symlink to one would hang the checker before any handle-based check
    // could run. It authorises nothing — the checks that actually gate the
    // read are taken from the open handle below.
    match std::fs::metadata(path) {
        Ok(md) if !md.is_file() => return Err(Candidate::Skipped(SkipReason::NotRegularFile)),
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return Err(Candidate::Skipped(SkipReason::Vanished))
        }
        Err(e) => return Err(metadata_failure(&e)),
    }

    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return Err(Candidate::Skipped(SkipReason::Vanished))
        }
        Err(e) => return Err(read_failure(&e)),
    };
    // From the handle, so it describes the file that will actually be read
    // rather than whatever the path resolves to on a second lookup.
    let md = file.metadata().map_err(|e| metadata_failure(&e))?;
    if !md.is_file() {
        return Err(Candidate::Skipped(SkipReason::NotRegularFile));
    }
    if md.len() > max_bytes {
        return Err(Candidate::Skipped(SkipReason::TooLarge {
            len: md.len(),
            cap: max_bytes,
        }));
    }
    Ok((file, md.len()))
}

/// Read `file` with the cap enforced by the reader itself.
fn read_bounded(file: File, len: u64, max_bytes: u64) -> Candidate {
    // `max_bytes + 1` rather than `max_bytes`: reading one byte past the cap
    // is what makes an over-cap file *detectable* instead of silently
    // truncated and then parsed as if it were the whole document.
    let ceiling = max_bytes.saturating_add(1);
    let mut bytes = Vec::with_capacity(usize::try_from(len.min(max_bytes)).unwrap_or(0));
    if let Err(e) = file.take(ceiling).read_to_end(&mut bytes) {
        return read_failure(&e);
    }
    // The stat above was a snapshot; the file may have grown since. This is
    // the check that actually holds, because it measures what was read.
    let read = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if read > max_bytes {
        return Candidate::Skipped(SkipReason::TooLarge {
            len: read,
            cap: max_bytes,
        });
    }
    Candidate::Bytes(bytes)
}

fn metadata_failure(e: &std::io::Error) -> Candidate {
    Candidate::Failed(FailureKind::Metadata(e.kind()), format!("metadata: {e}"))
}

fn read_failure(e: &std::io::Error) -> Candidate {
    Candidate::Failed(FailureKind::Read(e.kind()), format!("read: {e}"))
}

fn relative_to(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}
