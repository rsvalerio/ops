//! Detect whether tools/components are installed on the active toolchain.
//!
//! ARCH-1 / TASK-1158: split into per-concern submodules
//! ([`path`], [`cargo`], [`rustup`], [`timeout`]). `mod.rs` is the public
//! composition point and the dispatcher for [`check_tool_status_with`].

mod cargo;
mod path;
mod rustup;
mod timeout;

use std::collections::HashSet;

use ops_core::config::tools::{ToolSource, ToolSpec};

use crate::ToolStatus;

pub use cargo::{capture_cargo_list, check_cargo_tool_installed};
pub use path::{
    capture_path_index, check_binary_installed, check_binary_installed_with, PathIndex,
};
pub use rustup::{
    capture_rustup_components, check_rustup_component_installed, get_active_toolchain,
    ActiveToolchain,
};
pub use timeout::ProbeOutcome;

// Crate-internal re-exports for sibling modules and tests.
pub(crate) use cargo::{cargo_list_index, is_in_cargo_list, is_in_cargo_set};
pub(crate) use rustup::{is_component_in_list, is_component_in_set, rustup_components_index};

#[must_use]
pub fn check_tool_status(name: &str, spec: &ToolSpec) -> ToolStatus {
    check_tool_status_with(name, spec, None, None, None)
}

/// Variant of [`check_tool_status`] that reuses precomputed `cargo --list`,
/// `rustup component list --installed`, and `$PATH` index outputs, so the
/// caller can resolve them once per probe sweep and amortise the spawn /
/// directory-walk cost across all entries.
/// API / TASK-1200: distinguishes a *probe-failed* outcome from a
/// *not-installed* outcome on the rustup-component / cargo-list paths.
/// A timed-out `rustup component list` or `cargo --list` no longer
/// collapses onto `NotInstalled` (which `tools_cmd::run_install` then
/// reinstalls); it surfaces as [`ToolStatus::ProbeFailed`] so the
/// install path skips the entry.
#[must_use]
pub fn check_tool_status_with(
    name: &str,
    spec: &ToolSpec,
    cargo_list: Option<&str>,
    rustup_components: Option<&str>,
    path_index: Option<&PathIndex>,
) -> ToolStatus {
    if let Some(component) = spec.rustup_component() {
        let installed = match rustup_components {
            Some(s) => is_component_in_list(s, component),
            None => match check_rustup_component_installed(component) {
                ProbeOutcome::Ok(b) => b,
                ProbeOutcome::Failed => return ToolStatus::ProbeFailed,
            },
        };
        if !installed {
            return ToolStatus::NotInstalled;
        }
    }

    let is_installed = match spec.source() {
        ToolSource::Cargo => match cargo_list {
            Some(s) => is_in_cargo_list(s, name) || check_binary_installed_with(name, path_index),
            None => match check_cargo_tool_installed(name) {
                ProbeOutcome::Ok(b) => b,
                ProbeOutcome::Failed => return ToolStatus::ProbeFailed,
            },
        },
        ToolSource::System => check_binary_installed_with(name, path_index),
    };

    if is_installed {
        ToolStatus::Installed
    } else {
        ToolStatus::NotInstalled
    }
}

/// PERF-3 / TASK-1616: sweep-mode variant of [`check_tool_status_with`]
/// that consults precomputed hash sets instead of re-walking the
/// captured stdout per tool. The public [`check_tool_status_with`] API
/// (which takes `&str`) is preserved for per-tool callers — see AC#2 in
/// TASK-1616.
pub(crate) fn check_tool_status_with_sets(
    name: &str,
    spec: &ToolSpec,
    cargo_list: Option<&HashSet<String>>,
    rustup_components: Option<&HashSet<String>>,
    path_index: Option<&PathIndex>,
) -> ToolStatus {
    if let Some(component) = spec.rustup_component() {
        let installed = match rustup_components {
            Some(s) => is_component_in_set(s, component),
            None => match check_rustup_component_installed(component) {
                ProbeOutcome::Ok(b) => b,
                ProbeOutcome::Failed => return ToolStatus::ProbeFailed,
            },
        };
        if !installed {
            return ToolStatus::NotInstalled;
        }
    }

    let is_installed = match spec.source() {
        ToolSource::Cargo => match cargo_list {
            Some(s) => is_in_cargo_set(s, name) || check_binary_installed_with(name, path_index),
            None => match check_cargo_tool_installed(name) {
                ProbeOutcome::Ok(b) => b,
                ProbeOutcome::Failed => return ToolStatus::ProbeFailed,
            },
        },
        ToolSource::System => check_binary_installed_with(name, path_index),
    };

    if is_installed {
        ToolStatus::Installed
    } else {
        ToolStatus::NotInstalled
    }
}

#[cfg(test)]
mod probe_log_format_tests {
    use ops_core::output::format_error_tail;

    /// ERR-7 / TASK-0979: subprocess stderr snippets flow through the `?`
    /// formatter so cargo/rustup ANSI escapes or registry-served diagnostics
    /// containing newlines cannot forge log records.
    #[test]
    fn stderr_snippet_debug_escapes_control_characters() {
        let snippet = "warn\nerror: \u{1b}[31mhi\u{1b}[0m";
        let rendered = format!("{snippet:?}");
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains('\u{1b}'));
        assert!(rendered.contains("\\n"));
    }

    /// ERR-1 / TASK-1032: byte-bounded snippet handles non-ASCII safely.
    #[test]
    fn stderr_snippet_handles_non_ascii_without_mid_grapheme_cut() {
        let mut stderr = Vec::new();
        for i in 0..50 {
            stderr.extend_from_slice(format!("行{i}は失敗\n").as_bytes());
        }
        let snippet = format_error_tail(&stderr, 10);
        assert!(!snippet.contains('\u{FFFD}'), "no replacement chars");
        assert_eq!(snippet.lines().count(), 10);
        assert!(snippet.ends_with("行49は失敗"));
        assert!(snippet.is_char_boundary(snippet.len()));
    }

    /// ERR-1 / TASK-1032 AC#2: snippet stays bounded for pathological stderr.
    #[test]
    fn stderr_snippet_caps_line_count() {
        let stderr = "x\n".repeat(10_000);
        let snippet = format_error_tail(stderr.as_bytes(), 10);
        assert_eq!(snippet.lines().count(), 10);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolStatus;
    use ops_core::config::tools::{ExtendedToolSpec, ToolSource, ToolSpec};

    #[test]
    #[ignore = "requires rustup + cargo-fmt installed; run with: cargo test -- --ignored"]
    fn check_tool_status_simple_installed() {
        let spec = ToolSpec::Simple("Format code".to_string());
        assert_eq!(check_tool_status("cargo-fmt", &spec), ToolStatus::Installed);
    }

    /// API / TASK-1200: when the underlying probe (`rustup component list
    /// --installed`) cannot be answered (here: simulated by pointing
    /// `$RUSTUP` at a script that exits non-zero), the tool's status must
    /// surface as [`ToolStatus::ProbeFailed`] rather than silently
    /// collapsing onto [`ToolStatus::NotInstalled`]. The CLI install path
    /// (`run_tools_install`) filters strictly on `NotInstalled`, so a
    /// `ProbeFailed` entry no longer triggers the reinstall mutation that
    /// motivated this finding.
    ///
    /// We exercise the non-zero-exit branch (rather than a real timeout)
    /// to keep the test fast and deterministic; the
    /// `timeout_returns_none_quickly` test in `probe::timeout` already
    /// pins that the timeout path itself surfaces as
    /// `ProbeOutcome::Failed`, which `check_tool_status_with` then maps
    /// to `ProbeFailed` via the same arm.
    #[test]
    #[cfg(unix)]
    #[serial_test::serial]
    fn check_tool_status_surfaces_probe_failed_on_wedged_rustup() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let fake = dir.path().join("rustup");
        std::fs::write(&fake, "#!/bin/sh\necho 'rustup is wedged' >&2\nexit 1\n").unwrap();
        let mut perms = std::fs::metadata(&fake).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&fake, perms).unwrap();

        let spec = ToolSpec::Extended(ExtendedToolSpec {
            description: "needs rustup".to_string(),
            rustup_component: Some("clippy".to_string()),
            package: None,
            source: ToolSource::Cargo,
        });

        let prev_rustup = std::env::var_os("RUSTUP");
        // SAFETY: serial_test::serial guards env-mutation; the probe spawned
        // below honours `RUSTUP` synchronously.
        unsafe { std::env::set_var("RUSTUP", &fake) };

        let status = check_tool_status("clippy", &spec);

        unsafe {
            match prev_rustup {
                Some(v) => std::env::set_var("RUSTUP", v),
                None => std::env::remove_var("RUSTUP"),
            }
        };

        assert_eq!(
            status,
            ToolStatus::ProbeFailed,
            "a wedged rustup probe must surface as ProbeFailed, not NotInstalled (which would trigger reinstall)"
        );
    }

    /// API / TASK-1200: pin the install-path policy: `run_tools_install`
    /// filters strictly on `ToolStatus::NotInstalled`, so a `ProbeFailed`
    /// entry must NOT be picked up for reinstall. The previous shape
    /// collapsed timeout/IO errors onto `NotInstalled`, turning a transient
    /// probe failure into a real `cargo install` / `rustup component add`
    /// mutation.
    #[test]
    fn probe_failed_status_excluded_from_install_filter() {
        let statuses = [
            ToolStatus::Installed,
            ToolStatus::NotInstalled,
            ToolStatus::ProbeFailed,
        ];
        let to_install: Vec<_> = statuses
            .iter()
            .filter(|s| **s == ToolStatus::NotInstalled)
            .collect();
        assert_eq!(to_install.len(), 1);
        assert_eq!(*to_install[0], ToolStatus::NotInstalled);
    }

    #[test]
    #[serial_test::serial]
    fn check_tool_status_simple_not_installed() {
        let spec = ToolSpec::Simple("desc".to_string());
        assert_eq!(
            check_tool_status("cargo-nonexistent-abc123", &spec),
            ToolStatus::NotInstalled
        );
    }

    #[test]
    #[ignore = "requires rustup + clippy component installed; run with: cargo test -- --ignored"]
    fn check_tool_status_extended_with_rustup_component() {
        let spec = ToolSpec::Extended(ExtendedToolSpec {
            description: "Clippy lints".to_string(),
            rustup_component: Some("clippy".to_string()),
            package: None,
            source: ToolSource::Cargo,
        });
        assert_eq!(
            check_tool_status("cargo-clippy", &spec),
            ToolStatus::Installed
        );
    }

    #[test]
    #[ignore = "requires rustup installed; run with: cargo test -- --ignored"]
    fn check_tool_status_system_binary() {
        let spec = ToolSpec::Extended(ExtendedToolSpec {
            description: "Rust toolchain manager".to_string(),
            rustup_component: None,
            package: None,
            source: ToolSource::System,
        });
        assert_eq!(check_tool_status("rustup", &spec), ToolStatus::Installed);
    }

    #[test]
    fn check_tool_status_system_missing() {
        let spec = ToolSpec::Extended(ExtendedToolSpec {
            description: "desc".to_string(),
            rustup_component: None,
            package: None,
            source: ToolSource::System,
        });
        assert_eq!(
            check_tool_status("nonexistent-abc123", &spec),
            ToolStatus::NotInstalled
        );
    }

    #[test]
    #[serial_test::serial]
    fn check_tool_status_missing_rustup_component() {
        let spec = ToolSpec::Extended(ExtendedToolSpec {
            description: "desc".to_string(),
            rustup_component: Some("nonexistent-component-xyz".to_string()),
            package: None,
            source: ToolSource::Cargo,
        });
        assert_eq!(
            check_tool_status("cargo-fmt", &spec),
            ToolStatus::NotInstalled
        );
    }
}
