//! Parsing logic for `cargo upgrade` and `cargo deny` output.

use crate::{
    AdvisoryEntry, BanEntry, DenyResult, LicenseEntry, SourceEntry, UpgradeEntry, UpgradeResult,
};
use ops_core::subprocess::run_cargo;
use serde::Deserialize;
use std::path::Path;
use std::time::Duration;

/// Default timeout for `cargo upgrade --dry-run`; overridable via
/// `OPS_SUBPROCESS_TIMEOUT_SECS`.
const CARGO_UPGRADE_TIMEOUT: Duration = Duration::from_secs(180);

/// Default timeout for `cargo deny check`; overridable via
/// `OPS_SUBPROCESS_TIMEOUT_SECS`. Advisory DB refresh can dominate runtime.
const CARGO_DENY_TIMEOUT: Duration = Duration::from_secs(240);

// ── cargo upgrade parsing ───────────────────────────────────────────────────

/// Run `cargo upgrade --dry-run` and parse the table output.
///
/// ERR-1 (TASK-0913): `cargo upgrade` exits non-zero on lockfile contention,
/// network failures, or a malformed `Cargo.toml`. The previous code parsed
/// stdout regardless of exit status and silently returned an empty
/// `Vec<UpgradeEntry>`, masking the upstream failure as "no upgrades
/// available". Surface non-zero exits as an error including the stderr
/// tail so the deps gate fails loudly. Mirrors the cargo-update fix made
/// in TASK-0502 and the cargo-deny exit-code handling below.
pub fn run_cargo_upgrade_dry_run(working_dir: &Path) -> anyhow::Result<Vec<UpgradeEntry>> {
    let output = run_cargo(
        &["upgrade", "--dry-run"],
        working_dir,
        CARGO_UPGRADE_TIMEOUT,
        "cargo upgrade --dry-run",
    )
    .map_err(|e| anyhow::anyhow!("failed to run cargo upgrade: {}", e))?;

    interpret_upgrade_output(output.status.code(), &output.stdout, &output.stderr)
}

/// Map a `cargo upgrade --dry-run` `(exit_code, stdout, stderr)` triple to
/// either a parsed `Vec<UpgradeEntry>` or an error including the stderr
/// tail. Split out so unit tests can pin the exit-code semantics without
/// spawning the binary.
pub fn interpret_upgrade_output(
    exit_code: Option<i32>,
    stdout: &[u8],
    stderr: &[u8],
) -> anyhow::Result<Vec<UpgradeEntry>> {
    match exit_code {
        Some(0) => {
            let stdout = String::from_utf8_lossy(stdout);
            let (entries, diag) = parse_upgrade_table_inner(&stdout);
            // PATTERN-1 / TASK-1074: if cargo-edit's table prints a `====`
            // separator row but the header line above it does not match any
            // recognised token shape (renamed / localised columns), refuse to
            // score the run as "no upgrades available". The separator alone
            // would still calibrate column offsets and yield rows, but the
            // header drift is the canary for an upstream format change we
            // have not vetted — fail closed so operators see the drift
            // signal instead of silently downgrading the supply-chain gate.
            if diag.saw_separator && !diag.saw_recognised_header {
                tracing::warn!(
                    "TASK-1074: cargo-upgrade stdout had a `====` separator row but no recognised header line; \
                     refusing to parse output as authoritative — suspect cargo-edit header-token drift"
                );
                anyhow::bail!(
                    "cargo upgrade --dry-run produced a table whose header line was not recognised \
                     (no `name` / `old req` / `new req` tokens); refusing to score as `no upgrades` — \
                     suspect cargo-edit format drift"
                );
            }
            Ok(entries)
        }
        None => anyhow::bail!(
            "cargo upgrade --dry-run terminated by signal (exit_code = None); \
             refusing to treat partial output as authoritative"
        ),
        Some(other) => {
            let stderr = String::from_utf8_lossy(stderr);
            // ERR-7 / SEC-21 / TASK-1160: format the stderr tail via Debug ({:?})
            // so embedded ANSI escapes / newlines / NULs from registry-served
            // diagnostics cannot forge log entries or repaint the operator
            // terminal. Mirrors the workspace policy enforced in probe.rs and
            // query.rs.
            anyhow::bail!(
                "cargo upgrade --dry-run exited with status {other}; \
                 refusing to parse output as authoritative. \
                 stderr (truncated): {:?}",
                truncate_for_log(stderr.trim())
            )
        }
    }
}

/// Parse the table output from `cargo upgrade --dry-run`.
///
/// Table format:
/// ```text
/// name   old req compatible latest  new req note
/// ====   ======= ========== ======  ======= ====
/// clap   3.0.0   3.2.25     4.6.0   3.2.25  incompatible
/// serde  1.0.100 1.0.228    1.0.228 1.0.228
/// ```
///
/// SEC-15 / TASK-0383: column offsets are calibrated from the `====` separator
/// row rather than splitting on whitespace, so multi-word notes (e.g. "pinned
/// by parent") and any future column additions don't silently shift values
/// across `UpgradeEntry` fields.
pub fn parse_upgrade_table(stdout: &str) -> Vec<UpgradeEntry> {
    parse_upgrade_table_inner(stdout).0
}

/// Diagnostics surfaced by [`parse_upgrade_table_inner`] so callers higher up
/// the stack (e.g. [`interpret_upgrade_output`]) can decide whether to bail
/// on suspected cargo-edit format drift instead of silently scoring the run
/// as "no upgrades available".
pub(crate) struct UpgradeParseDiagnostics {
    /// `true` once a `====` row aligned the column offsets.
    pub saw_separator: bool,
    /// `true` once a header line matched the recognised token shape
    /// (`name` + `old req` / `new req`, case-insensitive).
    pub saw_recognised_header: bool,
}

/// Inner parser shared by [`parse_upgrade_table`] and
/// [`interpret_upgrade_output`]. Returns the parsed rows plus diagnostics so
/// the exit-code-aware caller can fail closed on header-token drift
/// (TASK-1074) while the public [`parse_upgrade_table`] keeps its tolerant
/// `-> Vec<_>` signature for unit-test ergonomics.
pub(crate) fn parse_upgrade_table_inner(
    stdout: &str,
) -> (Vec<UpgradeEntry>, UpgradeParseDiagnostics) {
    let mut entries = Vec::new();
    let mut columns: Option<Vec<(usize, usize)>> = None;
    let mut saw_separator = false;
    let mut saw_recognised_header = false;
    let mut saw_body_line = false;

    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }

        // ERR-1 / TASK-1026: header detection must be case-insensitive and
        // tolerant of cargo-edit's column-name drift (e.g. `Name`, `Old Req`,
        // `New Req`). The cargo-edit table format is not a stable API, so a
        // capitalisation flip used to silently break detection — `columns`
        // stayed `None`, every data row was skipped, and the parser returned
        // an empty Vec, masquerading as "no upgrades available".
        let trimmed = line.trim_start();
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("name") && (lower.contains("old req") || lower.contains("new req")) {
            columns = None;
            saw_recognised_header = true;
            continue;
        }

        // Separator row: `====   ======= ==========` defines exact byte columns.
        if trimmed.starts_with("====") {
            columns = Some(separator_columns(line));
            saw_separator = true;
            continue;
        }

        saw_body_line = true;
        if let Some(cols) = columns.as_deref() {
            if let Some(entry) = parse_upgrade_row(line, cols) {
                entries.push(entry);
            }
        }
    }

    // ERR-1 / TASK-1026: if we saw non-empty body content but never recognised
    // a separator row, header detection almost certainly drifted (renamed or
    // localised columns). Surface this as a warn so the operator log carries
    // a breadcrumb instead of a silent empty-Vec / "no upgrades" result. The
    // exit-code guard from TASK-0913 only catches non-zero exits; cargo-upgrade
    // emits exit 0 with a re-rendered table, so this is the only signal.
    if saw_body_line && !saw_separator {
        tracing::warn!(
            "TASK-1026: cargo-upgrade stdout had body lines but no `====` separator row; \
             parse_upgrade_table is returning an empty result — suspect cargo-edit table-format drift"
        );
    }

    (
        entries,
        UpgradeParseDiagnostics {
            saw_separator,
            saw_recognised_header,
        },
    )
}

/// Slice a single data row from `cargo upgrade --dry-run` into an
/// [`UpgradeEntry`], or return `None` (with a tracing breadcrumb) when the
/// row cannot be aligned to the column offsets carried by the preceding
/// separator. FN-1 (TASK-0794): split out from `parse_upgrade_table` so the
/// state-machine and the row-slicing concerns sit at one abstraction level
/// each.
fn parse_upgrade_row(line: &str, cols: &[(usize, usize)]) -> Option<UpgradeEntry> {
    if cols.len() < 5 {
        tracing::debug!(
            column_count = cols.len(),
            line = %line,
            "TASK-0404: skipping cargo-upgrade row — separator row had fewer than 5 columns"
        );
        return None;
    }

    // READ-2 (TASK-0609): closure renamed from `take` (shadowed
    // `Iterator::take`) to `slice_col`; both row fields and note now
    // index `cols` via `.get(...)` for consistency.
    // ERR-1 / TASK-0960: clamp byte offsets to UTF-8 char boundaries before
    // slicing so a data row containing multi-byte characters (e.g. localised
    // note text) cannot panic when start/end land mid-codepoint.
    let slice_col = |idx: usize| -> Option<String> {
        let &(start, end) = cols.get(idx)?;
        let (start, end) = clamp_to_char_boundaries(line, start, end)?;
        let slice = &line[start..end];
        let trimmed = slice.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    };

    let (Some(name), Some(old_req), Some(compatible), Some(latest), Some(new_req)) = (
        slice_col(0),
        slice_col(1),
        slice_col(2),
        slice_col(3),
        slice_col(4),
    ) else {
        tracing::debug!(
            line = %line,
            "TASK-0404: skipping cargo-upgrade row that did not fill the 5 fixed columns"
        );
        return None;
    };

    // The note absorbs every byte from the start of column 5 to end of
    // line so multi-word notes survive intact. If the upstream format
    // grows extra columns past the note, they roll up here too — at least
    // the five fixed fields stay correctly aligned. When the separator
    // row has no note column at all, there is simply no note.
    let note = cols.get(5).and_then(|(start, _)| {
        // ERR-1 / TASK-0960: clamp `start` to a UTF-8 char boundary so a row
        // with multi-byte characters cannot panic when slicing.
        let (start, end) = clamp_to_char_boundaries(line, *start, line.len())?;
        let slice = line[start..end].trim();
        if slice.is_empty() {
            None
        } else {
            Some(slice.to_string())
        }
    });

    Some(UpgradeEntry {
        name,
        old_req,
        compatible,
        latest,
        new_req,
        note,
    })
}

/// Clamp a `[start, end)` byte range derived from the (ASCII) separator row
/// to UTF-8 char boundaries on a (possibly multi-byte) data row.
///
/// ERR-1 / TASK-0960: `cargo upgrade --dry-run` prints a separator row of
/// `=` characters that defines column boundaries by byte offset. Data rows
/// can contain multi-byte UTF-8 (localised notes, non-ASCII metadata) so the
/// raw separator offsets can land mid-codepoint and cause `&line[start..end]`
/// to panic. Clamp inward to the nearest char boundary so the slice is
/// always valid UTF-8 — emit a `tracing::warn` breadcrumb when clamping
/// actually had to move an offset so schema-shape drift is observable.
/// Returns `None` when the clamp collapses the range to empty (start past
/// end-of-line, or no boundary at-or-below `start`).
fn clamp_to_char_boundaries(line: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let len = line.len();
    if start >= len {
        return None;
    }
    let clamped_end = end.min(len);
    let mut s = start;
    while s < clamped_end && !line.is_char_boundary(s) {
        s += 1;
    }
    let mut e = clamped_end;
    while e > s && !line.is_char_boundary(e) {
        e -= 1;
    }
    if s >= e {
        return None;
    }
    if s != start || e != clamped_end {
        tracing::warn!(
            requested_start = start,
            requested_end = clamped_end,
            adjusted_start = s,
            adjusted_end = e,
            line = %line,
            "TASK-0960: cargo-upgrade row slice clamped to UTF-8 char boundaries (multi-byte content crossed a column edge)"
        );
    }
    Some((s, e))
}

/// Return `(start, end)` byte offsets for each `====` block in the separator
/// row. Whitespace gaps between blocks become column boundaries, and the
/// final block extends to end-of-line to capture trailing-note overflow.
fn separator_columns(line: &str) -> Vec<(usize, usize)> {
    let bytes = line.as_bytes();
    let mut cols = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'=' {
            let start = i;
            while i < bytes.len() && bytes[i] == b'=' {
                i += 1;
            }
            cols.push((start, i));
        } else {
            i += 1;
        }
    }
    // Stretch each column's `end` to the start of the next column so the
    // intervening whitespace belongs to the preceding column when we slice.
    for idx in 0..cols.len().saturating_sub(1) {
        cols[idx].1 = cols[idx + 1].0;
    }
    if let Some(last) = cols.last_mut() {
        last.1 = line.len();
    }
    cols
}

/// PERF-3 / TASK-1112: case-insensitive ASCII substring scan that does not
/// allocate. The previous `n.to_ascii_lowercase().contains(needle)` form
/// allocated a fresh `String` per `UpgradeEntry` note solely to feed a
/// single `.contains()` check; on an active workspace `cargo upgrade
/// --dry-run` emits dozens of rows. Walk `haystack.as_bytes().windows(...)`
/// and compare with `eq_ignore_ascii_case` per window so the check stays
/// allocation-free while preserving ASCII case-insensitive semantics
/// (`needle` is expected to be ASCII at the call site).
fn contains_ascii_ci(haystack: &str, needle: &str) -> bool {
    let n = needle.as_bytes();
    if n.is_empty() {
        return true;
    }
    let h = haystack.as_bytes();
    if h.len() < n.len() {
        return false;
    }
    h.windows(n.len()).any(|w| w.eq_ignore_ascii_case(n))
}

/// Split upgrade entries into compatible and incompatible.
pub fn categorize_upgrades(entries: Vec<UpgradeEntry>) -> UpgradeResult {
    let mut compatible = Vec::new();
    let mut incompatible = Vec::new();

    for entry in entries {
        // TASK-0437: cargo-edit emits the literal token "incompatible" but the
        // wording could grow a suffix (e.g. "incompatible (semver)") in a
        // future release. Use a case-insensitive substring check so a wording
        // drift does not silently misclassify breaking upgrades as compatible.
        // See cargo-edit upgrade output formatting for the source token.
        // PERF-3 / TASK-1112: `contains_ascii_ci` walks bytes in place rather
        // than allocating a per-row lowercase `String`.
        let is_incompatible = entry
            .note
            .as_deref()
            .is_some_and(|n| contains_ascii_ci(n, "incompatible"));
        if is_incompatible {
            incompatible.push(entry);
        } else {
            compatible.push(entry);
        }
    }

    UpgradeResult {
        compatible,
        incompatible,
    }
}

// ── cargo deny parsing ──────────────────────────────────────────────────────

/// Truncate a log line for tracing — operators get enough context to
/// diagnose schema drift without flooding logs with multi-KB cargo-deny
/// diagnostics.
fn truncate_for_log(s: &str) -> String {
    const MAX: usize = 200;
    if s.len() <= MAX {
        s.to_string()
    } else {
        let mut end = MAX;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}

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
///
/// cargo-deny's documented exit contract is:
/// * `0` — clean: no issues found.
/// * `1` — issues: stderr is the JSON diagnostic stream.
/// * `2` — configuration / usage error.
///
/// Any other code (e.g. `101` for a panic, a future code, or `None` for a
/// signal) fails closed — the supply-chain gate must never score an
/// unrecognised exit as a green build. Split out from `run_cargo_deny` so
/// unit tests can cover the exit-code semantics without spawning the binary.
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
                // ERR-7 / SEC-21 / TASK-1160: Debug-format the stderr tail so
                // ANSI / newlines / NULs in cargo-deny text-mode output cannot
                // forge log entries.
                anyhow::bail!(
                    "cargo deny exited with status 1 but stderr decoded zero diagnostics; \
                     refusing to score as clean — likely non-JSON (text-mode) output. \
                     stderr (truncated): {:?}",
                    truncate_for_log(stderr)
                );
            }
            Ok(parsed)
        }
        // SEC-21 / TASK-1250: scrub control bytes by Debug-formatting the
        // stderr tail so registry-served deny.toml diagnostics cannot inject
        // ANSI / newlines into operator-facing anyhow chains.
        Some(2) => anyhow::bail!(
            "cargo deny exited with status 2 (configuration error): {:?}",
            truncate_for_log(stderr.trim())
        ),
        // ERR-7 (TASK-0598): exit_code == None means cargo-deny was killed
        // by a signal (SIGKILL / OOM-killer / parent timeout). Fail loudly
        // so CI does not score a killed run as a green build.
        None => anyhow::bail!(
            "cargo deny terminated by signal (exit_code = None); \
             refusing to treat partial diagnostics as authoritative"
        ),
        // ERR-7 (TASK-0799): unrecognised exit codes (101 panic, future
        // additions, etc.) used to fall through to `parse_deny_output`,
        // which for a non-diagnostic stderr returns an empty `DenyResult`
        // — a fail-open hole in the supply-chain gate. Fail closed instead.
        // ERR-7 / SEC-21 / TASK-1160: Debug-format stderr so panic/abort text
        // (cargo-deny exit 101 etc.) with embedded ANSI cannot repaint logs.
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

/// Decoded view of one `cargo deny --format json` diagnostic line, with the
/// JSON-envelope and field-shape failures already routed to tracing. FN-1
/// (TASK-0793): keeps `parse_deny_output` at the line-level loop while the
/// two-stage decode lives in `decode_diagnostic`.
struct DecodedDiagnostic {
    code: String,
    severity: String,
    message: String,
    advisory: Option<DenyAdvisory>,
    graphs: Option<Vec<DenyGraph>>,
}

/// Parse a single trimmed stderr line. Returns `None` for non-diagnostic
/// envelopes, malformed JSON, unexpected field shapes, or diagnostics
/// without a `code`. Logs every drop reason at `debug` so operators can
/// tell schema drift from a clean run.
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
    // ERR-2 / TASK-0845: previously a missing severity was substituted with
    // the literal "error" sentinel. That collides with the legitimate
    // cargo-deny severity "error" — has_issues fails closed on unknown
    // severities, but had no way to distinguish "explicit error" from
    // "schema drift / cargo-deny stopped emitting severity". Use a
    // distinct sentinel so has_issues routes missing-severity through the
    // explicit fail-closed-and-warn branch and operators see the drift
    // signal in logs instead of silently rating informational diagnostics
    // as actionable errors.
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

/// Resolve the package name for a diagnostic, falling back to the
/// `<no package>` sentinel with a tracing breadcrumb when neither source
/// supplies one. ERR-7 (TASK-0597).
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

/// Append a classified diagnostic to the appropriate `DenyResult` section.
/// FN-1 (TASK-0793): isolates the per-class entry construction so each
/// section's shape lives next to its sibling and not interleaved with
/// dispatch logic.
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
