//! Deps extension: comprehensive dependency health report.
//!
//! Combines `cargo upgrade --dry-run` (available upgrades) and `cargo deny check`
//! (advisories, licenses, bans, sources) into a single `ops deps` command.
//!
//! Both `cargo-edit` and `cargo-deny` must be installed.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]

mod format;
mod parse;
#[cfg(test)]
pub(crate) mod test_support;
#[cfg(test)]
mod tests;
mod types;

use anyhow::Context as _;
use ops_core::subprocess::{run_cargo, RunError};
use ops_extension::{
    Context, DataField, DataProvider, DataProviderError, DataProviderSchema, ExtensionType,
};
use std::time::Duration;

pub use format::build_report;

// ARCH-9 / TASK-1846: the published parse surface is the *guarded* one. Both
// tools now expose the same pair — a `run_cargo_*` entry point that spawns,
// and an `interpret_*_output` / `interpret_*_result` entry point that applies
// every drift guard to an already-collected `(exit code, output)` triple.
// `parse_upgrade_table` used to sit here too: it is
// `parse_upgrade_table_inner(stdout).0`, so it *discards*
// `UpgradeParseDiagnostics` and thereby bypasses the header-drift (TASK-1074),
// row-shape-drift (TASK-1202) and missing-separator (TASK-1817) guards this
// crate spent four tasks building. Exporting the obvious-looking name for the
// one function that cannot fail and cannot warn made opting out of the whole
// fail-closed posture the path of least resistance, so it is now crate-private.
pub use parse::{
    categorize_upgrades, interpret_deny_result, interpret_upgrade_output, parse_deny_output,
    run_cargo_deny, run_cargo_upgrade_dry_run,
};

// ARCH-4 / TASK-1846: an explicit re-export list, not `pub use types::*`.
// The glob made every type added to `types.rs` a public API change by
// default, including ones meant as internals.
pub use types::{
    AdvisoryEntry, BanEntry, DenyEntry, DenyResult, DepsReport, LicenseEntry, SourceEntry,
    UpgradeEntry, UpgradeResult,
};

pub const NAME: &str = "deps";
pub const DESCRIPTION: &str = "Dependency health: upgrades, advisories, licenses, bans, sources";
pub const SHORTNAME: &str = "deps";
pub const DATA_PROVIDER_NAME: &str = "deps";

// ── Tool detection ──────────────────────────────────────────────────────────

/// A cargo subcommand we depend on, paired with the install package name and
/// the args used to probe for its presence.
pub(crate) struct CargoTool {
    /// Cargo subcommand (e.g. `"upgrade"`, `"deny"`).
    pub(crate) subcommand: &'static str,
    /// Crate to suggest in the install hint (e.g. `"cargo-edit"`).
    pub(crate) install_crate: &'static str,
    /// Args to spawn for the probe. First element is typically `subcommand`.
    pub(crate) probe_args: &'static [&'static str],
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

/// Default timeout for the `cargo <sub> --version` probe spawned by
/// `check_tool`. ASYNC-6 (TASK-0791): a wedged registry probe, broken sccache
/// shim, or sibling cargo holding `target/` lock could otherwise stall the
/// probe indefinitely. Routed through `run_cargo` so it inherits
/// `OPS_SUBPROCESS_TIMEOUT_SECS` overrides plus the `$CARGO` resolution that
/// keeps nested invocations on the parent toolchain.
const CARGO_TOOL_PROBE_TIMEOUT: Duration = Duration::from_secs(30);

fn check_tool(tool: &CargoTool) -> anyhow::Result<()> {
    check_tool_in(tool, std::path::Path::new("."))
}

pub(crate) fn check_tool_in(tool: &CargoTool, working_dir: &std::path::Path) -> anyhow::Result<()> {
    match run_cargo(
        tool.probe_args,
        working_dir,
        CARGO_TOOL_PROBE_TIMEOUT,
        &format!("cargo {} --version", tool.subcommand),
    ) {
        Ok(output) if output.status.success() => Ok(()),
        Ok(_) => anyhow::bail!(
            "cargo {} is not installed. Install with: cargo install {}",
            tool.subcommand,
            tool.install_crate
        ),
        Err(RunError::Timeout(t)) => anyhow::bail!(
            "cargo {} probe timed out after {}s; the cargo registry, an sccache wrapper, \
             or a sibling cargo build holding the target lock may be wedged",
            tool.subcommand,
            t.timeout.as_secs()
        ),
        Err(RunError::Io(e)) => {
            anyhow::bail!("failed to run cargo {}: {}", tool.subcommand, e)
        }
        Err(other) => anyhow::bail!("cargo {} probe failed: {}", tool.subcommand, other),
    }
}

/// # Errors
///
/// If any tool in `REQUIRED_CARGO_TOOLS` is not installed, naming the tool
/// and the command that installs it.
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
///
/// # Errors
///
/// If the current working directory cannot be determined.
pub fn build_user_context() -> anyhow::Result<Context> {
    let cwd =
        std::env::current_dir().context("deps: failed to determine current working directory")?;
    let config = ops_core::config::load_config_or_default_at(&cwd, "deps");
    Ok(Context::new(std::sync::Arc::new(config), cwd))
}

/// Options for the deps command.
///
/// API-9 / TASK-1850: `#[non_exhaustive]` matches every other public type
/// this crate exports (`UpgradeEntry`, `DenyResult`, `DepsReport`,
/// `DepsExtension`, …) and keeps the next `ops deps` flag additive. This is
/// the options bag most likely to grow, and `#[non_exhaustive]` cannot be
/// added once external exhaustive-literal construction sites exist without
/// breaking them — so the cost of the attribute only goes up with time.
/// Construct via [`DepsOptions::new`] or [`Default`] plus struct-update
/// syntax.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct DepsOptions {
    /// Re-collect dependency data instead of serving the payload persisted
    /// in the data cache. Wired to `ops deps --refresh`.
    pub refresh: bool,
}

impl DepsOptions {
    /// Options with `refresh` set as given and every other field defaulted.
    #[must_use]
    pub const fn new(refresh: bool) -> Self {
        Self { refresh }
    }
}

/// Run the deps command: check tool availability, collect data, print report.
///
/// # Errors
///
/// If a required cargo tool is missing, if `cargo deny` / `cargo upgrade`
/// cannot be run or returns output that fails to parse, or if writing the
/// report fails.
pub fn run_deps(
    data_registry: &ops_extension::DataRegistry,
    opts: &DepsOptions,
) -> anyhow::Result<()> {
    ensure_tools()?;

    // ERR-4 / TASK-0405: route through the same config-loading path as
    // sibling subcommands (`run_about`, `run_extension_show`). Previously
    // this constructed `Config::empty()`, so any `[deps]`/global settings
    // that happen to be added to `Config` would silently no-op for `ops
    // deps` while working for `ops about deps`.
    // ARCH-9 / TASK-1874: `refresh` is set through the consuming builder, not
    // by assignment — it changes cache semantics for every provider that runs
    // on this context afterwards.
    let mut ctx = build_user_context()?;
    if opts.refresh {
        ctx = ctx.with_refresh();
    }

    // Resolve the theme + column width from the same config the runner commands
    // use, BEFORE `get_or_provide` borrows `ctx` mutably. `ops deps` now renders
    // through the shared theme machinery (`render_report`) instead of hand-rolled
    // `println!`, so a custom theme restyles it exactly as it restyles `ops verify`.
    let columns = ctx.config().output.resolve_columns();
    let theme = ops_theme::resolve_theme(&ctx.config().output.theme, &ctx.config().themes)
        .map_err(|e| anyhow::anyhow!("deps: {e}"))?;

    // ERR-4 / TASK-1827: both of these were the crate's only bare `?`s, so a
    // failure surfaced as serde's or the registry's own message with nothing
    // naming `ops deps`. The deserialize one matters most: `get_or_provide`
    // serves a *previously persisted* payload when one exists, and
    // `DepsReport` is `#[non_exhaustive]` and still growing fields — so a
    // cache written by an older `ops` is a live failure mode whose remedy
    // (`--refresh`) the operator cannot guess from `missing field
    // `upgrades``.
    let value = ctx
        .get_or_provide(DATA_PROVIDER_NAME, data_registry)
        .with_context(|| {
            format!("deps: the `{DATA_PROVIDER_NAME}` data provider failed to produce a report")
        })?;
    let report: DepsReport = serde_json::from_value(std::sync::Arc::unwrap_or_clone(value))
        .context(
            "deps: failed to decode the dependency report payload; it may have been written to \
             the data cache by an older `ops`. Re-run with `ops deps --refresh` to discard the \
             cached payload and re-collect it",
        )?;

    for line in theme.render_report(&build_report(&report), columns) {
        println!("{line}");
    }

    if has_issues(&report) {
        anyhow::bail!("dependency issues found");
    }

    Ok(())
}

/// DUP-3 / TASK-0989, TASK-1821: the crate's single severity-classifier
/// feeds both gates. The *partition* — which cargo-deny severity strings
/// exist and which of them are benign — lives exactly once, in
/// [`format::SeverityClass::classify`], the same definition the renderer
/// uses for icons, colours and [`ops_core::report::ReportStatus`]. Encoding
/// it twice meant a drift between the two copies was undetectable from
/// either side: each module's own tests still passed while the visible row
/// status contradicted the process exit code.
///
/// `relax_warning = true` is the bans-only relaxation (cargo-deny emits
/// duplicate-crate diagnostics at `warning` and project policy treats those
/// as informational — "transitive, usually harmless"). It stays here, at
/// the call site, rather than inside the shared classifier: it is a policy
/// of *this gate*, not a fact about the severity string.
///
/// ERR-2 (TASK-0601): unknown severities fail closed and fire a
/// `tracing::warn!` so schema drift surfaces in logs without skipping the
/// gate — mirroring `SeverityClass::Unknown` rendering as
/// `ReportStatus::Error`.
fn severity_is_actionable(severity: &str, relax_warning: bool) -> bool {
    match format::SeverityClass::classify(severity) {
        format::SeverityClass::Error => true,
        format::SeverityClass::Warning => !relax_warning,
        // Known-benign in cargo-deny output: informational diagnostics
        // that should not fail CI.
        format::SeverityClass::Info => false,
        format::SeverityClass::Unknown => {
            tracing::warn!(
                severity = %severity,
                "TASK-0601: unknown cargo-deny severity treated as actionable (fail-closed); update SeverityClass::classify if this is benign"
            );
            true
        }
    }
}

/// Returns true if the report contains any actionable issues.
///
/// Duplicate crate bans (warnings) are excluded — they are informational.
///
/// ERR-2 (TASK-0601): fail-closed for unknown severities. Previously the
/// allowlist `matches!(s, "error" | "warning")` silently treated any
/// future cargo-deny severity (`help`, `note`, a hypothetical `critical`)
/// as non-actionable, exactly inverting the desired safety property.
/// Combined with `parse_deny_output` defaulting a missing severity field
/// to `error`, the prior code treated explicit-but-unknown severities as
/// benign while treating absent severities as failures — backwards. Now
/// any severity outside the explicitly-known-benign set fails the gate;
/// unknown severities fire a one-off `tracing::warn!` so schema drift
/// surfaces in logs without skipping the gate.
fn has_issues(report: &DepsReport) -> bool {
    report
        .deny
        .advisories
        .iter()
        .any(|e| severity_is_actionable(&e.severity, false))
        || report
            .deny
            .licenses
            .iter()
            .any(|e| severity_is_actionable(&e.severity, false))
        || report
            .deny
            .bans
            .iter()
            .any(|e| severity_is_actionable(&e.severity, true))
        || report
            .deny
            .sources
            .iter()
            .any(|e| severity_is_actionable(&e.severity, false))
}

// ── Extension + DataProvider ────────────────────────────────────────────────

/// API-9 / TASK-0922: construct via the registered extension factory only.
///
/// `#[non_exhaustive]` keeps a future state field additive at the type
/// level; downstream code that needs a value must go through the
/// `ExtensionFactory` registration path.
#[non_exhaustive]
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
        let _ = registry.register(DATA_PROVIDER_NAME, Box::new(DepsProvider));
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
        let upgrade_entries = run_cargo_upgrade_dry_run(ctx.working_directory())
            .context("cargo upgrade failed")
            .map_err(DataProviderError::from)?;

        let upgrades = categorize_upgrades(upgrade_entries);

        let deny = run_cargo_deny(ctx.working_directory())
            .context("cargo deny failed")
            .map_err(DataProviderError::from)?;

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
