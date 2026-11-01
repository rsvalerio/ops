//! `ops semver` and `ops feature-matrix`: generic Rust checks backed by a
//! cargo plugin.
//!
//! Each is one fixed `cargo <plugin>` line. The value over a `[commands.*]`
//! entry is the failure mode: a missing plugin is reported by name with the
//! command that installs it, as `ops deps` does, instead of cargo's
//! `no such command`. Both compile into `target/`, so as builtins they keep
//! the exclusive default.

use std::io::Write;
use std::path::Path;
use std::process::{Command, ExitCode};
use std::time::Duration;

use anyhow::Context;
use ops_core::subprocess::{resolve_cargo_bin, run_cargo};

/// A check that is one `cargo <plugin> ...` invocation.
#[derive(Debug, Clone, Copy)]
pub struct CargoCheck {
    /// The cargo subcommand, e.g. `"semver-checks"`.
    pub plugin: &'static str,
    /// The crate that provides it, e.g. `"cargo-semver-checks"`.
    pub install_crate: &'static str,
    /// Arguments after the subcommand.
    pub args: &'static [&'static str],
}

/// `ops semver`: `cargo-semver-checks` compares each publishable workspace
/// crate against its latest crates.io release.
pub const SEMVER: CargoCheck = CargoCheck {
    plugin: "semver-checks",
    install_crate: "cargo-semver-checks",
    args: &["--workspace"],
};

/// `ops feature-matrix`: `cargo-hack` checks each feature alone, no default
/// features, and the default build.
pub const FEATURE_MATRIX: CargoCheck = CargoCheck {
    plugin: "hack",
    install_crate: "cargo-hack",
    args: &["check", "--workspace", "--each-feature"],
};

/// Bound on the `cargo <plugin> --version` probe, which should answer at once;
/// `OPS_SUBPROCESS_TIMEOUT_SECS` overrides it.
const PROBE_TIMEOUT: Duration = Duration::from_secs(30);

impl CargoCheck {
    /// The executable cargo dispatches to.
    pub fn binary(&self) -> String {
        format!("cargo-{}", self.plugin)
    }

    /// The command that installs the plugin.
    pub fn install_command(&self) -> String {
        format!("cargo install {}", self.install_crate)
    }

    fn command_line(&self) -> String {
        format!("cargo {} {}", self.plugin, self.args.join(" "))
    }

    /// Whether `cargo <plugin> --version` succeeds from `root`.
    fn is_installed(&self, root: &Path) -> anyhow::Result<bool> {
        let label = format!("cargo {} --version", self.plugin);
        let output = run_cargo(&[self.plugin, "--version"], root, PROBE_TIMEOUT, &label)
            .with_context(|| format!("failed to probe for {}", self.binary()))?;
        Ok(output.status.success())
    }
}

/// Run `check` in `root`, or only print its command line under `dry_run`.
///
/// # Errors
///
/// The plugin is missing (naming its install command), or cargo could not be
/// spawned.
pub fn run_cargo_check(check: &CargoCheck, root: &Path, dry_run: bool) -> anyhow::Result<ExitCode> {
    let mut out = std::io::stdout().lock();
    if dry_run {
        writeln!(out, "{}", check.command_line())?;
        return Ok(ExitCode::SUCCESS);
    }
    if !check.is_installed(root)? {
        anyhow::bail!(
            "cargo {} is not installed. Install with: {}",
            check.plugin,
            check.install_command()
        );
    }
    writeln!(out, "{}", check.command_line())?;
    out.flush()?;
    let status = Command::new(resolve_cargo_bin())
        .arg(check.plugin)
        .args(check.args)
        .current_dir(root)
        .status()
        .with_context(|| format!("failed to spawn {}", check.command_line()))?;
    Ok(if status.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MISSING: CargoCheck = CargoCheck {
        plugin: "ops-no-such-plugin",
        install_crate: "cargo-ops-no-such-plugin",
        args: &[],
    };

    #[test]
    fn a_missing_plugin_reports_its_install_command() {
        let dir = tempfile::tempdir().unwrap();
        let err = run_cargo_check(&MISSING, dir.path(), false)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("cargo ops-no-such-plugin is not installed")
                && err.contains("cargo install cargo-ops-no-such-plugin"),
            "{err}"
        );
    }

    #[test]
    fn dry_run_spawns_nothing_even_for_a_missing_plugin() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            run_cargo_check(&MISSING, dir.path(), true).unwrap(),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn command_lines_name_the_plugin_and_its_arguments() {
        assert_eq!(SEMVER.command_line(), "cargo semver-checks --workspace");
        assert_eq!(
            FEATURE_MATRIX.command_line(),
            "cargo hack check --workspace --each-feature"
        );
    }
}
