//! The checking engine: discover candidates, read them under a hard byte
//! cap, hand the bytes to a parser, and record what happened.

use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;

use ops_core::bounded_read::{
    Rejected, SkipReason, read_candidate, record_failure, relative_to, report_walk_errors,
};

use crate::error::CheckError;
use crate::options::CheckerOptions;
use crate::report::{CheckerReport, FailedFile, FailureKind};
use crate::{json, yaml};

/// Validate every `*.json` file under `opts.root`.
///
/// The returned [`CheckerReport`] is `#[must_use]` on the type, so a
/// discarded report warns even after `?` unwraps the `Result`.
///
/// # Errors
/// Propagates discovery failures and writer I/O errors with the checker
/// label and root attached for diagnosis.
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
/// The returned [`CheckerReport`] is `#[must_use]` on the type, so a
/// discarded report warns even after `?` unwraps the `Result`.
///
/// # Errors
/// Propagates discovery failures and writer I/O errors with the checker
/// label and root attached for diagnosis.
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

/// What happened to one candidate file.
enum Outcome {
    /// Read in full and accepted by the parser.
    Passed,
    /// Deliberately not examined.
    Skipped(SkipReason),
    /// The stat, open or read failed, so the parser never saw it.
    Unreadable(FailureKind, String),
    /// Read in full and rejected by the parser, with the parser's message.
    Invalid(String),
}

/// Validate every candidate under `opts.root` whose extension `ext_ok`
/// accepts, using `check` as the parser.
///
/// A per-file failure is rendered on `writer` and recorded in the report; it
/// does not abort the run.
///
/// # Errors
///
/// If discovery fails, or if `writer` fails.
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
    let mut report = CheckerReport::default();
    let candidates = discover_candidates(opts, writer, label, &mut report)?;

    // The counters below tally entries of `candidates`, an in-memory `Vec`
    // produced by one discovery walk, so their totals are bounded by its
    // length and the `saturating_add` guards can never actually saturate.
    for path in candidates {
        if !ext_ok(path.extension()) {
            continue;
        }
        let display = relative_to(&path, &opts.root);
        let (kind, message) = match check_one(&path, opts.max_bytes, &check) {
            Outcome::Passed => {
                report.files_scanned = report.files_scanned.saturating_add(1);
                continue;
            }
            Outcome::Skipped(reason) => {
                report.files_skipped = report.files_skipped.saturating_add(1);
                writeln!(writer, "{label}: {}: skipped ({reason})", display.display())
                    .with_context(|| format!("{label}: writing skip notice failed"))?;
                continue;
            }
            Outcome::Unreadable(kind, message) => (kind, message),
            Outcome::Invalid(message) => {
                report.files_scanned = report.files_scanned.saturating_add(1);
                (FailureKind::Parse, message)
            }
        };
        let failure = FailedFile {
            path: display,
            kind,
            message,
        };
        record_failure(&mut report, writer, label, failure)?;
    }

    Ok(report)
}

/// Discover the candidate files under `opts.root`, reporting what the
/// discovery itself has to say.
///
/// A `--tracked` request that could not be honoured is announced on `writer`,
/// because the candidate set widened from the git index to every non-ignored
/// file. Traversal errors are printed and adopted into `report`; see
/// [`report_walk_errors`] for why an untraversable directory must fail the
/// run rather than only print.
fn discover_candidates(
    opts: &CheckerOptions,
    writer: &mut dyn Write,
    label: &str,
    report: &mut CheckerReport,
) -> anyhow::Result<Vec<PathBuf>> {
    let discovered = ops_text_fixers::discovery::discover(&opts.root, opts.tracked_only)
        .with_context(|| {
            format!(
                "{label}: file discovery failed for root {} (tracked_only={})",
                opts.root.display(),
                opts.tracked_only
            )
        })?;
    if let Some(fallback) = discovered.fallback {
        writeln!(
            writer,
            "{label}: --tracked unavailable ({fallback}); falling back to a full walk of {}",
            opts.root.display()
        )
        .with_context(|| format!("{label}: writing the discovery fallback notice failed"))?;
    }
    report_walk_errors(report, writer, label, discovered.walk_errors)?;
    Ok(discovered.files)
}

/// Read `path` under the byte cap and hand its bytes to `check`.
///
/// The read is [`read_candidate`], shared with the text fixers: a symlink is
/// judged by itself, never by what it points at, and the cap is enforced by
/// the read.
fn check_one<C>(path: &Path, max_bytes: u64, check: &C) -> Outcome
where
    C: Fn(&[u8]) -> Result<(), CheckError>,
{
    match read_candidate(path, max_bytes) {
        Err(Rejected::Skipped(reason)) => Outcome::Skipped(reason),
        Err(Rejected::Failed(kind, message)) => Outcome::Unreadable(kind, message),
        Ok((bytes, _metadata)) => match check(&bytes) {
            Ok(()) => Outcome::Passed,
            Err(err) => Outcome::Invalid(err.to_string()),
        },
    }
}
