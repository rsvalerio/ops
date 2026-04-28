//! Detect whether tools/components are installed on the active toolchain.

use ops_core::config::tools::{ToolSource, ToolSpec};
use std::process::Command;

use crate::ToolStatus;

pub fn get_active_toolchain() -> Option<String> {
    // `--quiet` is rustup's global flag, not a subcommand option, so it
    // appears before `show`. It silences "info: ..." progress lines so the
    // first line of stdout is reliably the toolchain name on every rustup.
    let output = Command::new("rustup")
        .args(["--quiet", "show", "active-toolchain"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_active_toolchain(&stdout)
}

/// Parse the toolchain name out of `rustup show active-toolchain` stdout.
///
/// Handles both output shapes rustup has shipped:
///
/// * Legacy (rustup <1.28): a single line like `stable-aarch64-apple-darwin (default)`.
/// * Current (rustup ≥1.28): a multi-line block whose first non-empty line
///   is the bare toolchain name (e.g. `stable-aarch64-apple-darwin`)
///   followed by `active because: ...`.
///
/// Both shapes are handled by skipping blank/leading lines and returning the
/// first whitespace-separated token of the first non-empty line.
pub(crate) fn parse_active_toolchain(stdout: &str) -> Option<String> {
    let line = stdout.lines().map(str::trim).find(|l| !l.is_empty())?;
    line.split_whitespace().next().map(str::to_string)
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
    // TASK-0526: an empty short-name (caller passed "cargo-" or "") would
    // match any line whose first whitespace token is empty, i.e. any line
    // with leading whitespace. Reject early so a malformed [tools] entry
    // can't produce a false positive.
    if cargo_name.is_empty() {
        return false;
    }
    stdout.lines().any(|line| {
        line.split_whitespace()
            .next()
            .is_some_and(|cmd| cmd == cargo_name)
    })
}

/// SEC-13: walk `PATH` directly instead of shelling out to `which`. The
/// previous implementation paid spawn overhead for every probe (called per
/// tool per status check) and silently returned `false` on Windows, where
/// `which` is not a built-in. Walking `PATH` ourselves is portable, faster,
/// and avoids invoking another binary at all.
///
/// On Windows we also try the executable suffixes listed in `PATHEXT`
/// (defaulting to `.COM;.EXE;.BAT;.CMD`); on Unix we only check the bare
/// name and rely on the executable bit.
pub fn check_binary_installed(name: &str) -> bool {
    find_on_path(name).is_some()
}

pub(crate) fn find_on_path(name: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    find_on_path_in(name, &path)
}

pub(crate) fn find_on_path_in(
    name: &str,
    path_var: &std::ffi::OsStr,
) -> Option<std::path::PathBuf> {
    for dir in std::env::split_paths(path_var) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(name);
        if is_executable(&candidate) {
            return Some(candidate);
        }
        if cfg!(windows) {
            for ext in pathext_suffixes() {
                let mut with_ext = candidate.clone().into_os_string();
                with_ext.push(&ext);
                let p = std::path::PathBuf::from(with_ext);
                if is_executable(&p) {
                    return Some(p);
                }
            }
        }
    }
    None
}

#[cfg(windows)]
fn pathext_suffixes() -> Vec<std::ffi::OsString> {
    let raw = std::env::var_os("PATHEXT")
        .unwrap_or_else(|| std::ffi::OsString::from(".COM;.EXE;.BAT;.CMD"));
    std::env::split_paths(&raw)
        .map(std::path::PathBuf::into_os_string)
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(not(windows))]
fn pathext_suffixes() -> Vec<std::ffi::OsString> {
    Vec::new()
}

#[cfg(unix)]
fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(path) {
        Ok(m) => m.is_file() && m.permissions().mode() & 0o111 != 0,
        Err(_) => false,
    }
}

#[cfg(not(unix))]
fn is_executable(path: &std::path::Path) -> bool {
    // On Windows, file existence + extension match (caller's PATHEXT loop)
    // is the standard heuristic; the OS does not surface an executable bit
    // through `Permissions`. Match the behaviour of `which` and similar
    // tooling.
    std::fs::metadata(path).is_ok_and(|m| m.is_file())
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
