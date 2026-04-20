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

pub(crate) fn install_cargo_tool_with_timeout(
    name: &str,
    package: Option<&str>,
    timeout: Duration,
) -> anyhow::Result<()> {
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
