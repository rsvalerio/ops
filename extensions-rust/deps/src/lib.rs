//! Deps extension: comprehensive dependency health report.
//!
//! Combines `cargo upgrade --dry-run` (available upgrades) and `cargo deny check`
//! (advisories, licenses, bans, sources) into a single `ops deps` command.
//!
//! Both `cargo-edit` and `cargo-deny` must be installed.

mod format;
mod parse;
#[cfg(test)]
mod tests;

use ops_extension::{
    Context, DataField, DataProvider, DataProviderError, DataProviderSchema, ExtensionType,
};
use serde::{Deserialize, Serialize};
use std::process::Command;

pub use format::format_report;
pub use parse::{
    categorize_upgrades, parse_deny_output, parse_upgrade_table, run_cargo_deny,
    run_cargo_upgrade_dry_run,
};

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

/// A single issue (license, ban, or source) from `cargo deny check`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DenyEntry {
    pub package: String,
    pub message: String,
    pub severity: String,
}

/// Backwards-compatible type aliases.
pub type LicenseEntry = DenyEntry;
pub type BanEntry = DenyEntry;
pub type SourceEntry = DenyEntry;

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
/// Duplicate crate bans (warnings) are excluded — they are informational.
fn has_issues(report: &DepsReport) -> bool {
    let is_actionable = |s: &str| matches!(s, "error" | "warning");

    report
        .deny
        .advisories
        .iter()
        .any(|e| is_actionable(&e.severity))
        || report
            .deny
            .licenses
            .iter()
            .any(|e| is_actionable(&e.severity))
        || report.deny.bans.iter().any(|e| e.severity == "error")
        || report
            .deny
            .sources
            .iter()
            .any(|e| is_actionable(&e.severity))
}

// ── Extension + DataProvider ────────────────────────────────────────────────

pub struct DepsExtension;

ops_extension::impl_extension! {
    DepsExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE | ExtensionType::COMMAND,
    stack: Some(ops_extension::Stack::Rust),
    command_names: &["deps"],
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_commands: |_self, registry| {
        registry.insert(
            "deps".to_string().into(),
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
                DataField::new(
                    "upgrades.compatible",
                    "Vec<UpgradeEntry>",
                    "Semver-compatible upgrades available",
                ),
                DataField::new(
                    "upgrades.incompatible",
                    "Vec<UpgradeEntry>",
                    "Breaking (incompatible) upgrades available",
                ),
                DataField::new(
                    "deny.advisories",
                    "Vec<AdvisoryEntry>",
                    "Security advisories from RustSec",
                ),
                DataField::new(
                    "deny.licenses",
                    "Vec<LicenseEntry>",
                    "License compliance issues",
                ),
                DataField::new(
                    "deny.bans",
                    "Vec<BanEntry>",
                    "Banned or duplicate crate issues",
                ),
                DataField::new("deny.sources", "Vec<SourceEntry>", "Source trust issues"),
            ],
        }
    }
}
