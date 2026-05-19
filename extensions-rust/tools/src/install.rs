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
/// crate-name shape `[A-Za-z0-9][A-Za-z0-9_-]*`: at least one character,
/// alphanumeric leading byte, no dash leader, no dots, no other characters.
///
/// SEC-13 / TASK-1199: prior to TASK-1199 the allow-set also included `.`,
/// which is broader than crates.io's grammar (`[a-zA-Z][a-zA-Z0-9_-]*`) and
/// rustup's component shape. Allowing `.` let entries like `tool.cargo` or
/// `cargo.deny.something` reach `cargo install` / `rustup component add`
/// and invited future contributors to assume the validator had vetted names
/// in path-construction or display contexts where `.` carries meaning (e.g.
/// dotfile shells, `parent.<x>` traversal). We reject `.` here so the
/// validator's defense-in-depth contract matches the upstream grammars it is
/// guarding. The leading byte stays alphanumeric (rather than tightening to
/// alphabetic to fully match crates.io) so that version-leading rustup
/// toolchain identifiers like `1.70.0-x86_64-apple-darwin` are still
/// rejected on the dot — not on the leading digit — keeping the operator
/// error message pointed at the actual policy break.
pub(crate) fn validate_cargo_tool_arg(value: &str, label: &str) -> anyhow::Result<()> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        anyhow::bail!("{label} is empty");
    };
    if !first.is_ascii_alphanumeric() {
        anyhow::bail!(
            "{label} {value:?} must start with an alphanumeric character (cannot begin with `-` or `.`)"
        );
    }
    // TASK-0519: skip the explicit leading-character guard above when
    // validating the rest of the string. Re-checking `first` against the
    // broader allow-set hides a subtle SEC-13 dependency: deleting the
    // leading-character check would silently accept a leading `-` again
    // because the loop allows it.
    for ch in chars {
        if !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-') {
            anyhow::bail!(
                "{label} {value:?} contains invalid character {ch:?}; allowed: [A-Za-z0-9_-]"
            );
        }
    }
    Ok(())
}

/// DUP-3 / TASK-1565: shared spawn + bounded-wait scaffold for the
/// install paths. The stdio policy (stdin closed, stdout/stderr
/// inherited) and timeout-bounded wait live in one place so future
/// install entry points (e.g. `cargo binstall`, `rustup toolchain
/// install`) cannot drift on the contract.
///
/// CONC-3: stdout/stderr are inherited because nothing in this process
/// reads them (cargo writes directly to the inherited fds), keeping the
/// bounded wait safe from the pipe-buffer deadlock fixed in TASK-0650.
/// CONC-5: stdin is closed so an unexpected interactive prompt hits EOF
/// and bails deterministically rather than blocking until the deadline.
fn spawn_install_with_timeout(
    bin: impl AsRef<std::ffi::OsStr>,
    args: &[&str],
    timeout: Duration,
    spawn_label: &'static str,
    wait_label: &str,
) -> anyhow::Result<std::process::ExitStatus> {
    let child = Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to spawn {spawn_label}"))?;
    run_with_timeout(child, timeout, wait_label)
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
    let status = spawn_install_with_timeout(
        resolve_cargo_bin(),
        &args,
        timeout,
        "cargo install",
        &format!("cargo install {name}"),
    )?;
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
    let status = spawn_install_with_timeout(
        resolve_rustup_bin(),
        &["component", "add", component, "--toolchain", toolchain],
        timeout,
        "rustup component add",
        &format!("rustup component add {component}"),
    )?;
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
