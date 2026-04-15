//! Parsing logic for `cargo upgrade` and `cargo deny` output.

use crate::{
    AdvisoryEntry, BanEntry, DenyResult, LicenseEntry, SourceEntry, UpgradeEntry, UpgradeResult,
};
use serde::Deserialize;
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

// ── cargo upgrade parsing ───────────────────────────────────────────────────

/// Run `cargo upgrade --dry-run` and parse the table output.
pub fn run_cargo_upgrade_dry_run(working_dir: &Path) -> anyhow::Result<Vec<UpgradeEntry>> {
    let output = Command::new("cargo")
        .args(["upgrade", "--dry-run"])
        .current_dir(working_dir)
        .output()
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
pub fn parse_upgrade_table(stdout: &str) -> Vec<UpgradeEntry> {
    let mut entries = Vec::new();
    let mut in_table = false;

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Detect header row
        if trimmed.starts_with("name") && trimmed.contains("old req") {
            in_table = true;
            continue;
        }

        // Skip separator row
        if trimmed.starts_with("====") {
            continue;
        }

        if !in_table {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        // Minimum: name old_req compatible latest new_req
        if parts.len() >= 5 {
            let note = if parts.len() >= 6 {
                Some(parts[5..].join(" "))
            } else {
                None
            };
            entries.push(UpgradeEntry {
                name: parts[0].to_string(),
                old_req: parts[1].to_string(),
                compatible: parts[2].to_string(),
                latest: parts[3].to_string(),
                new_req: parts[4].to_string(),
                note,
            });
        }
    }

    entries
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
    let output = Command::new("cargo")
        .args(["deny", "--format", "json", "check"])
        .current_dir(working_dir)
        .output()
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
            Err(_) => continue,
        };

        if deny_line.line_type != "diagnostic" {
            continue;
        }

        let fields: DiagnosticFields = match serde_json::from_value(deny_line.fields) {
            Ok(f) => f,
            Err(_) => continue,
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
