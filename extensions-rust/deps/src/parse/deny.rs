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

/// FN-1 (TASK-0793): cargo-deny diagnostic class. Centralises the code →
/// section mapping so adding a new class is one row in `CODE_CLASSES`
/// rather than a fifth `if … contains` branch in `parse_deny_output`.
#[derive(Copy, Clone)]
enum DiagClass {
    Advisory,
    License,
    Ban,
    Source,
}

fn classify_code(code: &str) -> Option<DiagClass> {
    match code {
        "vulnerability" | "notice" | "unmaintained" | "unsound" | "yanked" => {
            Some(DiagClass::Advisory)
        }
        "rejected" | "unlicensed" | "no-license-field" => Some(DiagClass::License),
        "banned" | "not-allowed" | "duplicate" | "workspace-duplicate" => Some(DiagClass::Ban),
        "source-not-allowed" | "git-source-underspecified" => Some(DiagClass::Source),
        _ => None,
    }
}

/// Run `cargo deny check` and parse the JSON output.
///
/// cargo-deny uses three exit codes (per its source/docs):
///
/// * `0` — clean: no issues found.
/// * `1` — issues found: stderr contains the JSON diagnostics we want to parse.
/// * `2` — configuration / usage error: e.g. an invalid `deny.toml`. In this
///   case stderr is *not* a diagnostic stream; treating it as one yields an
///   empty `DenyResult` and silently masks the misconfiguration. Surface the
///   error instead so operators see "broken deny.toml" rather than a clean
///   bill of health.
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

/// Map a cargo-deny `(exit_code, stderr)` pair to either a parsed
/// `DenyResult` or a hard error.
///
/// # Errors
///
/// If `cargo deny` exited 1 with empty stderr (the binary crashed before
/// printing diagnostics), was killed by a signal, or exited with an
/// unrecognised status.
pub fn interpret_deny_result(exit_code: Option<i32>, stderr: &str) -> anyhow::Result<DenyResult> {
    match exit_code {
        Some(0) => Ok(parse_deny_output(stderr)),
        Some(1) => {
            // ERR-1 / TASK-0612: cargo-deny's contract for exit 1 is "stderr
            // has the JSON diagnostic stream". An empty/whitespace-only
            // stderr at exit 1 means the binary crashed before printing
            // diagnostics — treating it as "no issues parsed" silently masks
            // a supply-chain pipeline failure.
            if stderr.trim().is_empty() {
                anyhow::bail!(
                    "cargo deny exited with status 1 but produced no diagnostics on stderr; \
                     treating as pipeline failure (binary may have crashed before emitting JSON)"
                );
            }
            let (parsed, diag) = parse_deny_output_inner(stderr);
            // ERR-1 / TASK-0958: cargo-deny's contract for exit 1 is "stderr
            // has at least one JSON diagnostic line". If the parse decoded
            // zero diagnostics from a non-empty stderr, the stream is text-mode
            // (forgotten `--format json`, future cargo-deny default change, or
            // a wrapper that swallowed JSON) — every line was logged at debug
            // by `decode_diagnostic` and the gate would otherwise score green.
            // Fail closed so schema drift surfaces instead of silently muting
            // the supply-chain gate.
            if parsed.advisories.is_empty()
                && parsed.licenses.is_empty()
                && parsed.bans.is_empty()
                && parsed.sources.is_empty()
            {
                anyhow::bail!(
                    "cargo deny exited with status 1 but stderr decoded zero diagnostics; \
                     refusing to score as clean — likely non-JSON (text-mode) output. \
                     stderr (truncated): {:?}",
                    truncate_for_log(stderr.trim())
                );
            }
            check_partial_decode_loss(&diag, stderr)?;
            Ok(parsed)
        }
        Some(2) => anyhow::bail!(
            "cargo deny exited with status 2 (configuration error): {:?}",
            truncate_for_log(stderr.trim())
        ),
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

/// ERR-1 / TASK-1840: what [`parse_deny_output_inner`] saw versus what it
/// kept. Mirrors `UpgradeParseDiagnostics` in `parse/upgrade.rs`, which
/// solved the same problem for the cargo-upgrade table: "we saw N candidate
/// rows and emitted zero entries" is the shape of drift the result value
/// alone cannot express.
struct DenyParseDiagnostics {
    /// Lines whose envelope decoded with `type == "diagnostic"` — cargo-deny
    /// telling us "this is a finding". `log` / `summary` envelopes and
    /// unparseable lines are excluded: they are not findings, so counting
    /// them would inflate the denominator on every normal run.
    candidate_diagnostics: usize,
    /// Candidates that made it into one of the four sections.
    entries_emitted: usize,
}

impl DenyParseDiagnostics {
    /// Candidates dropped by the missing-`code` path or by `classify_code`
    /// returning `None`.
    const fn dropped(&self) -> usize {
        self.candidate_diagnostics
            .saturating_sub(self.entries_emitted)
    }
}

/// ERR-1 / TASK-1840: the share of candidate diagnostics that may be dropped
/// before the stream stops being trustworthy, as `NUM / DEN`.
///
/// cargo-deny emits its four check classes (advisories, licenses, bans,
/// sources) from four different implementations with different field shapes,
/// so a schema change usually takes out *one whole class* while the other
/// three keep decoding. The zero-diagnostics guard above only sees total
/// loss, so that partial loss passed straight through: every advisory
/// dropped, one unrelated ban still decoded, `ops deps` rendered "Advisories:
/// None" in green and exited 0 with an unpatched RUSTSEC vulnerability in the
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
            "TASK-1840: cargo-deny reported diagnostics that could not be decoded or classified; \
             refusing to treat the surviving subset as the complete finding set"
        );
        anyhow::bail!(
            "cargo deny exited with status 1 and emitted {candidates} diagnostic line(s) but only \
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
#[derive(Deserialize)]
struct DenyLine {
    #[serde(rename = "type")]
    line_type: String,
    fields: DiagnosticFields,
}

#[derive(Deserialize)]
struct DiagnosticFields {
    severity: Option<String>,
    message: Option<String>,
    code: Option<String>,
    graphs: Option<Vec<DenyGraph>>,
    advisory: Option<DenyAdvisory>,
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

struct DecodedDiagnostic {
    code: String,
    severity: String,
    message: String,
    advisory: Option<DenyAdvisory>,
    graphs: Option<Vec<DenyGraph>>,
}

/// Decode one stderr line.
///
/// ERR-1 / TASK-1840: `diag` records whether the line was a *candidate*
/// diagnostic (`type == "diagnostic"`), which is what makes a later drop
/// countable. Unparseable lines and `log` / `summary` envelopes are not
/// candidates — cargo-deny is not claiming a finding on those.
fn decode_diagnostic(trimmed: &str, diag: &mut DenyParseDiagnostics) -> Option<DecodedDiagnostic> {
    let deny_line: DenyLine = match serde_json::from_str(trimmed) {
        Ok(l) => l,
        Err(e) => {
            tracing::debug!(
                error = %e,
                line = %truncate_for_log(trimmed),
                "ERR-1: skipping malformed cargo-deny JSON line"
            );
            return None;
        }
    };
    if deny_line.line_type != "diagnostic" {
        return None;
    }
    // One increment per line of an in-memory string, whose length is bounded
    // by `isize::MAX`, so `saturating_add` equals `+= 1` exactly.
    diag.candidate_diagnostics = diag.candidate_diagnostics.saturating_add(1);
    let fields = deny_line.fields;
    let Some(code) = fields.code else {
        // ERR-1 / TASK-1840 AC#4: this was the only drop path in the crate
        // with no tracing breadcrumb, so a schema change that moved `code`
        // under a nested object dropped diagnostics in complete silence.
        tracing::debug!(
            severity = %fields.severity.as_deref().unwrap_or(MISSING_SEVERITY_SENTINEL),
            message = %truncate_for_log(fields.message.as_deref().unwrap_or("")),
            "TASK-1840: skipping cargo-deny diagnostic with no `code` field (possible schema drift)"
        );
        return None;
    };
    let severity = if let Some(s) = fields.severity {
        s
    } else {
        tracing::warn!(
            code = %code,
            message = %truncate_for_log(fields.message.as_deref().unwrap_or("")),
            "TASK-0845: cargo-deny diagnostic missing severity; substituting `<missing-severity>` sentinel \
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
    })
}

/// ERR-2 / TASK-0845: shared sentinel used by [`decode_diagnostic`] when a
/// cargo-deny diagnostic line lacks a `severity` field. Routed through
/// `has_issues`'s fail-closed `_other` branch so the unknown-severity warn
/// fires and the gate still fails — preserving the safety property of
/// "schema drift surfaces, doesn't silently mute the gate".
pub const MISSING_SEVERITY_SENTINEL: &str = "<missing-severity>";

/// Answer "which package is this diagnostic about".
///
/// OWN-1 / TASK-1848: this borrows immutably and clones. It used to take
/// `&mut` and *hollow out* what it read — `advisory.package.take()`,
/// `mem::take(&mut krate.name)` — leaving the `DecodedDiagnostic` in a state
/// no reader could tell apart from genuine missing data, so a second call
/// returned the `<no package>` sentinel for a diagnostic with a perfectly
/// good package name and logged a false TASK-0597 warning to match. Nothing
/// called it twice, but the only thing preventing that was an unwritten
/// agreement between this function and [`push_diagnostic`] about which
/// fields had been emptied — invisible in the types, and a wrong-data bug
/// (not a compile error) the moment either side moved. The mutation bought
/// one `String` clone per cargo-deny diagnostic on a path that already
/// allocates a `String` per field.
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
        .unwrap_or_else(|| {
            tracing::debug!(
                code = %diag.code,
                severity = %diag.severity,
                message = %truncate_for_log(&diag.message),
                "TASK-0597: cargo-deny diagnostic had no package name in advisory or graphs[0].krate; \
                 substituting <no package> sentinel"
            );
            "<no package>".to_string()
        })
}

/// Parse newline-delimited JSON from `cargo deny --format json check` stderr.
///
/// Drops undecodable lines one at a time. Callers that need to know *how
/// many* were dropped — the difference between "clean" and "a whole
/// diagnostic class stopped decoding" — must go through
/// [`interpret_deny_result`], which applies [`check_partial_decode_loss`].
pub fn parse_deny_output(stderr: &str) -> DenyResult {
    parse_deny_output_inner(stderr).0
}

fn parse_deny_output_inner(stderr: &str) -> (DenyResult, DenyParseDiagnostics) {
    let mut result = DenyResult::default();
    let mut counts = DenyParseDiagnostics {
        candidate_diagnostics: 0,
        entries_emitted: 0,
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
                "TASK-0436: skipping cargo-deny diagnostic with unknown code (possible schema drift)"
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
    // OWN-1 / TASK-1848: an immutable read, so the fields consumed below are
    // still whatever cargo-deny sent. No agreement with `resolve_package`
    // about which of them it emptied is needed, because it empties none.
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
