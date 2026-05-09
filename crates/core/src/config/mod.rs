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
pub use edit::{atomic_write, edit_ops_toml, read_ops_toml, write_ops_toml};
pub use overlay::{
    AboutConfigOverlay, ConfigOverlay, DataConfigOverlay, ExtensionConfigOverlay,
    OutputConfigOverlay,
};

#[cfg(test)]
pub(crate) use loader::global_config_path;
pub use loader::{load_config, load_config_or_default, read_config_file};
#[cfg(any(test, feature = "test-support"))]
pub use loader::{load_config_call_count, reset_load_config_call_count};
pub use merge::merge_config;

use crate::config::theme_types::ThemeConfig;
use crate::config::tools::ToolSpec;
use crate::serde_defaults;
use anyhow::Context;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Maximum recursion depth for composite expansion. Mirrors the runner's
/// `MAX_DEPTH` so the same configs that are accepted at load time are also
/// accepted at run time.
pub const MAX_COMPOSITE_DEPTH: usize = 100;

/// Root configuration structure.
///
/// TRAIT-4 / TASK-0872: `Default` is **gated to test/test-support builds**
/// so a buggy production CLI path cannot silently fall back to a blank
/// `Config` (no commands, no themes, etc.) instead of going through
/// [`load_config_or_default`]. Production code that genuinely needs a
/// blank-slate Config (the load-failure degradation, init-template
/// scaffolding) calls [`Config::empty`] explicitly so the choice is visible
/// at the call site. The user-visible defaults (theme = "classic", etc.)
/// come from `.default.ops.toml` via the loader.
#[derive(Debug, Clone, Deserialize, Serialize)]
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
    /// Construct a blank `Config` for the documented degradation paths
    /// ([`load_config_or_default`] fallback, [`init_template`] scaffolding).
    /// Production code that wants user-visible defaults should call
    /// [`load_config`] / [`load_config_or_default`] instead.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            output: OutputConfig::default(),
            commands: IndexMap::default(),
            data: DataConfig::default(),
            themes: IndexMap::default(),
            extensions: ExtensionConfig::default(),
            about: AboutConfig::default(),
            stack: None,
            tools: IndexMap::default(),
        }
    }

    /// Validate all command specs. Called after loading to fail fast on invalid config.
    ///
    /// Validates exec specs unconditionally. Composite specs are not checked
    /// here because composite commands may reference stack defaults or
    /// extension-registered commands that are not known at config load time —
    /// see [`Config::validate_commands`] for full composite validation.
    pub fn validate(&self) -> anyhow::Result<()> {
        for (name, spec) in &self.commands {
            if let CommandSpec::Exec(exec) = spec {
                exec.validate(name)?;
            }
        }
        Ok(())
    }

    /// Validate exec specs and every composite's references against the
    /// merged set of `config.commands` plus `externals` (stack defaults +
    /// registered extension command ids).
    ///
    /// Catches three failure modes that would otherwise only surface when
    /// the user invokes the affected command:
    /// - unknown reference (typo such as `commands = ["buidl"]`)
    /// - cycle (self-reference or indirect cycle)
    /// - depth violation (deeper than [`MAX_COMPOSITE_DEPTH`])
    ///
    /// Does not stand up a [`crate::runner::CommandRunner`]; the caller
    /// passes in the externally-known ids explicitly, so this can run from
    /// tests or from any setup path that already knows the extra command
    /// stores.
    pub fn validate_commands(&self, externals: &[&str]) -> anyhow::Result<()> {
        self.validate()?;

        let known: std::collections::HashSet<&str> = self
            .commands
            .keys()
            .map(String::as_str)
            .chain(externals.iter().copied())
            .collect();

        for (name, spec) in &self.commands {
            if let CommandSpec::Composite(_) = spec {
                let mut visiting = std::collections::HashSet::new();
                self.walk_composite(name, &known, &mut visiting, 0)?;
            }
        }
        Ok(())
    }

    fn walk_composite<'a>(
        &'a self,
        name: &'a str,
        known: &std::collections::HashSet<&'a str>,
        visiting: &mut std::collections::HashSet<&'a str>,
        depth: usize,
    ) -> anyhow::Result<()> {
        if depth > MAX_COMPOSITE_DEPTH {
            anyhow::bail!(
                "command '{name}': composite expansion exceeded depth limit {MAX_COMPOSITE_DEPTH}"
            );
        }
        if !visiting.insert(name) {
            anyhow::bail!("command '{name}': cycle detected in composite command");
        }
        if let Some(CommandSpec::Composite(c)) = self.commands.get(name) {
            for sub in &c.commands {
                let sub_str = sub.as_str();
                if !known.contains(sub_str) {
                    anyhow::bail!("command '{name}': references unknown command '{sub_str}'");
                }
                // Only recurse into config-defined composites; externals are
                // opaque from this side and may be exec or composite — their
                // internal cycles, if any, would be caught by their own
                // validate path, not this one.
                if let Some(CommandSpec::Composite(_)) = self.commands.get(sub_str) {
                    self.walk_composite(sub_str, known, visiting, depth + 1)?;
                }
            }
        }
        visiting.remove(name);
        Ok(())
    }

    /// Find the canonical command name for an alias.
    /// Returns `Some(command_name)` if the alias matches a command's aliases list.
    ///
    /// O(N·M) over commands × aliases. The alias lookup is called once per
    /// CLI invocation so an inline scan is still cheap in practice — each
    /// user has tens of commands and a handful of aliases.
    pub fn resolve_alias(&self, alias: &str) -> Option<&str> {
        for (name, spec) in &self.commands {
            if spec.aliases().iter().any(|a| a == alias) {
                return Some(name.as_str());
            }
        }
        None
    }
}

/// TRAIT-4 / TASK-0872: `Default` is intentionally test-only. Production
/// code uses [`Config::empty`] (explicit blank slate) or
/// [`load_config_or_default`] (user-visible defaults). The serde defaults
/// on individual fields do not require `Config: Default`.
#[cfg(any(test, feature = "test-support"))]
impl Default for Config {
    fn default() -> Self {
        Self::empty()
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
fn scale_columns(width: u16) -> u16 {
    let scaled = u32::from(width) * 9 / 10;
    u16::try_from(scaled).unwrap_or(u16::MAX)
}

fn is_default_columns(v: &u16) -> bool {
    *v == AUTO_COLUMNS
}

impl OutputConfig {
    /// Effective column width for rendering. When `columns` is the auto
    /// sentinel (`0`), probe the terminal at call time; otherwise honour the
    /// pinned config value. READ-5 / TASK-1219.
    #[must_use]
    pub fn resolve_columns(&self) -> u16 {
        if self.columns == AUTO_COLUMNS {
            terminal_size::terminal_size()
                .map(|(w, _)| scale_columns(w.0))
                .unwrap_or(FALLBACK_COLUMNS)
        } else {
            self.columns
        }
    }
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

    let mut config = Config::empty();

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
