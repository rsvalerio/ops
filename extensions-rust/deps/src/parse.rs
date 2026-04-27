//! Parsing logic for `cargo upgrade` and `cargo deny` output.

use crate::{
    AdvisoryEntry, BanEntry, DenyResult, LicenseEntry, SourceEntry, UpgradeEntry, UpgradeResult,
};
use ops_core::subprocess::run_cargo;
use serde::Deserialize;
use std::collections::HashSet;
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
pub fn run_cargo_upgrade_dry_run(working_dir: &Path) -> anyhow::Result<Vec<UpgradeEntry>> {
    let output = run_cargo(
        &["upgrade", "--dry-run"],
        working_dir,
        CARGO_UPGRADE_TIMEOUT,
        "cargo upgrade --dry-run",
    )
    .map_err(|e| anyhow::anyhow!("failed to run cargo upgrade: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_upgrade_table(&stdout))
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
    let mut entries = Vec::new();
    let mut columns: Option<Vec<(usize, usize)>> = None;

    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }

        // Header row resets state but doesn't yet provide the offsets.
        if line.trim_start().starts_with("name") && line.contains("old req") {
            columns = None;
            continue;
        }

        // Separator row: `====   ======= ==========` defines exact byte columns.
        if line.trim_start().starts_with("====") {
            columns = Some(separator_columns(line));
            continue;
        }

        let Some(cols) = columns.as_deref() else {
            continue;
        };

        // Need at least the 5 fixed columns; anything beyond column[4] (incl.
        // any trailing characters past the last `====` block) is the note.
        if cols.len() < 5 {
            continue;
        }

        let take = |idx: usize| -> Option<String> {
            let (start, end) = cols[idx];
            if start >= line.len() {
                return None;
            }
            let slice = &line[start..end.min(line.len())];
            let trimmed = slice.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        };

        // Require all five fixed fields to be present; a row that doesn't
        // reach the `new req` column is an incomplete table line and should
        // be skipped to match prior behavior.
        let (Some(name), Some(old_req), Some(compatible), Some(latest), Some(new_req)) =
            (take(0), take(1), take(2), take(3), take(4))
        else {
            continue;
        };

        // The note absorbs every byte from the start of column 5 to end of
        // line so multi-word notes survive intact. If the upstream format
        // grows extra columns past the note, they roll up here too — at least
        // the five fixed fields stay correctly aligned. When the separator
        // row has no note column at all, there is simply no note.
        let note = cols.get(5).and_then(|(start, _)| {
            if *start >= line.len() {
                return None;
            }
            let slice = line[*start..].trim();
            if slice.is_empty() {
                None
            } else {
                Some(slice.to_string())
            }
        });

        entries.push(UpgradeEntry {
            name,
            old_req,
            compatible,
            latest,
            new_req,
            note,
        });
    }

    entries
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

/// Split upgrade entries into compatible and incompatible.
pub fn categorize_upgrades(entries: Vec<UpgradeEntry>) -> UpgradeResult {
    let mut compatible = Vec::new();
    let mut incompatible = Vec::new();

    for entry in entries {
        if entry.note.as_deref() == Some("incompatible") {
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

/// Advisory-related diagnostic codes.
const ADVISORY_CODES: &[&str] = &[
    "vulnerability",
    "notice",
    "unmaintained",
    "unsound",
    "yanked",
];

/// License-related diagnostic codes.
const LICENSE_CODES: &[&str] = &["rejected", "unlicensed", "no-license-field"];

/// Ban-related diagnostic codes.
const BAN_CODES: &[&str] = &["banned", "not-allowed", "duplicate", "workspace-duplicate"];

/// Source-related diagnostic codes.
const SOURCE_CODES: &[&str] = &["source-not-allowed", "git-source-underspecified"];

/// Run `cargo deny check` and parse the JSON output.
pub fn run_cargo_deny(working_dir: &Path) -> anyhow::Result<DenyResult> {
    let output = run_cargo(
        &["deny", "--format", "json", "check"],
        working_dir,
        CARGO_DENY_TIMEOUT,
        "cargo deny check",
    )
    .map_err(|e| anyhow::anyhow!("failed to run cargo deny: {}", e))?;

    // cargo deny exits non-zero when issues are found — that's expected
    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok(parse_deny_output(&stderr))
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

/// Parse newline-delimited JSON from `cargo deny --format json check` stderr.
pub fn parse_deny_output(stderr: &str) -> DenyResult {
    let advisory_set: HashSet<&str> = ADVISORY_CODES.iter().copied().collect();
    let license_set: HashSet<&str> = LICENSE_CODES.iter().copied().collect();
    let ban_set: HashSet<&str> = BAN_CODES.iter().copied().collect();
    let source_set: HashSet<&str> = SOURCE_CODES.iter().copied().collect();

    let mut result = DenyResult::default();

    for line in stderr.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let deny_line: DenyLine = match serde_json::from_str(trimmed) {
            Ok(l) => l,
            Err(e) => {
                tracing::debug!(
                    error = %e,
                    line = %truncate_for_log(trimmed),
                    "ERR-1: skipping malformed cargo-deny JSON line"
                );
                continue;
            }
        };

        if deny_line.line_type != "diagnostic" {
            continue;
        }

        let fields: DiagnosticFields = match serde_json::from_value(deny_line.fields) {
            Ok(f) => f,
            Err(e) => {
                tracing::debug!(
                    error = %e,
                    line = %truncate_for_log(trimmed),
                    "ERR-1: skipping cargo-deny diagnostic with unexpected fields shape"
                );
                continue;
            }
        };

        let code = match &fields.code {
            Some(c) => c.as_str(),
            None => continue,
        };

        let severity = fields.severity.as_deref().unwrap_or("error").to_string();
        let message = fields.message.clone().unwrap_or_default();

        // Extract package name from graphs or advisory
        let package = fields
            .advisory
            .as_ref()
            .and_then(|a| a.package.clone())
            .or_else(|| {
                fields
                    .graphs
                    .as_ref()
                    .and_then(|g| g.first())
                    .and_then(|g| g.krate.as_ref())
                    .map(|k| k.name.clone())
            })
            .unwrap_or_else(|| "unknown".to_string());

        if advisory_set.contains(code) {
            let (id, title) = if let Some(adv) = &fields.advisory {
                (
                    adv.id.clone(),
                    adv.title.clone().unwrap_or_else(|| message.clone()),
                )
            } else {
                (code.to_string(), message.clone())
            };
            result.advisories.push(AdvisoryEntry {
                id,
                package,
                severity,
                title,
            });
        } else if license_set.contains(code) {
            result.licenses.push(LicenseEntry {
                package,
                message,
                severity,
            });
        } else if ban_set.contains(code) {
            result.bans.push(BanEntry {
                package,
                message,
                severity,
            });
        } else if source_set.contains(code) {
            result.sources.push(SourceEntry {
                package,
                message,
                severity,
            });
        }
    }

    result
}
