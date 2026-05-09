//! Detect whether tools/components are installed on the active toolchain.
//!
//! ARCH-1 / TASK-1158: split into per-concern submodules
//! ([`path`], [`cargo`], [`rustup`], [`timeout`]). `mod.rs` is the public
//! composition point and the dispatcher for [`check_tool_status_with`].

mod cargo;
mod path;
mod rustup;
mod timeout;

use ops_core::config::tools::{ToolSource, ToolSpec};

use crate::ToolStatus;

pub use cargo::{capture_cargo_list, check_cargo_tool_installed};
pub use path::{
    capture_path_index, check_binary_installed, check_binary_installed_with, PathIndex,
};
pub use rustup::{
    capture_rustup_components, check_rustup_component_installed, get_active_toolchain,
};

// Crate-internal re-exports for sibling modules and tests.
pub(crate) use cargo::is_in_cargo_list;
#[cfg(test)]
pub(crate) use path::{capture_path_index_from, find_on_path, find_on_path_in, is_in_path_index};
pub(crate) use rustup::is_component_in_list;
#[cfg(test)]
pub(crate) use rustup::parse_active_toolchain;

pub fn check_tool_status(name: &str, spec: &ToolSpec) -> ToolStatus {
    check_tool_status_with(name, spec, None, None, None)
}

/// Variant of [`check_tool_status`] that reuses precomputed `cargo --list`,
/// `rustup component list --installed`, and `$PATH` index outputs, so the
/// caller can resolve them once per probe sweep and amortise the spawn /
/// directory-walk cost across all entries.
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
            None => check_rustup_component_installed(component),
        };
        if !installed {
            return ToolStatus::NotInstalled;
        }
    }

    let is_installed = match spec.source() {
        ToolSource::Cargo => match cargo_list {
            Some(s) => is_in_cargo_list(s, name) || check_binary_installed_with(name, path_index),
            None => check_cargo_tool_installed(name),
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
