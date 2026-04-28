//! Install cargo tools and rustup components via subprocess with a timeout.

use anyhow::Context;
use ops_core::config::tools::{ToolSource, ToolSpec};
use std::process::Command;
use std::time::Duration;

use crate::probe::get_active_toolchain;
use crate::timeout::{run_with_timeout, DEFAULT_INSTALL_TIMEOUT};

pub fn install_cargo_tool(name: &str, package: Option<&str>) -> anyhow::Result<()> {
    install_cargo_tool_with_timeout(name, package, DEFAULT_INSTALL_TIMEOUT)
}

/// SEC-13: defense-in-depth validation for crate names that flow into
/// `cargo install`. `Command::args` already prevents shell interpolation, but
/// the values still land as positional arguments to a privileged operation —
/// a name beginning with `-` would be parsed as a flag (e.g. `--config`,
/// `--git`) and silently change install semantics. We accept the conservative
/// crate-name shape `[A-Za-z0-9][A-Za-z0-9_.\-]*`: at least one character,
/// no leading dash, and no other characters.
pub(crate) fn validate_cargo_tool_arg(value: &str, label: &str) -> anyhow::Result<()> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        anyhow::bail!("{label} is empty");
    };
    if !first.is_ascii_alphanumeric() {
        anyhow::bail!(
            "{label} {value:?} must start with an alphanumeric character (cannot begin with `-`)"
        );
    }
    // TASK-0519: skip the explicit alphanumeric guard above when validating
    // the rest of the string. Re-checking `first` against the broader allow-set
    // hides a subtle SEC-13 dependency: deleting the alphanumeric check would
    // silently accept a leading `-` again because the loop allows it.
    for ch in chars {
        if !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '-') {
            anyhow::bail!(
                "{label} {value:?} contains invalid character {ch:?}; allowed: [A-Za-z0-9_.-]"
            );
        }
    }
    Ok(())
}

pub(crate) fn install_cargo_tool_with_timeout(
    name: &str,
    package: Option<&str>,
    timeout: Duration,
) -> anyhow::Result<()> {
    validate_cargo_tool_arg(name, "tool name")?;
    if let Some(pkg) = package {
        validate_cargo_tool_arg(pkg, "package name")?;
    }
    let mut args = vec!["install"];
    if let Some(pkg) = package {
        args.push(pkg);
        args.push("--bin");
        args.push(name);
    } else {
        args.push(name);
    }
    let child = Command::new("cargo")
        .args(&args)
        .spawn()
        .context("failed to spawn cargo install")?;
    let status = run_with_timeout(child, timeout, &format!("cargo install {name}"))?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("cargo install {} failed", name)
    }
}

pub fn install_rustup_component(component: &str, toolchain: &str) -> anyhow::Result<()> {
    install_rustup_component_with_timeout(component, toolchain, DEFAULT_INSTALL_TIMEOUT)
}

pub(crate) fn install_rustup_component_with_timeout(
    component: &str,
    toolchain: &str,
    timeout: Duration,
) -> anyhow::Result<()> {
    validate_cargo_tool_arg(component, "rustup component")?;
    validate_cargo_tool_arg(toolchain, "rustup toolchain")?;
    let child = Command::new("rustup")
        .args(["component", "add", component, "--toolchain", toolchain])
        .spawn()
        .context("failed to spawn rustup component add")?;
    let status = run_with_timeout(child, timeout, &format!("rustup component add {component}"))?;
    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("rustup component add {} failed", component)
    }
}

pub fn install_tool(name: &str, spec: &ToolSpec) -> anyhow::Result<()> {
    if let Some(component) = spec.rustup_component() {
        let toolchain = get_active_toolchain()
            .ok_or_else(|| anyhow::anyhow!("could not determine active toolchain"))?;
        install_rustup_component(component, &toolchain)?;
    }

    match spec.source() {
        ToolSource::Cargo => {
            install_cargo_tool(name, spec.package())?;
        }
        ToolSource::System => {
            if spec.rustup_component().is_none() {
                anyhow::bail!("system tools cannot be auto-installed: {}", name);
            }
        }
    }

    Ok(())
}
