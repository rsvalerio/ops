//! Hierarchical configuration parsing and command resolution.
//!
//! Resolution order: internal default → global config → local `.ops.toml` → env vars.

pub(crate) mod commands;
mod edit;
mod loader;
pub(crate) mod merge;
pub(crate) mod overlay;
pub mod theme_types;
pub mod tools;

pub use commands::{CommandId, CommandSpec, CompositeCommandSpec, ExecCommandSpec};
pub use edit::{edit_ops_toml, read_ops_toml, write_ops_toml};
pub use overlay::{
    AboutConfigOverlay, ConfigOverlay, DataConfigOverlay, ExtensionConfigOverlay,
    OutputConfigOverlay,
};

#[cfg(test)]
pub(crate) use loader::global_config_path;
pub use loader::{load_config, load_config_or_default, read_config_file};
pub use merge::merge_config;

use crate::config::theme_types::ThemeConfig;
use crate::config::tools::ToolSpec;
use crate::serde_defaults;
use anyhow::Context;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Root configuration structure.
///
/// `Config::default` is intended for tests and downstream extension wiring
/// where a blank slate is wanted. Runtime code should call
/// [`load_config`] so the user-visible defaults (theme = "classic", etc.)
/// come from the single source of truth embedded in
/// `.default.ops.toml`. See TRAIT-4 in the backlog for the rationale.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub commands: IndexMap<String, CommandSpec>,
    #[serde(default, skip_serializing_if = "DataConfig::is_default")]
    pub data: DataConfig,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub themes: IndexMap<String, ThemeConfig>,
    #[serde(default, skip_serializing_if = "ExtensionConfig::is_default")]
    pub extensions: ExtensionConfig,
    #[serde(default, skip_serializing_if = "AboutConfig::is_default")]
    pub about: AboutConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub tools: IndexMap<String, ToolSpec>,
}

impl Config {
    /// Validate all command specs. Called after loading to fail fast on invalid config.
    pub fn validate(&self) -> anyhow::Result<()> {
        for (name, spec) in &self.commands {
            if let CommandSpec::Exec(exec) = spec {
                exec.validate(name)?;
            }
        }
        Ok(())
    }

    /// Find the canonical command name for an alias.
    /// Returns `Some(command_name)` if the alias matches a command's aliases list.
    ///
    /// O(N·M) over commands × aliases. The alias lookup is called once per
    /// CLI invocation so an inline scan is still cheap in practice — each
    /// user has tens of commands and a handful of aliases. Build
    /// [`Config::build_alias_map`] once if a hot path ever needs O(1)
    /// lookups instead.
    pub fn resolve_alias(&self, alias: &str) -> Option<&str> {
        for (name, spec) in &self.commands {
            if spec.aliases().iter().any(|a| a == alias) {
                return Some(name.as_str());
            }
        }
        None
    }

    /// Build an `alias → canonical command name` map. Amortizes lookups for
    /// callers that resolve many aliases against the same config.
    ///
    /// The default `resolve_alias` path is O(N·M); building this map is also
    /// O(N·M) once, but each subsequent lookup is O(1).
    #[must_use]
    pub fn build_alias_map(&self) -> HashMap<&str, &str> {
        let mut map = HashMap::new();
        for (name, spec) in &self.commands {
            for alias in spec.aliases() {
                map.insert(alias.as_str(), name.as_str());
            }
        }
        map
    }
}

/// Extension configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionConfig {
    /// List of extension names to enable. Empty = no extensions.
    /// If None (missing from config), all compiled-in extensions are enabled.
    pub enabled: Option<Vec<String>>,
}

impl ExtensionConfig {
    fn is_default(&self) -> bool {
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
    fn is_default(&self) -> bool {
        self.fields.is_none()
    }
}

/// Data storage settings (DuckDB path).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DataConfig {
    /// Optional path override for the DuckDB database.
    /// Absolute paths are used as-is; relative paths resolve from workspace root.
    /// Default (when None): .ops/data.duckdb (stack-dependent)
    pub path: Option<PathBuf>,
}

impl DataConfig {
    fn is_default(&self) -> bool {
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
            columns: default_columns(),
            show_error_detail: true,
            stderr_tail_lines: default_stderr_tail_lines(),
            category_order: Vec::new(),
        }
    }
}

fn default_theme() -> String {
    "classic".into()
}

/// Fixed default used by the serde skip predicate so serialization is deterministic
/// regardless of terminal width. Runtime display uses terminal-responsive `default_columns()`.
const SERIALIZATION_DEFAULT_COLUMNS: u16 = 80;

fn default_columns() -> u16 {
    terminal_size::terminal_size()
        .map(|(w, _)| scale_columns(w.0))
        .unwrap_or(SERIALIZATION_DEFAULT_COLUMNS)
}

/// Compute 90% of the reported terminal width without wrapping u16.
/// SEC-15 / TASK-0344: widths above ~7281 cols would overflow `w * 9`.
/// Promote to u32 for the multiply, then clamp back to u16.
fn scale_columns(width: u16) -> u16 {
    let scaled = u32::from(width) * 9 / 10;
    u16::try_from(scaled).unwrap_or(u16::MAX)
}

fn is_default_columns(v: &u16) -> bool {
    *v == SERIALIZATION_DEFAULT_COLUMNS
}

fn default_stderr_tail_lines() -> usize {
    5
}

fn is_default_stderr_tail_lines(v: &usize) -> bool {
    *v == default_stderr_tail_lines()
}

/// Default config content from `src/.default.ops.toml` (embedded at build; used as base config and for `cargo ops init`).
/// Build fails if the file is missing.
pub fn default_ops_toml() -> &'static str {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/.default.ops.toml"
    ))
}

/// Controls which sections are included in `ops init` output.
#[derive(Debug, Clone)]
pub struct InitSections {
    pub output: bool,
    pub themes: bool,
    pub commands: bool,
}

impl InitSections {
    /// Build from CLI flags. When no flags are given, default to output-only.
    pub fn from_flags(output: bool, themes: bool, commands: bool) -> Self {
        if !output && !themes && !commands {
            Self {
                output: true,
                themes: false,
                commands: false,
            }
        } else {
            Self {
                output,
                themes,
                commands,
            }
        }
    }
}

/// Build init template with only the requested sections.
pub fn init_template(workspace_root: &Path, sections: &InitSections) -> anyhow::Result<String> {
    let full: Config =
        toml::from_str(default_ops_toml()).context("failed to parse internal default config")?;

    let mut config = Config::default();

    if sections.output {
        config.output = full.output;
    }

    if sections.themes {
        config.themes = full.themes;
    }

    if sections.commands {
        if let Some(stack) = crate::stack::Stack::detect(workspace_root) {
            for (id, spec) in stack.default_commands() {
                config.commands.insert(id, spec);
            }
            config.stack = Some(stack.as_str().to_string());
        }
    }

    toml::to_string_pretty(&config).context("failed to serialize init config")
}

#[cfg(test)]
mod tests;
