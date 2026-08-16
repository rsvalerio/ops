//! The `[extensions]`, `[about]`, `[data]` and `[output]` config sections.
//!
//! Each is an independent leaf of [`super::root::Config`]; the terminal-width
//! resolution behind `output.columns` is the only non-trivial logic.

use crate::serde_defaults;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;

/// Extension configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionConfig {
    /// List of extension names to enable. Empty = no extensions.
    /// If None (missing from config), all compiled-in extensions are enabled.
    pub enabled: Option<Vec<String>>,
}

impl ExtensionConfig {
    pub(crate) fn is_default(&self) -> bool {
        self.enabled.is_none()
    }
}

/// About card display settings.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AboutConfig {
    /// Fields to display on the about card. None = show all fields.
    /// Values: "project", "modules", "codebase", "authors", "repository", "coverage"
    pub fields: Option<Vec<String>>,
}

impl AboutConfig {
    pub(crate) fn is_default(&self) -> bool {
        self.fields.is_none()
    }
}

/// Data storage settings (`DuckDB` path).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DataConfig {
    /// Optional path override for the `DuckDB` database.
    /// Absolute paths are used as-is; relative paths resolve from workspace root.
    /// Default (when None): .ops/data.duckdb (stack-dependent)
    pub path: Option<PathBuf>,
}

impl DataConfig {
    pub(crate) fn is_default(&self) -> bool {
        self.path.is_none()
    }
}

/// Output and theme settings.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OutputConfig {
    /// Theme name (built-in: "classic", "compact"; or custom theme from [themes]).
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Line width in columns for step lines (command + spacer + time). No runtime change.
    /// When omitted, auto-detected from terminal width (90%).
    #[serde(
        default = "default_columns",
        skip_serializing_if = "is_default_columns"
    )]
    pub columns: u16,
    /// When true (default), show error details (exit status, stderr tail) inline
    /// below the failed step line. When false, only the step line with failure icon is shown.
    #[serde(default = "serde_defaults::default_true")]
    pub show_error_detail: bool,
    /// Maximum number of stderr tail lines to show in error details.
    /// Default: 5. Use `--verbose` to show all lines.
    #[serde(
        default = "default_stderr_tail_lines",
        skip_serializing_if = "is_default_stderr_tail_lines"
    )]
    pub stderr_tail_lines: usize,
    /// Display order of command categories in help output.
    /// Categories listed here appear first, in the given order.
    /// Unlisted categories are appended alphabetically after.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub category_order: Vec<String>,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            columns: AUTO_COLUMNS,
            show_error_detail: true,
            stderr_tail_lines: default_stderr_tail_lines(),
            category_order: Vec::new(),
        }
    }
}

fn default_theme() -> String {
    "classic".into()
}

/// READ-5 / TASK-1219: deserialising `[output]` must produce a deterministic
/// `Config` regardless of the calling terminal. Use `0` as an "auto" sentinel
/// for the serde default; terminal-aware width is resolved at render time via
/// [`OutputConfig::resolve_columns`].
pub(crate) const AUTO_COLUMNS: u16 = 0;

/// Fallback used when no terminal is attached (CI, piped output) and the user
/// did not pin `columns` in `.ops.toml`.
const FALLBACK_COLUMNS: u16 = 80;

fn default_columns() -> u16 {
    AUTO_COLUMNS
}

/// Compute 90% of the reported terminal width without wrapping u16.
/// SEC-15 / TASK-0344: widths above ~7281 cols would overflow `w * 9`.
/// Promote to u32 for the multiply, then clamp back to u16.
pub(crate) fn scale_columns(width: u16) -> u16 {
    let scaled = u32::from(width) * 9 / 10;
    u16::try_from(scaled).unwrap_or(u16::MAX)
}

// serde's `skip_serializing_if` predicates are always called with `&T`.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_default_columns(v: &u16) -> bool {
    *v == AUTO_COLUMNS
}

/// READ-5 / TASK-1416: process-stable cache of the auto-resolved terminal
/// width. `terminal_size` issues a `TIOCGWINSZ` ioctl (or the Windows
/// console-handle equivalent) on every call; rendering paths (step lines,
/// help/about cards) call [`OutputConfig::resolve_columns`] per render, and
/// the value is stable for the lifetime of a single `ops <cmd>` — SIGWINCH
/// resizes within a single invocation are not observed, matching the
/// `TMPDIR_DISPLAY` and `OPS_TOML_MAX_BYTES` `OnceLock` discipline already
/// in use.
static AUTO_COLUMNS_CACHE: OnceLock<u16> = OnceLock::new();

fn probe_auto_columns() -> u16 {
    terminal_size::terminal_size().map_or(FALLBACK_COLUMNS, |(w, _)| scale_columns(w.0))
}

impl OutputConfig {
    /// Effective column width for rendering. When `columns` is the auto
    /// sentinel (`0`), probe the terminal once per process and cache the
    /// result; otherwise honour the pinned config value. READ-5 / TASK-1219,
    /// READ-5 / TASK-1416.
    #[must_use]
    pub fn resolve_columns(&self) -> u16 {
        if self.columns == AUTO_COLUMNS {
            *AUTO_COLUMNS_CACHE.get_or_init(probe_auto_columns)
        } else {
            self.columns
        }
    }
}

fn default_stderr_tail_lines() -> usize {
    5
}

// serde's `skip_serializing_if` predicates are always called with `&T`.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_default_stderr_tail_lines(v: &usize) -> bool {
    *v == default_stderr_tail_lines()
}
