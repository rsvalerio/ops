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
    categorize_upgrades, interpret_deny_result, parse_deny_output, parse_upgrade_table,
    run_cargo_deny, run_cargo_upgrade_dry_run,
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
#[must_use = "UpgradeResult carries compatible/incompatible upgrade entries — silently dropping it loses the parsed report"]
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
#[must_use = "DenyResult carries advisory/license/ban/source findings — silently dropping it hides cargo-deny output"]
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

/// A cargo subcommand we depend on, paired with the install package name and
/// the args used to probe for its presence.
struct CargoTool {
    /// Cargo subcommand (e.g. `"upgrade"`, `"deny"`).
    subcommand: &'static str,
    /// Crate to suggest in the install hint (e.g. `"cargo-edit"`).
    install_crate: &'static str,
    /// Args to spawn for the probe. First element is typically `subcommand`.
    probe_args: &'static [&'static str],
}

const REQUIRED_CARGO_TOOLS: &[CargoTool] = &[
    CargoTool {
        subcommand: "upgrade",
        install_crate: "cargo-edit",
        probe_args: &["upgrade", "--version"],
    },
    CargoTool {
        subcommand: "deny",
        install_crate: "cargo-deny",
        probe_args: &["deny", "--version"],
    },
];

fn check_tool(tool: &CargoTool) -> anyhow::Result<()> {
    let status = Command::new("cargo")
        .args(tool.probe_args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| anyhow::anyhow!("failed to run cargo {}: {}", tool.subcommand, e))?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!(
            "cargo {} is not installed. Install with: cargo install {}",
            tool.subcommand,
            tool.install_crate
        )
    }
}

pub fn ensure_tools() -> anyhow::Result<()> {
    for tool in REQUIRED_CARGO_TOOLS {
        check_tool(tool)?;
    }
    Ok(())
}

// ── Public entry point ──────────────────────────────────────────────────────

/// Build a [`Context`] using the user's loaded `.ops.toml` (TASK-0405).
///
/// Uses [`ops_core::config::load_config_or_default`] so a malformed
/// config file degrades to defaults with a logged warning instead of
/// failing the command outright — matches the "tolerate broken config"
/// posture of `cli/main.rs::early_config`.
pub fn build_user_context() -> anyhow::Result<Context> {
    let config = ops_core::config::load_config_or_default("deps");
    let cwd = std::env::current_dir()?;
    Ok(Context::new(std::sync::Arc::new(config), cwd))
}

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

    // ERR-4 / TASK-0405: route through the same config-loading path as
    // sibling subcommands (`run_about`, `run_extension_show`). Previously
    // this constructed `Config::default()`, so any `[deps]`/global settings
    // that happen to be added to `Config` would silently no-op for `ops
    // deps` while working for `ops about deps`.
    let mut ctx = build_user_context()?;
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
            ops_core::config::CommandSpec::Exec(
                ops_core::config::ExecCommandSpec::new("ops", ["deps"]),
            ),
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
        DataProviderSchema::new(
            "Dependency health: upgrades, advisories, licenses, bans, sources",
            vec![
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
        )
    }
}
