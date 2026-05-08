//! Install cargo tools and rustup components via subprocess with a timeout.

use anyhow::Context;
use ops_core::config::tools::{ToolSource, ToolSpec};
use ops_core::subprocess::{resolve_cargo_bin, resolve_rustup_bin};
use std::process::{Command, Stdio};
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
    // CONC-3: deliberately inherit stdout/stderr so cargo's progress output
    // streams straight to the user's terminal. The bounded `wait_timeout`
    // below is safe under inheritance because nothing in this process is
    // reading those fds — cargo writes directly to the inherited
    // descriptors. We do *not* capture stdout/stderr: that would require a
    // draining reader thread to avoid the same pipe-buffer deadlock fixed
    // in TASK-0650 for `git diff --cached`.
    //
    // CONC-5: stdin is closed via `Stdio::null()` so an unexpected
    // interactive prompt (rare in cargo, occasional in rustup) hits EOF
    // and bails deterministically instead of blocking until the timeout.
    let child = Command::new(resolve_cargo_bin())
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to spawn cargo install")?;
    let status = run_with_timeout(child, timeout, &format!("cargo install {name}"))?;
    if status.success() {
        Ok(())
    } else {
        // ERR-2 (TASK-1048): when both `package` and `name` are present, the
        // invocation is `cargo install <pkg> --bin <name>` — and a common
        // failure mode is the package not exposing a `<name>` bin target.
        // Naming only `name` in the error hides the package and misleads
        // operators debugging a misconfigured `[tools]` table. Surface both
        // identifiers (and the `--bin` redirection) so the failure points at
        // the actual cargo invocation.
        match package {
            Some(pkg) => anyhow::bail!("cargo install {} --bin {} failed", pkg, name),
            None => anyhow::bail!("cargo install {} failed", name),
        }
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
    // CONC-3: same inherited-stdio choice as `install_cargo_tool_with_timeout`.
    // See that function for the deadlock rationale.
    let child = Command::new(resolve_rustup_bin())
        .args(["component", "add", component, "--toolchain", toolchain])
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
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
    let has_rustup_component = spec.rustup_component().is_some();

    if let Some(component) = spec.rustup_component() {
        let toolchain = get_active_toolchain()
            .ok_or_else(|| anyhow::anyhow!("could not determine active toolchain"))?;
        install_rustup_component(component, &toolchain)?;
    }

    match spec.source() {
        ToolSource::Cargo => {
            if should_run_cargo_install(spec) {
                install_cargo_tool(name, spec.package())?;
            } else {
                // ERR-2 (TASK-1038): if a tool spec lists *both* a Cargo source
                // and a rustup_component, the rustup component install above
                // already ran; also running `cargo install` would silently
                // produce two installations where the operator's intent was
                // probably one or the other. Prefer the rustup-component path
                // (warn-and-skip) so the chosen behaviour is explicit and
                // observable.
                tracing::info!(
                    tool = name,
                    "preferred rustup component over cargo install for {}",
                    name
                );
            }
        }
        ToolSource::System => {
            if !has_rustup_component {
                anyhow::bail!("system tools cannot be auto-installed: {}", name);
            }
        }
    }

    Ok(())
}

/// ERR-2 (TASK-1038): pure dispatch helper for `install_tool`. Returns `false`
/// when a `ToolSpec` has both a `rustup_component` set and `ToolSource::Cargo`,
/// signalling the caller to skip `cargo install` and rely on the rustup
/// component path that already ran. Extracted so the both-set policy can be
/// pinned by a unit test without spawning subprocesses.
pub(crate) fn should_run_cargo_install(spec: &ToolSpec) -> bool {
    !matches!(
        (spec.source(), spec.rustup_component()),
        (ToolSource::Cargo, Some(_))
    )
}
