//! Parser for `cargo deny check` JSON output.

use crate::{AdvisoryEntry, BanEntry, DenyEntry, DenyResult, LicenseEntry, SourceEntry};
use ops_core::subprocess::run_cargo;
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;

use super::truncate_for_log;

/// Default timeout for `cargo deny check`; overridable via
/// `OPS_SUBPROCESS_TIMEOUT_SECS`. Advisory DB refresh can dominate runtime.
const CARGO_DENY_TIMEOUT: Duration = Duration::from_mins(4);

/// cargo-deny diagnostic class. The code → section mapping lives once, in
/// [`classify_code`], so adding a class is one match arm rather than another
/// branch inside `parse_deny_output`.
#[derive(Copy, Clone)]
enum DiagClass {
    Advisory,
    License,
    /// An unused `[licenses]` config entry; see [`is_unused_license_config`].
    UnusedLicenseAllowance,
    Ban,
    Source,
}

fn classify_code(code: &str) -> Option<DiagClass> {
    match code {
        "vulnerability" | "notice" | "unmaintained" | "unsound" | "yanked" => {
            Some(DiagClass::Advisory)
        }
        "rejected" | "unlicensed" | "no-license-field" => Some(DiagClass::License),
        code if is_unused_license_config(code) => Some(DiagClass::UnusedLicenseAllowance),
        "banned" | "not-allowed" | "duplicate" | "workspace-duplicate" => Some(DiagClass::Ban),
        "source-not-allowed" | "git-source-underspecified" => Some(DiagClass::Source),
        _ => None,
    }
}

/// cargo-deny codes that report an unused `[licenses]` config entry rather
/// than a crate: `license-not-encountered` (an `allow` entry no dependency
/// uses, governed by `unused-allowed-license`, `warn` by default) and its
/// `exceptions` sibling. They carry `graphs: []`, and their subject is the
/// license named in `labels[0].span`. Classifying them keeps a shared
/// allow-list baseline from tripping the partial-decode-loss guard. They land
/// in their own [`DenyResult::unused_license_allowances`] section so the gate
/// can treat them as config hygiene: a `warning` never fails it, while
/// `unused-allowed-license = "deny"` (an `error`) still does.
fn is_unused_license_config(code: &str) -> bool {
    matches!(
        code,
        "license-not-encountered" | "license-exception-not-encountered"
    )
}

/// Run `cargo deny check` and parse the JSON output.
///
/// See [`interpret_deny_result`] for how the exit code is read.
///
/// # Errors
///
/// If `cargo deny` cannot be spawned, exceeds its timeout, or exits with a
/// status that does not carry a parseable diagnostic stream.
pub fn run_cargo_deny(working_dir: &Path) -> anyhow::Result<DenyResult> {
    let output = run_cargo(
        &["deny", "--format", "json", "check"],
        working_dir,
        CARGO_DENY_TIMEOUT,
        "cargo deny check",
    )
    .map_err(|e| anyhow::anyhow!("failed to run cargo deny: {e}"))?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    interpret_deny_result(output.status.code(), &stderr)
}

/// Every bit `cargo deny check` can set in a failing exit code: advisories
/// (`1`), bans (`2`), licenses (`4`) and sources (`8`). A licenses-only
/// failure exits `4`, bans plus licenses exits `6`.
const CHECK_FAILURE_MASK: i32 = 0b1111;

/// Whether `code` is a non-empty set of failed checks.
const fn is_check_failure(code: i32) -> bool {
    code > 0 && code & !CHECK_FAILURE_MASK == 0
}

/// Map a cargo-deny `(exit_code, stderr)` pair to either a parsed
/// `DenyResult` or a hard error.
///
/// `cargo deny check` exits `0` when no check failed, and otherwise with a
/// bitset of the checks that did ([`CHECK_FAILURE_MASK`]). Two failures that
/// are not findings share those values: a broken `deny.toml` exits `1` after
/// an `ERROR` log envelope, and a usage error exits `2` with plain-text
/// stderr. The stream content tells them apart from a check failure, since
/// neither carries a classifiable diagnostic.
///
/// # Errors
///
/// If `cargo deny` reported a failure without a decodable finding (a
/// configuration or usage error, or a crash before any diagnostic), was
/// killed by a signal, or exited with a status outside the check bitset.
pub fn interpret_deny_result(exit_code: Option<i32>, stderr: &str) -> anyhow::Result<DenyResult> {
    match exit_code {
        Some(0) => interpret_passing_run(stderr),
        Some(code) if is_check_failure(code) => interpret_failing_run(code, stderr),
        None => anyhow::bail!(
            "cargo deny terminated by signal (exit_code = None); \
             refusing to treat partial diagnostics as authoritative"
        ),
        Some(other) => anyhow::bail!(
            "cargo deny exited with unexpected status code {other}; \
             refusing to treat partial diagnostics as authoritative. \
             stderr (truncated): {:?}",
            truncate_for_log(stderr.trim())
        ),
    }
}

/// Exit 0 is not the rare case — cargo-deny exits 0 whenever every finding is
/// at `warning` level (the default for `[bans] multiple-versions`, and for
/// `unmaintained` / `yanked` configured as `warn`). It goes through the same
/// guarded parse as a failing run so `check_partial_decode_loss` runs on the
/// code path the gate normally takes. On a genuinely clean run there are no
/// diagnostic envelopes and the guard is a no-op.
fn interpret_passing_run(stderr: &str) -> anyhow::Result<DenyResult> {
    let (parsed, diag) = parse_deny_output_inner(stderr);
    // Fail closed on a non-empty stream that decoded *nothing* — not even a
    // `log` / `summary` envelope. A warning printed as plain text (a wrapper
    // around cargo-deny, a future default output change) is not a clean run;
    // scoring it green would be the same silent muting the zero-diagnostics
    // guard of a failing run exists to prevent. A stream whose every line
    // decoded as an envelope stays accepted, warnings included.
    if !stderr.trim().is_empty() && diag.envelopes_seen == 0 {
        anyhow::bail!(
            "cargo deny exited with status 0 but stderr carried no decodable JSON \
             envelopes; refusing to score as clean — likely non-JSON (text-mode) \
             output. stderr (truncated): {:?}",
            truncate_for_log(stderr.trim())
        );
    }
    check_partial_decode_loss(&diag, stderr)?;
    Ok(parsed)
}

/// A failing exit promises that stderr carries the JSON diagnostics of the
/// checks that failed. Anything short of that is a pipeline failure, never
/// "no issues parsed".
fn interpret_failing_run(code: i32, stderr: &str) -> anyhow::Result<DenyResult> {
    // An empty/whitespace-only stderr means the binary crashed before
    // printing diagnostics.
    if stderr.trim().is_empty() {
        anyhow::bail!(
            "cargo deny exited with status {code} but produced no diagnostics on stderr; \
             treating as pipeline failure (binary may have crashed before emitting JSON)"
        );
    }
    let (parsed, diag) = parse_deny_output_inner(stderr);
    if diag.entries_emitted == 0 {
        // cargo-deny gave up before any check reported: it logs the reason
        // (an unreadable or invalid `deny.toml`, a missing manifest) at
        // `ERROR` and exits non-zero.
        if let Some(reason) = &diag.error_log {
            anyhow::bail!(
                "cargo deny exited with status {code} before reporting any finding \
                 (configuration error): {:?}",
                truncate_for_log(reason)
            );
        }
        // Zero diagnostics decoded from a non-empty stderr means the stream
        // is text-mode (a usage error, a forgotten `--format json`, a
        // cargo-deny default change, or a wrapper that swallowed the JSON) —
        // every line was logged at debug by `decode_diagnostic` and the gate
        // would otherwise score green. Fail closed so drift surfaces instead
        // of silently muting the supply-chain gate.
        anyhow::bail!(
            "cargo deny exited with status {code} but stderr decoded zero diagnostics; \
             refusing to score as clean — likely a usage error or non-JSON (text-mode) \
             output. stderr (truncated): {:?}",
            truncate_for_log(stderr.trim())
        );
    }
    check_partial_decode_loss(&diag, stderr)?;
    Ok(parsed)
}

/// What [`parse_deny_output_inner`] saw versus what it kept. Mirrors
/// `UpgradeParseDiagnostics` in `parse/upgrade.rs`, which carries the same
/// information for the cargo-upgrade table: "we saw N candidate lines and
/// emitted zero entries" is the shape of drift the result value alone cannot
/// express.
struct DenyParseDiagnostics {
    /// Lines that decoded as a cargo-deny JSON envelope of *any* `type`
    /// (`diagnostic`, `log`, `summary`, …). Zero envelopes from a non-empty
    /// stream means the output was not the `--format json` contract at all.
    envelopes_seen: usize,
    /// Lines whose envelope decoded with `type == "diagnostic"` — cargo-deny
    /// telling us "this is a finding". `log` / `summary` envelopes and
    /// unparseable lines are excluded: they are not findings, so counting
    /// them would inflate the denominator on every normal run.
    candidate_diagnostics: usize,
    /// Candidates that made it into one of the `DenyResult` sections.
    entries_emitted: usize,
    /// The message of the first `log` envelope at `ERROR` level: cargo-deny
    /// explaining why it stopped (e.g. a `deny.toml` it could not parse).
    error_log: Option<String>,
}

impl DenyParseDiagnostics {
    /// Candidates dropped by the missing-`code` path or by `classify_code`
    /// returning `None`.
    const fn dropped(&self) -> usize {
        self.candidate_diagnostics
            .saturating_sub(self.entries_emitted)
    }
}

/// The share of candidate diagnostics that may be dropped before the stream
/// stops being trustworthy, as `NUM / DEN`.
///
/// cargo-deny emits its four check classes (advisories, licenses, bans,
/// sources) from four different implementations with different field shapes,
/// so a schema change usually takes out *one whole class* while the other
/// three keep decoding. The zero-diagnostics check above sees only total
/// loss; without a share-based guard, every advisory could drop while one
/// unrelated ban still decoded, and `ops deps` would render "Advisories:
/// None" in green and exit 0 with an unpatched RUSTSEC vulnerability in the
/// tree.
///
/// One unrecognised code among many findings is ordinary forward drift and
/// stays tolerated (it is still logged at debug). A quarter of the stream
/// disappearing is a class going missing.
const MAX_DROPPED_SHARE_NUM: usize = 1;
const MAX_DROPPED_SHARE_DEN: usize = 4;

/// Fail closed when cargo-deny reported diagnostics that we largely could not
/// decode or classify.
fn check_partial_decode_loss(diag: &DenyParseDiagnostics, stderr: &str) -> anyhow::Result<()> {
    let dropped = diag.dropped();
    // Both operands are line counts of an in-memory string, so the
    // saturating ops equal plain multiplication here; they keep the
    // comparison total rather than a debug-only panic.
    if dropped > 0
        && dropped.saturating_mul(MAX_DROPPED_SHARE_DEN)
            > diag
                .candidate_diagnostics
                .saturating_mul(MAX_DROPPED_SHARE_NUM)
    {
        tracing::warn!(
            candidate_diagnostics = diag.candidate_diagnostics,
            entries_emitted = diag.entries_emitted,
            dropped,
            "cargo-deny reported diagnostics that could not be decoded or classified; \
             refusing to treat the surviving subset as the complete finding set"
        );
        anyhow::bail!(
            "cargo deny emitted {candidates} diagnostic line(s) but only \
             {emitted} could be decoded and classified ({dropped} dropped); refusing to score the \
             surviving subset as the complete finding set — suspect a per-code cargo-deny schema \
             change that silently removed a whole diagnostic class. \
             stderr (truncated): {tail:?}",
            candidates = diag.candidate_diagnostics,
            emitted = diag.entries_emitted,
            dropped = dropped,
            tail = truncate_for_log(stderr.trim())
        );
    }
    Ok(())
}

/// JSON structures for cargo deny output (newline-delimited JSON on stderr).
///
/// The envelope is deliberately decoded on its own, with `fields` left as an
/// undecoded `Value`. Recognising a line as a diagnostic must not depend on
/// this crate agreeing with cargo-deny about the *shape* of `fields` — that
/// is exactly the schema drift the candidate counter exists to expose.
#[derive(Deserialize)]
struct DenyLine {
    #[serde(rename = "type")]
    line_type: String,
    fields: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct DiagnosticFields {
    severity: Option<String>,
    message: Option<String>,
    code: Option<String>,
    graphs: Option<Vec<DenyGraph>>,
    advisory: Option<DenyAdvisory>,
    /// Left undecoded: only config-level diagnostics read it (see
    /// [`label_subject`]), so a `labels` shape change must not drop every
    /// crate diagnostic along with it.
    labels: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct DenyGraph {
    #[serde(rename = "Krate")]
    krate: Option<DenyKrate>,
}

#[derive(Deserialize)]
struct DenyKrate {
    name: String,
}

#[derive(Deserialize)]
struct DenyAdvisory {
    id: String,
    package: Option<String>,
    title: Option<String>,
}

/// The message of a `log` envelope at `ERROR` level, if `line` is one.
fn error_log_message(line: &DenyLine) -> Option<String> {
    if line.line_type != "log" {
        return None;
    }
    let fields = line.fields.as_ref()?;
    let level = fields.get("level")?.as_str()?;
    if !level.eq_ignore_ascii_case("error") {
        return None;
    }
    Some(fields.get("message")?.as_str()?.to_owned())
}

struct DecodedDiagnostic {
    code: String,
    severity: String,
    message: String,
    advisory: Option<DenyAdvisory>,
    graphs: Option<Vec<DenyGraph>>,
    labels: Option<serde_json::Value>,
}

/// Decode one stderr line.
///
/// `diag` records whether the line was a *candidate* diagnostic
/// (`type == "diagnostic"`), which is what makes a later drop countable.
/// Unparseable lines and `log` / `summary` envelopes are not candidates —
/// cargo-deny is not claiming a finding on those.
fn decode_diagnostic(trimmed: &str, diag: &mut DenyParseDiagnostics) -> Option<DecodedDiagnostic> {
    let deny_line: DenyLine = match serde_json::from_str(trimmed) {
        Ok(l) => {
            // Counted before the `line_type` dispatch: even a `log` /
            // `summary` envelope is evidence the stream *is* the JSON
            // contract, which is what the exit-0 zero-envelope guard reads.
            diag.envelopes_seen = diag.envelopes_seen.saturating_add(1);
            l
        }
        Err(e) => {
            tracing::debug!(
                error = %e,
                line = %truncate_for_log(trimmed),
                "skipping malformed cargo-deny JSON line"
            );
            return None;
        }
    };
    if deny_line.line_type != "diagnostic" {
        if diag.error_log.is_none() {
            diag.error_log = error_log_message(&deny_line);
        }
        return None;
    }
    // One increment per line of an in-memory string, whose length is bounded
    // by `isize::MAX`, so `saturating_add` equals `+= 1` exactly.
    //
    // The candidate is counted as soon as the *envelope* says
    // `type == "diagnostic"`, before `fields` is decoded. Counting only after
    // a successful `fields` decode would let a diagnostic whose `fields` no
    // longer match `DiagnosticFields` — a renamed key, a scalar where an
    // object is expected — be dropped without ever being counted, keeping the
    // drop-rate guard silent through precisely the schema drift it watches
    // for.
    diag.candidate_diagnostics = diag.candidate_diagnostics.saturating_add(1);
    let Some(raw_fields) = deny_line.fields else {
        tracing::debug!(
            line = %truncate_for_log(trimmed),
            "skipping cargo-deny diagnostic with no `fields` object (possible schema drift)"
        );
        return None;
    };
    let fields: DiagnosticFields = match serde_json::from_value(raw_fields) {
        Ok(f) => f,
        Err(e) => {
            tracing::debug!(
                error = %e,
                line = %truncate_for_log(trimmed),
                "skipping cargo-deny diagnostic whose `fields` failed to decode (possible schema drift)"
            );
            return None;
        }
    };
    let Some(code) = fields.code else {
        // Every drop path in this parser leaves a tracing breadcrumb,
        // including this one: a schema change that moves `code` under a
        // nested object must not drop diagnostics silently.
        tracing::debug!(
            severity = %fields.severity.as_deref().unwrap_or(MISSING_SEVERITY_SENTINEL),
            message = %truncate_for_log(fields.message.as_deref().unwrap_or("")),
            "skipping cargo-deny diagnostic with no `code` field (possible schema drift)"
        );
        return None;
    };
    let severity = if let Some(s) = fields.severity {
        s
    } else {
        tracing::warn!(
            code = %code,
            message = %truncate_for_log(fields.message.as_deref().unwrap_or("")),
            "cargo-deny diagnostic missing severity; substituting `<missing-severity>` sentinel \
             (treated as actionable / fail-closed by has_issues)"
        );
        MISSING_SEVERITY_SENTINEL.to_string()
    };
    Some(DecodedDiagnostic {
        code,
        severity,
        message: fields.message.unwrap_or_default(),
        advisory: fields.advisory,
        graphs: fields.graphs,
        labels: fields.labels,
    })
}

/// Sentinel substituted by [`decode_diagnostic`] when a cargo-deny diagnostic
/// line lacks a `severity` field. It classifies as
/// `SeverityClass::Unknown`, so the unknown-severity warn fires and the
/// `has_issues` gate still fails: schema drift surfaces rather than silently
/// muting the gate.
pub const MISSING_SEVERITY_SENTINEL: &str = "<missing-severity>";

/// Answer "which package is this diagnostic about".
///
/// Falls back to the advisory's package, then to `graphs[0].krate.name`, then
/// — for config-level diagnostics only ([`is_unused_license_config`]) — to
/// the license in `labels[0].span`, then to a `<no package>` sentinel.
///
/// The read is immutable and clones the name: leaving `diag` untouched makes
/// the function idempotent, so no caller has to know which fields a previous
/// call emptied. Hollowing the fields out instead would cost nothing at the
/// type level and produce wrong data — a second call would report
/// `<no package>` for a diagnostic that has one — for the sake of one
/// `String` clone per diagnostic on a path that already allocates a `String`
/// per field.
fn resolve_package(diag: &DecodedDiagnostic) -> String {
    diag.advisory
        .as_ref()
        .and_then(|a| a.package.clone())
        .or_else(|| {
            diag.graphs
                .as_ref()
                .and_then(|g| g.first())
                .and_then(|g| g.krate.as_ref())
                .map(|k| k.name.clone())
        })
        .or_else(|| {
            is_unused_license_config(&diag.code)
                .then(|| label_subject(diag))
                .flatten()
        })
        .unwrap_or_else(|| {
            tracing::debug!(
                code = %diag.code,
                severity = %diag.severity,
                message = %truncate_for_log(&diag.message),
                "cargo-deny diagnostic had no package name in advisory or graphs[0].krate; \
                 substituting <no package> sentinel"
            );
            "<no package>".to_string()
        })
}

/// The first label's `span` — for a config-level diagnostic, the `deny.toml`
/// value it is about (e.g. the unused license `0BSD`).
fn label_subject(diag: &DecodedDiagnostic) -> Option<String> {
    diag.labels
        .as_ref()?
        .get(0)?
        .get("span")?
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// Parse newline-delimited JSON from `cargo deny --format json check` stderr.
///
/// Drops undecodable lines one at a time. Callers that need to know *how
/// many* were dropped — the difference between "clean" and "a whole
/// diagnostic class stopped decoding" — must go through
/// [`interpret_deny_result`], which applies `check_partial_decode_loss`.
pub fn parse_deny_output(stderr: &str) -> DenyResult {
    parse_deny_output_inner(stderr).0
}

fn parse_deny_output_inner(stderr: &str) -> (DenyResult, DenyParseDiagnostics) {
    let mut result = DenyResult::default();
    let mut counts = DenyParseDiagnostics {
        envelopes_seen: 0,
        candidate_diagnostics: 0,
        entries_emitted: 0,
        error_log: None,
    };
    for line in stderr.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some(diag) = decode_diagnostic(trimmed, &mut counts) else {
            continue;
        };
        let Some(class) = classify_code(&diag.code) else {
            tracing::debug!(
                code = %diag.code,
                severity = %diag.severity,
                message = %truncate_for_log(&diag.message),
                "skipping cargo-deny diagnostic with unknown code (possible schema drift)"
            );
            continue;
        };
        push_diagnostic(&mut result, class, diag);
        // Bounded by the candidate count, itself bounded by the line count
        // of an in-memory string, so `saturating_add` equals `+= 1` exactly.
        counts.entries_emitted = counts.entries_emitted.saturating_add(1);
    }
    (result, counts)
}

fn push_diagnostic(result: &mut DenyResult, class: DiagClass, diag: DecodedDiagnostic) {
    // `resolve_package` reads immutably, so the fields consumed below are
    // still whatever cargo-deny sent.
    let package = resolve_package(&diag);
    match class {
        DiagClass::Advisory => {
            let (id, title) = match diag.advisory {
                Some(adv) => (adv.id, adv.title.unwrap_or(diag.message)),
                None => (diag.code, diag.message),
            };
            result.advisories.push(AdvisoryEntry {
                id,
                package,
                severity: diag.severity,
                title,
            });
        }
        DiagClass::License => result.licenses.push(LicenseEntry(DenyEntry {
            package,
            message: diag.message,
            severity: diag.severity,
        })),
        DiagClass::UnusedLicenseAllowance => {
            result
                .unused_license_allowances
                .push(LicenseEntry(DenyEntry {
                    package,
                    message: diag.message,
                    severity: diag.severity,
                }));
        }
        DiagClass::Ban => result.bans.push(BanEntry(DenyEntry {
            package,
            message: diag.message,
            severity: diag.severity,
        })),
        DiagClass::Source => result.sources.push(SourceEntry(DenyEntry {
            package,
            message: diag.message,
            severity: diag.severity,
        })),
    }
}

#[cfg(test)]
mod tests;
