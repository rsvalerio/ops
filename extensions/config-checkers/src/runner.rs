//! The checking engine: discover candidates, read them under a hard byte
//! cap, hand the bytes to a parser, and record what happened.

use std::ffi::OsStr;
use std::io::Write;

use anyhow::Context;

use ops_core::bounded_read::{
    read_candidate, record_failure, relative_to, report_walk_errors, Rejected,
};

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
    let mut discovered = ops_text_fixers::discovery::discover(&opts.root, opts.tracked_only)
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
    // DUP-2 / TASK-2162: the walk-error accounting loop is shared with the
    // text fixers; see `ops_core::bounded_read::report_walk_errors` for why
    // an untraversable directory must fail the run, not just print.
    report_walk_errors(
        &mut report,
        writer,
        label,
        std::mem::take(&mut discovered.walk_errors),
    )?;

    // The counters below tally entries of `discovered.files`, an in-memory
    // `Vec` produced by one discovery walk, so their totals are bounded by its
    // length and the `saturating_add` guards can never actually saturate.
    for path in discovered.files {
        if !ext_ok(path.extension()) {
            continue;
        }
        let display = relative_to(&path, &opts.root);
        // DUP-2 / TASK-2162: the bounded read pipeline is shared with the
        // text fixers. Sharing it is what picked up the symlink hardening:
        // `symlink_metadata` in the shared `open` means a symlink is judged
        // by itself here too, not by whatever it points at — the divergence
        // this dedup existed to close.
        match read_candidate(&path, opts.max_bytes) {
            Err(Rejected::Skipped(reason)) => {
                report.files_skipped = report.files_skipped.saturating_add(1);
                writeln!(writer, "{label}: {}: skipped ({reason})", display.display())
                    .with_context(|| format!("{label}: writing skip notice failed"))?;
            }
            Err(Rejected::Failed(kind, message)) => {
                let failure = FailedFile {
                    path: display,
                    kind,
                    message,
                };
                record_failure(&mut report, writer, label, failure)?;
            }
            Ok((bytes, _metadata)) => {
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
