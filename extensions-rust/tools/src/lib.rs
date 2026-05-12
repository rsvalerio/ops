//! Cargo tools extension: install and manage development tools.
//!
//! Split for cohesion:
//! - [`timeout`] — generic subprocess timeout primitive
//! - [`probe`]   — detect toolchain + installed cargo tools / rustup components
//! - [`install`] — install cargo tools and rustup components via subprocess

mod install;
mod probe;
#[cfg(test)]
mod tests;
mod timeout;

// Re-export tool types from core for convenience of downstream users.
pub use ops_core::config::tools::{ExtendedToolSpec, ToolSource, ToolSpec};

pub use install::{install_cargo_tool, install_rustup_component, install_tool};
pub use probe::{
    capture_cargo_list, capture_path_index, capture_rustup_components, check_binary_installed,
    check_binary_installed_with, check_cargo_tool_installed, check_rustup_component_installed,
    check_tool_status, check_tool_status_with, get_active_toolchain, PathIndex, ProbeOutcome,
};
pub use timeout::{run_with_timeout, DEFAULT_INSTALL_TIMEOUT};

use indexmap::IndexMap;
use ops_extension::ExtensionType;

pub const NAME: &str = "tools";
pub const DESCRIPTION: &str = "Install and manage cargo development tools";
pub const SHORTNAME: &str = "tools";

pub struct ToolsExtension;

ops_extension::impl_extension! {
    ToolsExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::COMMAND,
    stack: Some(ops_extension::Stack::Rust),
    data_provider_name: None,
    register_data_providers: |_self, _registry| {},
    factory: TOOLS_FACTORY = |_, _| {
        Some((NAME, Box::new(ToolsExtension)))
    },
}

/// Status of an installable tool.
///
/// READ-7 / TASK-0896: variants render through [`std::fmt::Display`] so
/// the user-facing string is a deliberate contract — not a `Debug` byproduct
/// that mutates whenever a variant gains a field. CLI consumers fall back
/// to `format!("{}", status)` for unknown variants instead of leaking the
/// `Debug` representation.
///
/// READ-7 / TASK-0992: a previous `Unknown` variant was declared but never
/// constructed — every probe-failure branch in `probe.rs` mapped to
/// `NotInstalled`. The dead variant misled downstream consumers into
/// writing defensive `Unknown` arms that could never fire.
///
/// API / TASK-1200: a distinct [`ToolStatus::ProbeFailed`] variant now
/// surfaces the case where the underlying probe (e.g. `rustup show
/// active-toolchain`, `cargo --list`, `rustup component list`) timed
/// out, failed to spawn, or exited non-zero. Previously every such
/// failure collapsed onto `NotInstalled`, which `tools_cmd::run_install`
/// then "fixed" by reinstalling — turning a transient probe failure
/// into a real mutation against a perfectly working toolchain. CLI
/// install paths filter on `NotInstalled` only, so a `ProbeFailed`
/// entry no longer triggers a reinstall.
///
/// **When adding a variant:** extend the `Display` impl below with an
/// intentional, stable user-facing string before merging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToolStatus {
    Installed,
    NotInstalled,
    /// API / TASK-1200: the probe itself failed (timeout, IO error, or
    /// non-zero exit from `rustup`/`cargo`). The tool's true install
    /// state is unknown; callers must NOT treat this as a missing tool
    /// to install.
    ProbeFailed,
}

impl std::fmt::Display for ToolStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ToolStatus::Installed => "installed",
            ToolStatus::NotInstalled => "not installed",
            ToolStatus::ProbeFailed => "probe failed",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub status: ToolStatus,
    pub has_rustup_component: bool,
}

impl ToolInfo {
    pub fn new(
        name: String,
        description: String,
        status: ToolStatus,
        has_rustup_component: bool,
    ) -> Self {
        Self {
            name,
            description,
            status,
            has_rustup_component,
        }
    }
}

/// TEST-25 / TASK-1295: build a `Vec<ToolInfo>` from a tool spec map using a
/// caller-supplied probe. Tests inject a deterministic probe so the suite
/// does not depend on whether the host (or CI image) happens to have
/// `rustfmt` / `cargo-fmt` installed.
pub fn collect_tools_with(
    tools: &IndexMap<String, ToolSpec>,
    probe: &dyn Fn(&str, &ToolSpec) -> ToolStatus,
) -> Vec<ToolInfo> {
    tools
        .iter()
        .map(|(name, spec)| ToolInfo {
            name: name.clone(),
            description: spec.description().to_string(),
            status: probe(name, spec),
            has_rustup_component: spec.rustup_component().is_some(),
        })
        .collect()
}

/// PATTERN-1 / TASK-1345: probe a single tool without first allocating a
/// single-entry `IndexMap`. The CLI's named-install path (`ops tools install
/// <name>`) consumes the only tool in `config.tools.get(name)` by reference;
/// the prior shape cloned the `ToolSpec` into a one-entry map purely to fit
/// the `collect_tools(&IndexMap)` signature. This helper skips the clone and
/// the global cargo-list / rustup-components / PATH index captures —
/// `check_tool_status` already falls back to per-tool probing when those are
/// `None`, which is the right policy for a single-tool query.
#[must_use]
pub fn collect_tool_one(name: &str, spec: &ToolSpec) -> ToolInfo {
    ToolInfo {
        name: name.to_string(),
        description: spec.description().to_string(),
        status: probe::check_tool_status(name, spec),
        has_rustup_component: spec.rustup_component().is_some(),
    }
}

pub fn collect_tools(tools: &IndexMap<String, ToolSpec>) -> Vec<ToolInfo> {
    let needs_cargo = tools
        .values()
        .any(|s| matches!(s.source(), ToolSource::Cargo));
    let needs_rustup = tools.values().any(|s| s.rustup_component().is_some());
    // PERF-3 / TASK-1046: any Cargo-source spec may fall through to the
    // PATH-binary fallback (standalone `cargo install` binaries that don't
    // appear in `cargo --list`, e.g. tokei/bacon), and System-source specs
    // always check PATH. Capture the index whenever either case applies so
    // the per-tool fallback becomes an O(1) hash lookup instead of an
    // O(|PATH| × |PATHEXT|) walk per tool.
    let needs_path_index = tools
        .values()
        .any(|s| matches!(s.source(), ToolSource::Cargo | ToolSource::System));
    // API / TASK-1200: convert the per-sweep captures from `ProbeOutcome`
    // to `Option<String>` for the existing `check_tool_status_with`
    // contract. A `ProbeOutcome::Failed` here intentionally falls back
    // to per-tool probing (`None`), where the failure surfaces as
    // `ToolStatus::ProbeFailed` for the affected entries instead of
    // collapsing the whole sweep.
    let cargo_list = if needs_cargo {
        match probe::capture_cargo_list() {
            ProbeOutcome::Ok(s) => Some(s),
            ProbeOutcome::Failed => None,
        }
    } else {
        None
    };
    let rustup_components = if needs_rustup {
        match probe::capture_rustup_components() {
            ProbeOutcome::Ok(s) => Some(s),
            ProbeOutcome::Failed => None,
        }
    } else {
        None
    };
    let path_index = if needs_path_index {
        probe::capture_path_index()
    } else {
        None
    };
    tools
        .iter()
        .map(|(name, spec)| {
            let status = probe::check_tool_status_with(
                name,
                spec,
                cargo_list.as_deref(),
                rustup_components.as_deref(),
                path_index.as_ref(),
            );
            ToolInfo {
                name: name.clone(),
                description: spec.description().to_string(),
                status,
                has_rustup_component: spec.rustup_component().is_some(),
            }
        })
        .collect()
}
