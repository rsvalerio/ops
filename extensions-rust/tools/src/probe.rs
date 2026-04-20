//! Detect whether tools/components are installed on the active toolchain.

use ops_core::config::tools::{ToolSource, ToolSpec};
use std::process::Command;

use crate::ToolStatus;

pub fn get_active_toolchain() -> Option<String> {
    let output = Command::new("rustup")
        .args(["show", "active-toolchain"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_active_toolchain(&stdout)
}

pub(crate) fn parse_active_toolchain(stdout: &str) -> Option<String> {
    let line = stdout.lines().next()?;
    let parts: Vec<&str> = line.split_whitespace().collect();
    parts.first().map(|s| s.to_string())
}

pub fn check_cargo_tool_installed(name: &str) -> bool {
    let output = match Command::new("cargo").args(["--list"]).output() {
        Ok(o) => o,
        Err(_) => return false,
    };

    if !output.status.success() {
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Standalone binaries installed via `cargo install` (e.g. tokei) don't
    // appear in `cargo --list` — fall back to checking PATH.
    is_in_cargo_list(&stdout, name) || check_binary_installed(name)
}

pub(crate) fn is_in_cargo_list(stdout: &str, name: &str) -> bool {
    let cargo_name = name.strip_prefix("cargo-").unwrap_or(name);
    stdout.lines().any(|line| {
        line.split_whitespace()
            .next()
            .is_some_and(|cmd| cmd == cargo_name)
    })
}

pub fn check_binary_installed(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .is_ok_and(|o| o.status.success())
}

pub fn check_rustup_component_installed(component: &str) -> bool {
    let output = match Command::new("rustup")
        .args(["component", "list", "--installed"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return false,
    };

    if !output.status.success() {
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    is_component_in_list(&stdout, component)
}

pub(crate) fn is_component_in_list(stdout: &str, component: &str) -> bool {
    let base_name = component.strip_suffix("-preview").unwrap_or(component);
    stdout
        .lines()
        .any(|line| line.trim().starts_with(&format!("{}-", base_name)) || line.trim() == base_name)
}

pub fn check_tool_status(name: &str, spec: &ToolSpec) -> ToolStatus {
    if let Some(component) = spec.rustup_component() {
        if !check_rustup_component_installed(component) {
            return ToolStatus::NotInstalled;
        }
    }

    let is_installed = match spec.source() {
        ToolSource::Cargo => check_cargo_tool_installed(name),
        ToolSource::System => check_binary_installed(name),
    };

    if is_installed {
        ToolStatus::Installed
    } else {
        ToolStatus::NotInstalled
    }
}
