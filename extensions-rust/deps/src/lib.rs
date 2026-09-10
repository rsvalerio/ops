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

// The published parse surface is the *guarded* one. Each tool exposes the same
// pair: a `run_cargo_*` entry point that spawns, and an
// `interpret_*_output` / `interpret_*_result` entry point that applies every
// format-drift guard to an already-collected `(exit code, output)` triple.
// The unguarded table slicer is crate-private on purpose — it discards the
// parse diagnostics, so an unrecognised table would read as "no upgrades
// available", and publishing it would make opting out of the crate's
// fail-closed posture the path of least resistance.
pub use parse::{
    categorize_upgrades, interpret_deny_result, interpret_upgrade_output, parse_deny_output,
    run_cargo_deny, run_cargo_upgrade_dry_run,
};

// An explicit re-export list rather than `pub use types::*`, so adding a type
// to `types.rs` is not a public API change by default.
pub use types::{
    AdvisoryEntry, BanEntry, DenyEntry, DenyResult, DepsReport, LicenseEntry, SourceEntry,
    UpgradeEntry, UpgradeResult,
};

/// Extension identifier used to register this crate in the engine's
/// extension registry.
pub const NAME: &str = "deps";
/// One-line description shown by `ops about` for this extension.
pub const DESCRIPTION: &str = "Dependency health: upgrades, advisories, licenses, bans, sources";
/// CLI-facing short name (`deps`) used in commands and user-facing output.
pub const SHORTNAME: &str = "deps";
/// Registry key of the `deps` data provider this crate registers — the key
/// the about dependencies subpage looks the health report up by.
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
/// `check_tool_in`. Bounded because a wedged registry probe, a broken sccache
/// shim, or a sibling cargo holding the `target/` lock would otherwise stall
/// the probe indefinitely. Routed through `run_cargo` so it inherits
/// `OPS_SUBPROCESS_TIMEOUT_SECS` overrides plus the `$CARGO` resolution that
/// keeps nested invocations on the parent toolchain.
const CARGO_TOOL_PROBE_TIMEOUT: Duration = Duration::from_secs(30);

/// Probe for one tool by running its `--version` in `working_dir`.
///
/// The directory is always the caller's, never `Path::new(".")`: a probe
/// resolved against the process CWD agrees with the directory the command
/// actually operates on only by coincidence — see [`ensure_tools`].
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

/// Probe every tool in `REQUIRED_CARGO_TOOLS` from `working_dir`.
///
/// `working_dir` is the directory the command operates on, passed in rather
/// than taken from the process CWD. Cargo resolves its workspace, its
/// `.cargo/config.toml` and its toolchain override from the directory it is
/// spawned in, so a probe run somewhere else answers a question about a
/// different workspace. `DepsProvider::provide` routes
/// `run_cargo_upgrade_dry_run` and `run_cargo_deny` through
/// `ctx.working_directory()`, so passing the same directory here makes the
/// probe agree with them by construction rather than by the coincidence that
/// `ops deps` is normally invoked from the directory it reports on.
///
/// # Errors
///
/// If any tool in `REQUIRED_CARGO_TOOLS` is not installed, naming the tool
/// and the command that installs it.
pub fn ensure_tools(working_dir: &std::path::Path) -> anyhow::Result<()> {
    for tool in REQUIRED_CARGO_TOOLS {
        check_tool_in(tool, working_dir)?;
    }
    Ok(())
}

// ── Public entry point ──────────────────────────────────────────────────────

/// Build a [`Context`] from the user's loaded `.ops.toml`.
///
/// Loads through `ops_core::config::load_config_or_default_at`, so a malformed
/// config file degrades to defaults with a logged warning instead of failing
/// the command outright — the same "tolerate broken config" posture as
/// `cli/main.rs::early_config`.
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
/// `#[non_exhaustive]`, like every other public type this crate exports
/// (`UpgradeEntry`, `DenyResult`, `DepsReport`, `DepsExtension`, …), so the
/// next `ops deps` flag stays an additive change. Construct via
/// [`DepsOptions::new`] or [`DepsOptions::default`]: the attribute also rules
/// out struct-update syntax (`DepsOptions { refresh: true,
/// ..Default::default() }`) from outside this crate — functional record update
/// still needs a struct literal — so `new` is the only way an external caller
/// sets a field.
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
    // The context comes from the same config-loading path as the sibling
    // subcommands (`run_about`, `run_extension_show`), so `[deps]` and global
    // settings apply identically to `ops deps` and `ops about deps`.
    // `refresh` goes through the consuming builder rather than field
    // assignment: it changes cache semantics for every provider that runs on
    // this context afterwards.
    let mut ctx = build_user_context()?;
    if opts.refresh {
        ctx = ctx.with_refresh();
    }

    // Probe the tools in the *context's* directory, so the probe and the
    // collection calls in `DepsProvider::provide` are answered by the same
    // cargo workspace. The context is built first for that reason; it only
    // reads `.ops.toml` and never runs cargo, so it does not need the tools
    // present.
    ensure_tools(ctx.working_directory())?;

    // Resolve the theme + column width from the same config the runner commands
    // use, BEFORE `get_or_provide` borrows `ctx` mutably. `ops deps` renders
    // through the shared theme machinery (`render_report`), so a custom theme
    // restyles it exactly as it restyles `ops verify`.
    let columns = ctx.config().output.resolve_columns();
    let theme = ops_theme::resolve_theme(&ctx.config().output.theme, &ctx.config().themes)
        .map_err(|e| anyhow::anyhow!("deps: {e}"))?;

    // Both calls carry `ops deps` context so a failure never surfaces as a
    // bare serde or registry message. The deserialize one matters most:
    // `get_or_provide` serves a persisted payload when one exists, and
    // `DepsReport` is `#[non_exhaustive]` and still gaining fields — so a
    // cache written by an older `ops` is a live failure mode whose remedy
    // (`--refresh`) the operator cannot guess from `missing field `upgrades``.
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

/// Decide whether one cargo-deny severity string fails the gate.
///
/// The *partition* — which cargo-deny severity strings exist and which of
/// them are benign — lives exactly once, in
/// [`format::SeverityClass::classify`], the same definition the renderer uses
/// for icons, colours and [`ops_core::report::ReportStatus`]. Classifying
/// through it is what keeps the process exit code and the rendered row status
/// from disagreeing: a new severity is one arm there, not two arms in two
/// modules.
///
/// `relax_warning = true` is the bans-only relaxation (cargo-deny emits
/// duplicate-crate diagnostics at `warning` and project policy treats those
/// as informational — "transitive, usually harmless"). It lives here, at
/// the gate, rather than inside the shared classifier: it is a policy of
/// *this gate*, not a fact about the severity string.
///
/// Unknown severities fail closed, mirroring `SeverityClass::Unknown`
/// rendering as `ReportStatus::Error`, and set `warned_unknown` after
/// emitting a single `tracing::warn!`. The caller threads one flag through
/// the whole gate evaluation, so schema drift leaves one breadcrumb per gate
/// run rather than one per finding.
fn severity_is_actionable(severity: &str, relax_warning: bool, warned_unknown: &mut bool) -> bool {
    match format::SeverityClass::classify(severity) {
        format::SeverityClass::Error => true,
        format::SeverityClass::Warning => !relax_warning,
        // Known-benign in cargo-deny output: informational diagnostics
        // that should not fail CI.
        format::SeverityClass::Info => false,
        format::SeverityClass::Unknown => {
            if !*warned_unknown {
                *warned_unknown = true;
                tracing::warn!(
                    severity = %severity,
                    "unknown cargo-deny severity treated as actionable (fail-closed); add it to SeverityClass::classify if it is benign"
                );
            }
            true
        }
    }
}

/// Returns true if the report contains any actionable issues.
///
/// Duplicate crate bans (warnings) are excluded — they are informational.
///
/// Any severity outside the explicitly-known-benign set fails the gate, so a
/// cargo-deny release that adds a severity (`critical`, a renamed `note`, the
/// `<missing-severity>` sentinel `parse_deny_output` substitutes) errs
/// towards failing CI rather than towards a silent pass. The first such
/// severity encountered fires a `tracing::warn!`; the `warned_unknown` flag
/// threaded through the four sections holds it to at most one line per call,
/// matching the per-section guard the report formatter applies per section.
fn has_issues(report: &DepsReport) -> bool {
    let warned_unknown = &mut false;
    report
        .deny
        .advisories
        .iter()
        .any(|e| severity_is_actionable(&e.severity, false, warned_unknown))
        || report
            .deny
            .licenses
            .iter()
            .any(|e| severity_is_actionable(&e.severity, false, warned_unknown))
        || report
            .deny
            .bans
            .iter()
            .any(|e| severity_is_actionable(&e.severity, true, warned_unknown))
        || report
            .deny
            .sources
            .iter()
            .any(|e| severity_is_actionable(&e.severity, false, warned_unknown))
}

// ── Extension + DataProvider ────────────────────────────────────────────────

/// The `deps` extension, constructed via the registered extension factory
/// only.
///
/// `#[non_exhaustive]` keeps a future state field additive at the type
/// level; downstream code that needs a value goes through the
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

/// Data provider assembling the dependency health report by shelling out to
/// `cargo upgrade --dry-run` and `cargo deny check`, served under the
/// [`DATA_PROVIDER_NAME`] key.
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
