//! Parser for `cargo deny check` JSON output.

use crate::{AdvisoryEntry, BanEntry, DenyResult, LicenseEntry, SourceEntry};
use ops_core::subprocess::run_cargo;
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;

use super::truncate_for_log;

/// Default timeout for `cargo deny check`; overridable via
/// `OPS_SUBPROCESS_TIMEOUT_SECS`. Advisory DB refresh can dominate runtime.
const CARGO_DENY_TIMEOUT: Duration = Duration::from_secs(240);

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

/// Single source of truth for the cargo-deny diagnostic code dispatch.
const CODE_CLASSES: &[(&str, DiagClass)] = &[
    // Advisories
    ("vulnerability", DiagClass::Advisory),
    ("notice", DiagClass::Advisory),
    ("unmaintained", DiagClass::Advisory),
    ("unsound", DiagClass::Advisory),
    ("yanked", DiagClass::Advisory),
    // Licenses
    ("rejected", DiagClass::License),
    ("unlicensed", DiagClass::License),
    ("no-license-field", DiagClass::License),
    // Bans
    ("banned", DiagClass::Ban),
    ("not-allowed", DiagClass::Ban),
    ("duplicate", DiagClass::Ban),
    ("workspace-duplicate", DiagClass::Ban),
    // Sources
    ("source-not-allowed", DiagClass::Source),
    ("git-source-underspecified", DiagClass::Source),
];

fn classify_code(code: &str) -> Option<DiagClass> {
    CODE_CLASSES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, class)| *class)
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
pub fn run_cargo_deny(working_dir: &Path) -> anyhow::Result<DenyResult> {
    let output = run_cargo(
        &["deny", "--format", "json", "check"],
        working_dir,
        CARGO_DENY_TIMEOUT,
        "cargo deny check",
    )
    .map_err(|e| anyhow::anyhow!("failed to run cargo deny: {}", e))?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    interpret_deny_result(output.status.code(), &stderr)
}

/// Map a cargo-deny `(exit_code, stderr)` pair to either a parsed
/// `DenyResult` or a hard error.
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
            let parsed = parse_deny_output(stderr);
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
                    truncate_for_log(stderr)
                );
            }
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
            truncate_for_log(stderr)
        ),
    }
}

/// JSON structures for cargo deny output (newline-delimited JSON on stderr).
#[derive(Deserialize)]
struct DenyLine {
    #[serde(rename = "type")]
    line_type: String,
    fields: serde_json::Value,
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

fn decode_diagnostic(trimmed: &str) -> Option<DecodedDiagnostic> {
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
    let fields: DiagnosticFields = match serde_json::from_value(deny_line.fields) {
        Ok(f) => f,
        Err(e) => {
            tracing::debug!(
                error = %e,
                line = %truncate_for_log(trimmed),
                "ERR-1: skipping cargo-deny diagnostic with unexpected fields shape"
            );
            return None;
        }
    };
    let code = fields.code?;
    let severity = match fields.severity {
        Some(s) => s,
        None => {
            tracing::warn!(
                code = %code,
                message = %truncate_for_log(fields.message.as_deref().unwrap_or("")),
                "TASK-0845: cargo-deny diagnostic missing severity; substituting `<missing-severity>` sentinel \
                 (treated as actionable / fail-closed by has_issues)"
            );
            MISSING_SEVERITY_SENTINEL.to_string()
        }
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
pub(crate) const MISSING_SEVERITY_SENTINEL: &str = "<missing-severity>";

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
pub fn parse_deny_output(stderr: &str) -> DenyResult {
    let mut result = DenyResult::default();
    for line in stderr.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some(diag) = decode_diagnostic(trimmed) else {
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
    }
    result
}

fn push_diagnostic(result: &mut DenyResult, class: DiagClass, diag: DecodedDiagnostic) {
    let package = resolve_package(&diag);
    match class {
        DiagClass::Advisory => {
            let (id, title) = match &diag.advisory {
                Some(adv) => (
                    adv.id.clone(),
                    adv.title.clone().unwrap_or_else(|| diag.message.clone()),
                ),
                None => (diag.code.clone(), diag.message.clone()),
            };
            result.advisories.push(AdvisoryEntry {
                id,
                package,
                severity: diag.severity,
                title,
            });
        }
        DiagClass::License => result.licenses.push(LicenseEntry {
            package,
            message: diag.message,
            severity: diag.severity,
        }),
        DiagClass::Ban => result.bans.push(BanEntry {
            package,
            message: diag.message,
            severity: diag.severity,
        }),
        DiagClass::Source => result.sources.push(SourceEntry {
            package,
            message: diag.message,
            severity: diag.severity,
        }),
    }
}
