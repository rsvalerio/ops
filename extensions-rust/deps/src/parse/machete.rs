//! Parser for `cargo machete` text output.
//!
//! cargo-machete (0.9) has no machine-readable output mode, so this parses
//! its human-readable stdout. The contract, from its source:
//!
//! * stdout carries either one `didn't find any unused dependencies` summary
//!   line, or a `found the following unused dependencies` header followed by
//!   `<package> -- <manifest>:` lines, each with one tab-indented dependency
//!   per line, ended by a blank line and an advice trailer.
//! * stderr carries progress lines, `error when handling <manifest>: …` for a
//!   manifest it could not analyse, and `⚠️ … marked as ignored, but is
//!   actually used!` notes.
//! * exit `0` means nothing unused, `1` means at least one unused dependency,
//!   `2` means an error.
//!
//! Suppressions come from `[package.metadata.cargo-machete] ignored = [...]`,
//! which cargo-machete applies itself; this parser never sees ignored
//! dependencies and does not re-implement the list.

use crate::UnusedDepEntry;
use std::path::Path;
use std::time::Duration;

use super::truncate_for_log;

/// Default timeout for `cargo machete`; overridable via
/// `OPS_SUBPROCESS_TIMEOUT_SECS`. cargo-machete greps sources without
/// compiling, so it takes well under a second on this workspace; the bound is
/// for a wedged filesystem, not for normal runtime.
const CARGO_MACHETE_TIMEOUT: Duration = Duration::from_mins(2);

const CLEAN_MARKER: &str = "cargo-machete didn't find any unused dependencies in ";
const FOUND_MARKER: &str = "cargo-machete found the following unused dependencies in ";
const MANIFEST_ERROR_MARKER: &str = "error when handling ";

/// Run `cargo machete` and parse its output.
///
/// Runs without `--with-metadata`: that flag asks cargo metadata for renamed
/// crates, but on this workspace it took ~10× as long, reported *more* false
/// positives, and cargo-machete warns it may rewrite `Cargo.lock`.
///
/// # Errors
///
/// If `cargo machete` cannot be spawned, exceeds its timeout, or its exit
/// code and output fail the checks in [`interpret_machete_output`].
pub fn run_cargo_machete(working_dir: &Path) -> anyhow::Result<Vec<UnusedDepEntry>> {
    let output = crate::run_cargo_tool(
        &["machete"],
        crate::CARGO_MACHETE.strip_env,
        working_dir,
        CARGO_MACHETE_TIMEOUT,
        "cargo machete",
    )
    .map_err(|e| anyhow::anyhow!("failed to run cargo machete: {e}"))?;

    interpret_machete_output(output.status.code(), &output.stdout, &output.stderr)
}

/// Interpret an already-collected `cargo machete` result.
///
/// Exit `1` is the "found something" answer, not a tool failure: it is parsed
/// like exit `0`, and each exit code must agree with what stdout says.
///
/// # Errors
///
/// Fails closed, rather than scoring the run as "no unused dependencies", if
/// cargo-machete was killed by a signal, exited with any status other than
/// `0` or `1`, reported a manifest it could not analyse (it does so on stderr
/// and still exits `0`), or printed stdout this parser does not recognise or
/// that contradicts its exit code.
pub fn interpret_machete_output(
    exit_code: Option<i32>,
    stdout: &[u8],
    stderr: &[u8],
) -> anyhow::Result<Vec<UnusedDepEntry>> {
    let stderr = String::from_utf8_lossy(stderr);
    let found_expected = match exit_code {
        Some(0) => false,
        Some(1) => true,
        Some(2) => anyhow::bail!(
            "cargo machete exited with status 2 (error): {:?}",
            truncate_for_log(stderr.trim())
        ),
        None => anyhow::bail!(
            "cargo machete terminated by signal (exit_code = None); \
             refusing to treat partial output as authoritative"
        ),
        Some(other) => anyhow::bail!(
            "cargo machete exited with unexpected status code {other}; \
             refusing to treat its output as authoritative. stderr (truncated): {:?}",
            truncate_for_log(stderr.trim())
        ),
    };

    check_manifest_errors(&stderr)?;

    let stdout = String::from_utf8_lossy(stdout);
    let entries = match parse_machete_stdout(&stdout)? {
        MacheteStdout::Clean if !found_expected => Vec::new(),
        MacheteStdout::Found(entries) if found_expected => entries,
        MacheteStdout::Clean => anyhow::bail!(
            "cargo machete exited with status 1 but stdout reported no unused dependencies; \
             refusing to score as clean — suspect cargo-machete output drift"
        ),
        MacheteStdout::Found(entries) => anyhow::bail!(
            "cargo machete exited with status 0 but stdout listed {} unused dependenc(ies); \
             refusing to guess which one is right — suspect cargo-machete output drift",
            entries.len()
        ),
    };
    Ok(entries)
}

/// cargo-machete logs a manifest it cannot read (a TOML error, an unreadable
/// file) to stderr, skips it, and still exits `0`. Scoring that run as clean
/// would pass every dependency of the broken manifest unchecked.
fn check_manifest_errors(stderr: &str) -> anyhow::Result<()> {
    if let Some(line) = stderr
        .lines()
        .find(|l| l.trim_start().starts_with(MANIFEST_ERROR_MARKER))
    {
        anyhow::bail!(
            "cargo machete could not analyse a manifest, so its result is incomplete: {:?}",
            truncate_for_log(line.trim())
        );
    }
    Ok(())
}

/// What cargo-machete's stdout said, independent of the exit code.
#[derive(Debug)]
enum MacheteStdout {
    Clean,
    Found(Vec<UnusedDepEntry>),
}

/// Where the line walk is in cargo-machete's stdout.
enum State {
    /// Before the summary or listing header.
    Preamble,
    /// After the clean summary line.
    Clean,
    /// Inside the `<package> -- <manifest>:` listing.
    Listing,
    /// After the blank line that ends the listing: free-form advice text.
    Trailer,
}

/// Parse cargo-machete's stdout, failing on any line outside the known shape
/// rather than skipping it: a skipped line in the listing is a dropped
/// unused dependency.
fn parse_machete_stdout(stdout: &str) -> anyhow::Result<MacheteStdout> {
    let mut state = State::Preamble;
    let mut entries: Vec<UnusedDepEntry> = Vec::new();
    // The package header the next dependency line belongs to, plus how many
    // dependencies it has collected so far.
    let mut current: Option<(String, String, usize)> = None;

    for line in stdout.lines() {
        match state {
            State::Preamble => {
                if line.trim().is_empty() {
                    continue;
                }
                if line.starts_with(CLEAN_MARKER) {
                    state = State::Clean;
                } else if line.starts_with(FOUND_MARKER) {
                    state = State::Listing;
                } else {
                    return Err(unrecognised(line));
                }
            }
            State::Clean => {
                if !line.trim().is_empty() {
                    return Err(unrecognised(line));
                }
            }
            State::Listing => {
                if line.trim().is_empty() {
                    check_package_has_deps(current.as_ref())?;
                    current = None;
                    state = State::Trailer;
                } else if let Some(dep) = line.strip_prefix('\t') {
                    let dep = dep.trim();
                    let Some((package, manifest, count)) = current.as_mut() else {
                        return Err(unrecognised(line));
                    };
                    if dep.is_empty() || dep.contains(char::is_whitespace) {
                        return Err(unrecognised(line));
                    }
                    entries.push(UnusedDepEntry {
                        package: package.clone(),
                        manifest_path: manifest.clone(),
                        dependency: dep.to_string(),
                    });
                    // One increment per line of an in-memory string, so
                    // `saturating_add` equals `+= 1` exactly.
                    *count = count.saturating_add(1);
                } else if let Some((package, manifest)) = parse_package_header(line) {
                    check_package_has_deps(current.as_ref())?;
                    current = Some((package.to_string(), manifest.to_string(), 0));
                } else {
                    return Err(unrecognised(line));
                }
            }
            State::Trailer => {}
        }
    }

    match state {
        State::Preamble => anyhow::bail!(
            "cargo machete printed neither its clean summary nor an unused-dependency listing; \
             refusing to score as clean — suspect cargo-machete output drift. \
             stdout (truncated): {:?}",
            truncate_for_log(stdout.trim())
        ),
        State::Clean => Ok(MacheteStdout::Clean),
        State::Listing | State::Trailer => {
            check_package_has_deps(current.as_ref())?;
            if entries.is_empty() {
                anyhow::bail!(
                    "cargo machete announced unused dependencies but listed none; \
                     refusing to score as clean — suspect cargo-machete output drift"
                );
            }
            Ok(MacheteStdout::Found(entries))
        }
    }
}

/// Split a `<package> -- <manifest>:` line. Package names cannot contain
/// spaces, so the first ` -- ` is the separator even if the path has one.
fn parse_package_header(line: &str) -> Option<(&str, &str)> {
    let (package, manifest) = line.strip_suffix(':')?.split_once(" -- ")?;
    if package.is_empty() || package.contains(char::is_whitespace) || manifest.is_empty() {
        return None;
    }
    Some((package, manifest))
}

/// cargo-machete prints a package only when it has an unused dependency, so a
/// header with nothing under it means the dependency lines changed shape.
fn check_package_has_deps(current: Option<&(String, String, usize)>) -> anyhow::Result<()> {
    if let Some((package, manifest, 0)) = current {
        anyhow::bail!(
            "cargo machete listed package `{package}` ({manifest}) with no dependency under it; \
             refusing to score the listing as complete — suspect cargo-machete output drift"
        );
    }
    Ok(())
}

fn unrecognised(line: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "cargo machete printed a line this parser does not recognise; refusing to score the \
         output as authoritative — suspect cargo-machete output drift. line: {:?}",
        truncate_for_log(line)
    )
}

#[cfg(test)]
mod tests;
