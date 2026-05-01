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
    capture_cargo_list, capture_rustup_components, check_binary_installed,
    check_cargo_tool_installed, check_rustup_component_installed, check_tool_status,
    check_tool_status_with, get_active_toolchain,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToolStatus {
    Installed,
    NotInstalled,
    #[allow(dead_code)]
    Unknown,
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

pub fn collect_tools(tools: &IndexMap<String, ToolSpec>) -> Vec<ToolInfo> {
    let needs_cargo = tools
        .values()
        .any(|s| matches!(s.source(), ToolSource::Cargo));
    let needs_rustup = tools.values().any(|s| s.rustup_component().is_some());
    let cargo_list = if needs_cargo {
        probe::capture_cargo_list()
    } else {
        None
    };
    let rustup_components = if needs_rustup {
        probe::capture_rustup_components()
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
