//! Deps extension: comprehensive dependency health report.
//!
//! Combines `cargo upgrade --dry-run` (available upgrades) and `cargo deny check`
//! (advisories, licenses, bans, sources) into a single `ops deps` command.
//!
//! Both `cargo-edit` and `cargo-deny` must be installed.

#[cfg(test)]
mod tests;

use ops_extension::{
    Context, DataField, DataProvider, DataProviderError, DataProviderSchema, ExtensionType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

pub const NAME: &str = "deps";
pub const DESCRIPTION: &str = "Dependency health: upgrades, advisories, licenses, bans, sources";
pub const SHORTNAME: &str = "deps";
pub const DATA_PROVIDER_NAME: &str = "deps";

// ── Data types ──────────────────────────────────────────────────────────────

/// A single available upgrade entry from `cargo upgrade --dry-run`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeEntry {
    pub name: String,
    pub old_req: String,
    pub compatible: String,
    pub latest: String,
    pub new_req: String,
    pub note: Option<String>,
}

/// Parsed result from `cargo upgrade --dry-run`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpgradeResult {
    pub compatible: Vec<UpgradeEntry>,
    pub incompatible: Vec<UpgradeEntry>,
}

/// A single advisory finding from `cargo deny check`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvisoryEntry {
    pub id: String,
    pub package: String,
    pub severity: String,
    pub title: String,
}

/// A single license issue from `cargo deny check`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseEntry {
    pub package: String,
    pub message: String,
    pub severity: String,
}

/// A single ban issue from `cargo deny check`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BanEntry {
    pub package: String,
    pub message: String,
    pub severity: String,
}

/// A single source issue from `cargo deny check`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEntry {
    pub package: String,
    pub message: String,
    pub severity: String,
}

/// Combined result from `cargo deny check`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DenyResult {
    pub advisories: Vec<AdvisoryEntry>,
    pub licenses: Vec<LicenseEntry>,
    pub bans: Vec<BanEntry>,
    pub sources: Vec<SourceEntry>,
}

/// Full dependency health report.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DepsReport {
    pub upgrades: UpgradeResult,
    pub deny: DenyResult,
}

// ── Tool detection ──────────────────────────────────────────────────────────

fn check_tool(tool: &str, args: &[&str]) -> anyhow::Result<()> {
    Command::new("cargo")
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| anyhow::anyhow!("failed to run cargo {}: {}", tool, e))
        .and_then(|s| {
            if s.success() {
                Ok(())
            } else {
                anyhow::bail!(
                    "cargo {} is not installed. Install with: cargo install {}",
                    tool,
                    if tool == "upgrade" {
                        "cargo-edit"
                    } else {
                        "cargo-deny"
                    }
                )
            }
        })
}

pub fn ensure_tools() -> anyhow::Result<()> {
    check_tool("upgrade", &["upgrade", "--version"])?;
    check_tool("deny", &["deny", "--version"])?;
    Ok(())
}

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

// ── Report formatting ───────────────────────────────────────────────────────

use ops_core::style::{bold, dim, green, red, yellow};

const P: &str = "  "; // left padding for the entire report

fn severity_icon(severity: &str) -> &'static str {
    match severity {
        "error" => "\u{2718}",   // ✘
        "warning" => "\u{26a0}", // ⚠
        _ => "\u{2139}",         // ℹ
    }
}

fn colorize_severity(text: &str, severity: &str) -> String {
    match severity {
        "error" => red(text),
        "warning" => yellow(text),
        _ => dim(text),
    }
}

pub fn format_report(report: &DepsReport) -> String {
    let mut out = String::new();

    out.push_str(&format!("\n{P}{}\n\n", bold("Dependency Health Report")));

    // Compatible upgrades
    format_upgrade_section(
        &mut out,
        "\u{2b06}\u{fe0f} Compatible Upgrades",
        &report.upgrades.compatible,
        false,
    );

    // Breaking upgrades
    format_upgrade_section(
        &mut out,
        "\u{1f4a5} Breaking Upgrades",
        &report.upgrades.incompatible,
        true,
    );

    // Advisories
    format_advisories(&mut out, &report.deny.advisories);

    // License issues
    format_deny_section(
        &mut out,
        "\u{1f4dc} License Issues",
        &report.deny.licenses,
        |l| (&l.package, &l.message, &l.severity),
        "Run `cargo deny check licenses` for details. Configure allowed licenses in deny.toml.",
    );

    // Duplicate crates (bans) — totals only
    format_bans_summary(&mut out, &report.deny.bans);

    // Source issues
    format_deny_section(
        &mut out,
        "\u{1f310} Source Issues",
        &report.deny.sources,
        |s| (&s.package, &s.message, &s.severity),
        "Configure trusted sources in deny.toml [sources] section.",
    );

    out
}

fn format_upgrade_section(
    out: &mut String,
    title: &str,
    entries: &[UpgradeEntry],
    is_breaking: bool,
) {
    if entries.is_empty() {
        out.push_str(&format!("{P}{} {}\n\n", title, green("\u{2714} None")));
    } else {
        out.push_str(&format!("{P}{} ({}):\n", title, entries.len()));
        let name_width = entries.iter().map(|e| e.name.len()).max().unwrap_or(0);
        let old_width = entries.iter().map(|e| e.old_req.len()).max().unwrap_or(0);
        for e in entries {
            out.push_str(&format!(
                "{P}    {:<name_w$}  {}  {}  {}\n",
                e.name,
                dim(&format!("{:<old_w$}", e.old_req, old_w = old_width)),
                dim("->"),
                green(&e.new_req),
                name_w = name_width,
            ));
        }
        out.push('\n');
        let advice = if is_breaking {
            "Run `cargo upgrade --incompatible` to apply breaking upgrades."
        } else {
            "Run `cargo upgrade` to apply compatible upgrades."
        };
        out.push_str(&format!("{P}    {} {}\n\n", dim("\u{1f4a1}"), dim(advice)));
    }
}

fn format_advisories(out: &mut String, advisories: &[AdvisoryEntry]) {
    if advisories.is_empty() {
        out.push_str(&format!(
            "{P}\u{1f6e1}\u{fe0f} Advisories {}\n\n",
            green("\u{2714} None")
        ));
    } else {
        out.push_str(&format!(
            "{P}\u{1f6e1}\u{fe0f} Advisories ({}):\n",
            advisories.len()
        ));
        let id_width = advisories.iter().map(|a| a.id.len()).max().unwrap_or(0);
        let pkg_width = advisories
            .iter()
            .map(|a| a.package.len())
            .max()
            .unwrap_or(0);
        for a in advisories {
            let icon = severity_icon(&a.severity);
            out.push_str(&format!(
                "{P}    {} {:<id_w$}  {:<pkg_w$}  {}\n",
                colorize_severity(icon, &a.severity),
                a.id,
                a.package,
                dim(&a.title),
                id_w = id_width,
                pkg_w = pkg_width,
            ));
        }
        out.push('\n');
        out.push_str(&format!(
            "{P}    {} {}\n\n",
            dim("\u{1f4a1}"),
            dim("Run `cargo deny check advisories` for details. Update affected crates or add exceptions to deny.toml.")
        ));
    }
}

fn format_bans_summary(out: &mut String, bans: &[BanEntry]) {
    let title = "\u{1f4e6} Duplicate Crates";
    if bans.is_empty() {
        out.push_str(&format!("{P}{} {}\n\n", title, green("\u{2714} None")));
    } else {
        let errors = bans.iter().filter(|b| b.severity == "error").count();
        let warnings = bans.iter().filter(|b| b.severity == "warning").count();
        let others = bans.len() - errors - warnings;

        let mut parts = Vec::new();
        if errors > 0 {
            parts.push(red(&format!(
                "{} error{}",
                errors,
                if errors == 1 { "" } else { "s" }
            )));
        }
        if warnings > 0 {
            parts.push(yellow(&format!(
                "{} warning{}",
                warnings,
                if warnings == 1 { "" } else { "s" }
            )));
        }
        if others > 0 {
            parts.push(dim(&format!("{} info", others)));
        }

        out.push_str(&format!(
            "{P}{} ({}): {}\n\n",
            title,
            bans.len(),
            parts.join(", ")
        ));

        let advice = "Duplicate versions are common in transitive dependencies and usually harmless.\nThey add binary size but rarely cause issues. Run `cargo update` to try to reduce them.";
        for line in advice.lines() {
            out.push_str(&format!("{P}    {} {}\n", dim("\u{1f4a1}"), dim(line)));
        }
        out.push('\n');
    }
}

fn format_deny_section<T, F>(out: &mut String, title: &str, entries: &[T], extract: F, advice: &str)
where
    F: Fn(&T) -> (&String, &String, &String),
{
    if entries.is_empty() {
        out.push_str(&format!("{P}{} {}\n\n", title, green("\u{2714} None")));
    } else {
        out.push_str(&format!("{P}{} ({}):\n", title, entries.len()));
        let pkg_width = entries
            .iter()
            .map(|e| extract(e).0.len())
            .max()
            .unwrap_or(0);
        for e in entries {
            let (pkg, msg, sev) = extract(e);
            let icon = severity_icon(sev);
            out.push_str(&format!(
                "{P}    {} {:<pkg_w$}  {}\n",
                colorize_severity(icon, sev),
                pkg,
                dim(msg),
                pkg_w = pkg_width,
            ));
        }
        out.push('\n');
        for line in advice.lines() {
            out.push_str(&format!("{P}    {} {}\n", dim("\u{1f4a1}"), dim(line)));
        }
        out.push('\n');
    }
}

// ── Public entry point ──────────────────────────────────────────────────────

/// Options for the deps command.
pub struct DepsOptions {
    pub refresh: bool,
}

/// Run the deps command: check tool availability, collect data, print report.
pub fn run_deps(
    data_registry: &ops_extension::DataRegistry,
    opts: &DepsOptions,
) -> anyhow::Result<()> {
    ensure_tools()?;

    let cwd = std::env::current_dir()?;
    let config = std::sync::Arc::new(ops_core::config::Config::default());
    let mut ctx = Context::new(config, cwd);
    if opts.refresh {
        ctx.refresh = true;
    }

    let value = ctx.get_or_provide(DATA_PROVIDER_NAME, data_registry)?;
    let report: DepsReport = serde_json::from_value((*value).clone())?;

    print!("{}", format_report(&report));

    if has_issues(&report) {
        anyhow::bail!("dependency issues found");
    }

    Ok(())
}

/// Returns true if the report contains any error- or warning-level issues.
fn has_issues(report: &DepsReport) -> bool {
    let is_actionable = |s: &str| matches!(s, "error" | "warning");

    report.deny.advisories.iter().any(|e| is_actionable(&e.severity))
        || report.deny.licenses.iter().any(|e| is_actionable(&e.severity))
        || report.deny.bans.iter().any(|e| is_actionable(&e.severity))
        || report.deny.sources.iter().any(|e| is_actionable(&e.severity))
}

// ── Extension + DataProvider ────────────────────────────────────────────────

pub struct DepsExtension;

ops_extension::impl_extension! {
    DepsExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE | ExtensionType::COMMAND,
    command_names: &["deps"],
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_commands: |_self, registry| {
        registry.insert(
            "deps".to_string(),
            ops_core::config::CommandSpec::Exec(ops_core::config::ExecCommandSpec {
                program: "ops".to_string(),
                args: vec!["deps".to_string()],
                ..Default::default()
            }),
        );
    },
    register_data_providers: |_self, registry| {
        registry.register(DATA_PROVIDER_NAME, Box::new(DepsProvider));
    },
    factory: DEPS_FACTORY = |_, _| {
        Some((NAME, Box::new(DepsExtension)))
    },
}

pub struct DepsProvider;

impl DataProvider for DepsProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let upgrade_entries = run_cargo_upgrade_dry_run(&ctx.working_directory)
            .map_err(|e| DataProviderError::from(anyhow::anyhow!("cargo upgrade failed: {}", e)))?;

        let upgrades = categorize_upgrades(upgrade_entries);

        let deny = run_cargo_deny(&ctx.working_directory)
            .map_err(|e| DataProviderError::from(anyhow::anyhow!("cargo deny failed: {}", e)))?;

        let report = DepsReport { upgrades, deny };
        serde_json::to_value(&report).map_err(DataProviderError::from)
    }

    fn schema(&self) -> DataProviderSchema {
        DataProviderSchema {
            description: "Dependency health: upgrades, advisories, licenses, bans, sources",
            fields: vec![
                DataField {
                    name: "upgrades.compatible",
                    type_name: "Vec<UpgradeEntry>",
                    description: "Semver-compatible upgrades available",
                },
                DataField {
                    name: "upgrades.incompatible",
                    type_name: "Vec<UpgradeEntry>",
                    description: "Breaking (incompatible) upgrades available",
                },
                DataField {
                    name: "deny.advisories",
                    type_name: "Vec<AdvisoryEntry>",
                    description: "Security advisories from RustSec",
                },
                DataField {
                    name: "deny.licenses",
                    type_name: "Vec<LicenseEntry>",
                    description: "License compliance issues",
                },
                DataField {
                    name: "deny.bans",
                    type_name: "Vec<BanEntry>",
                    description: "Banned or duplicate crate issues",
                },
                DataField {
                    name: "deny.sources",
                    type_name: "Vec<SourceEntry>",
                    description: "Source trust issues",
                },
            ],
        }
    }
}
