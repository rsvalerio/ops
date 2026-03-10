//! Hierarchical configuration parsing and command resolution.
//!
//! Resolution order: internal default → global config → local `.ops.toml` → env vars.

pub mod theme_types;
pub mod tools;

use crate::config::theme_types::ThemeConfig;
use crate::config::tools::ToolSpec;
use crate::serde_defaults;
use anyhow::Context;
use config as config_crate;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{debug, instrument};

/// Root configuration structure.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default)]
    pub commands: IndexMap<String, CommandSpec>,
    #[serde(default)]
    pub data: DataConfig,
    #[serde(default)]
    pub themes: IndexMap<String, ThemeConfig>,
    #[serde(default)]
    pub extensions: ExtensionConfig,
    #[serde(default)]
    pub stack: Option<String>,
    #[serde(default)]
    pub tools: IndexMap<String, ToolSpec>,
}

/// Extension configuration.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionConfig {
    /// List of extension names to enable. Empty = no extensions.
    /// If None (missing from config), all compiled-in extensions are enabled.
    pub enabled: Option<Vec<String>>,
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

/// Overlay configuration with optional fields — only explicitly-set values
/// overwrite the base config during merging.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigOverlay {
    #[serde(default)]
    pub output: Option<OutputConfigOverlay>,
    #[serde(default)]
    pub commands: Option<IndexMap<String, CommandSpec>>,
    #[serde(default)]
    pub data: Option<DataConfigOverlay>,
    #[serde(default)]
    pub themes: Option<IndexMap<String, ThemeConfig>>,
    #[serde(default)]
    pub extensions: Option<ExtensionConfigOverlay>,
    #[serde(default)]
    pub stack: Option<String>,
    #[serde(default)]
    pub tools: Option<IndexMap<String, ToolSpec>>,
}

/// Overlay for extension settings.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionConfigOverlay {
    pub enabled: Option<Vec<String>>,
}

/// Overlay for data settings.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataConfigOverlay {
    pub path: Option<PathBuf>,
}

/// Overlay for output settings — each field is optional so partial configs
/// don't overwrite intentional base values with defaults.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputConfigOverlay {
    pub theme: Option<String>,
    pub columns: Option<u16>,
    pub show_error_detail: Option<bool>,
}

/// Output and theme settings.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OutputConfig {
    /// Theme name (built-in: "classic", "compact"; or custom theme from [themes]).
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Line width in columns for step lines (command + spacer + time). No runtime change.
    #[serde(default = "default_columns")]
    pub columns: u16,
    /// When true (default), show error details (exit status, stderr tail) inline
    /// below the failed step line. When false, only the step line with failure icon is shown.
    #[serde(default = "serde_defaults::default_true")]
    pub show_error_detail: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            columns: default_columns(),
            show_error_detail: true,
        }
    }
}

fn default_theme() -> String {
    "classic".into()
}

fn default_columns() -> u16 {
    80
}

/// Command definition: either a single exec or a composite of multiple commands.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CommandSpec {
    Exec(ExecCommandSpec),
    Composite(CompositeCommandSpec),
}

/// Single executable command.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecCommandSpec {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
    /// Timeout in seconds; None means no timeout.
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

impl ExecCommandSpec {
    pub fn timeout(&self) -> Option<Duration> {
        self.timeout_secs.map(Duration::from_secs)
    }

    /// Format as a display string for CLI step lines (e.g. "cargo build --all-targets").
    ///
    /// EFF-ANTI-001: Returns a reference to the program string when args is empty,
    /// avoiding an unnecessary clone. The returned Cow<str> allows the caller to
    /// either use the borrowed reference or convert to owned as needed.
    pub fn display_cmd(&self) -> std::borrow::Cow<'_, str> {
        if self.args.is_empty() {
            std::borrow::Cow::Borrowed(&self.program)
        } else {
            std::borrow::Cow::Owned(format!("{} {}", self.program, self.args.join(" ")))
        }
    }
}

/// Composite command: runs multiple commands (sequential or parallel).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompositeCommandSpec {
    pub commands: Vec<String>,
    #[serde(default)]
    pub parallel: bool,
    /// When true (default), stop remaining steps on first failure. When false, run all steps.
    #[serde(default = "serde_defaults::default_true")]
    pub fail_fast: bool,
}

/// Command identifier (name used in config and CLI).
///
/// Command IDs are string keys in the config's `commands` map. They are used:
/// - As CLI arguments: `cargo ops <command_id>`
/// - In composite command definitions: `commands = ["build", "test"]`
/// - In error messages and event streams
///
/// The type alias provides semantic clarity and could be changed to a newtype
/// for stronger type safety if needed.
pub type CommandId = String;

/// Default config content from `src/.default.ops.toml` (embedded at build; used as base config and for `cargo ops init`).
/// Build fails if the file is missing.
pub fn default_ops_toml() -> &'static str {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/.default.ops.toml"
    ))
}

/// Build merged init template: base config plus stack default commands when a stack is detected at `workspace_root`.
/// Used by `cargo ops init` to write a stack-aware `.ops.toml`.
pub fn init_template(workspace_root: &Path) -> anyhow::Result<String> {
    let mut config: Config =
        toml::from_str(default_ops_toml()).context("failed to parse internal default config")?;
    if let Some(stack) = crate::stack::Stack::detect(workspace_root) {
        for (id, spec) in stack.default_commands() {
            config.commands.insert(id, spec);
        }
        config.stack = Some(stack.as_str().to_string());
    }
    toml::to_string_pretty(&config).context("failed to serialize init config")
}

fn merge_field<T>(base: &mut T, overlay: Option<T>) {
    if let Some(v) = overlay {
        *base = v;
    }
}

fn merge_indexmap<K: Clone + Eq + std::hash::Hash, V: Clone>(
    base: &mut IndexMap<K, V>,
    overlay: &Option<IndexMap<K, V>>,
) {
    if let Some(items) = overlay {
        for (k, v) in items {
            base.insert(k.clone(), v.clone());
        }
    }
}

fn merge_output(base: &mut OutputConfig, overlay: &OutputConfigOverlay) {
    merge_field(&mut base.theme, overlay.theme.clone());
    merge_field(&mut base.columns, overlay.columns);
    merge_field(&mut base.show_error_detail, overlay.show_error_detail);
}

/// Merge overlay into base — only explicitly-set values overwrite.
///
/// Uses destructuring so adding a field to the overlay types without
/// handling it here causes a compile error.
pub fn merge_config(base: &mut Config, overlay: &ConfigOverlay) {
    let ConfigOverlay {
        output,
        commands,
        data,
        themes,
        extensions,
        stack,
        tools,
    } = overlay;

    if let Some(output_overlay) = output {
        merge_output(&mut base.output, output_overlay);
    }
    merge_indexmap(&mut base.commands, commands);
    if let Some(data_overlay) = data {
        if let Some(path) = &data_overlay.path {
            base.data.path = Some(path.clone());
        }
    }
    merge_indexmap(&mut base.themes, themes);
    if let Some(ext_overlay) = extensions {
        if let Some(enabled) = &ext_overlay.enabled {
            base.extensions.enabled = Some(enabled.clone());
        }
    }
    if let Some(s) = stack {
        base.stack = Some(s.clone());
    }
    merge_indexmap(&mut base.tools, tools);
}

/// Load and merge configuration from all sources.
///
/// Order (later overrides earlier): internal default → global file → local `.ops.toml` → env vars.
///
/// # Architecture (CQ-022)
///
/// The configuration merge follows a strict order, where each stage can override
/// values from previous stages:
///
/// 1. **Internal default** (`default_ops_toml`): Embedded TOML with all defaults
/// 2. **Global file** (`~/.config/ops/config.toml`): User-wide settings
/// 3. **Local `.ops.toml`**: Project-specific settings
/// 4. **`.ops.d/*.toml`**: Additional local overrides (alphabetical order)
/// 5. **Environment variables** (`OPS__*`): Runtime overrides
///
/// This order enables:
/// - Teams to share `.ops.toml` in version control
/// - Individuals to override via global config
/// - CI to override via environment variables
///
/// # Trust model
///
/// Local `.ops.toml` files are **implicitly trusted**, similar to `Makefile`, `.envrc`,
/// or `.cargo/config.toml`. They can specify arbitrary programs and arguments that
/// `cargo ops` will execute. Users should only run `cargo ops` in directories they trust,
/// just as they would only run `make` or `cargo build` in trusted repositories.
///
/// # Secrets
///
/// Do NOT store secrets (API keys, tokens, passwords) in `.ops.toml` files or the
/// `env` section of command definitions. These values may be visible in process listings,
/// logs, or error messages. Use environment variables directly or a secrets manager
/// instead.
///
/// Merge environment variables with OPS prefix into config.
///
/// Only applies overlay when OPS__ prefixed env vars exist.
/// Without this guard, the `config` crate deserializes an empty config with
/// all-default values, and merge_config unconditionally overwrites the local
/// config's intentional settings.
fn merge_env_vars(config: &mut Config) {
    let has_ops_env = std::env::vars().any(|(k, _)| k.starts_with("OPS__"));
    if !has_ops_env {
        return;
    }
    let env_config = config_crate::Config::builder()
        .add_source(config_crate::Environment::with_prefix("OPS").separator("__"))
        .build();
    if let Ok(merged) = env_config {
        if let Ok(env_overlay) = merged.try_deserialize::<ConfigOverlay>() {
            merge_config(config, &env_overlay);
        }
    }
}

#[instrument(skip_all)]
pub fn load_config() -> anyhow::Result<Config> {
    let mut config: Config =
        toml::from_str(default_ops_toml()).context("failed to parse internal default config")?;
    debug!("loaded internal default config");

    load_global_config(&mut config);

    let local_path = PathBuf::from(".ops.toml");
    if let Some(overlay) = read_config_file(&local_path) {
        debug!(path = %local_path.display(), "merging local config");
        merge_config(&mut config, &overlay);
    }

    merge_conf_d(&mut config);

    merge_env_vars(&mut config);

    debug!(command_count = config.commands.len(), "config loaded");
    Ok(config)
}

pub fn read_config_file(path: &Path) -> Option<ConfigOverlay> {
    let s = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "config file read error"
            );
            return None;
        }
    };
    match toml::from_str(&s) {
        Ok(c) => Some(c),
        Err(e) => {
            tracing::error!(
                path = %path.display(),
                error = %e,
                "config file parse error — check TOML syntax"
            );
            None
        }
    }
}

/// Read sorted `.toml` files from a directory, returning None if the directory
/// doesn't exist or can't be read.
fn read_conf_d_files(dir: &Path) -> Option<Vec<PathBuf>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            tracing::warn!(
                path = %dir.display(),
                error = %e,
                "failed to read .ops.d directory"
            );
            return None;
        }
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    files.sort();
    Some(files)
}

fn merge_conf_d(config: &mut Config) {
    let Some(files) = read_conf_d_files(Path::new(".ops.d")) else {
        return;
    };
    for path in files {
        if let Some(overlay) = read_config_file(&path) {
            debug!(path = %path.display(), "merging conf.d config");
            merge_config(config, &overlay);
        }
    }
}

/// CQ-001: Path to global config file (e.g. ~/.config/ops/config.toml).
///
/// Respects `XDG_CONFIG_HOME` when set; falls back to `$HOME/.config/`.
///
/// # Security (SEC-003)
///
/// This function trusts the `HOME`, `USERPROFILE`, and `XDG_CONFIG_HOME` environment
/// variables to locate the config directory. In most environments, this is safe because:
///
/// - These variables are set by the shell/session manager
/// - They typically point to the user's home directory
/// - An attacker with control over these variables already has significant access
fn global_config_path() -> Option<PathBuf> {
    let config_dir = if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(xdg)
    } else {
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
        PathBuf::from(home).join(".config")
    };
    Some(config_dir.join("ops/config"))
}

/// CQ-001: Extracted helper for loading global config, reducing nesting in load_config().
fn load_global_config(config: &mut Config) {
    let Some(global_path) = global_config_path() else {
        return;
    };
    let to_try = [global_path.with_extension("toml"), global_path];
    for path in &to_try {
        if let Some(overlay) = read_config_file(path) {
            debug!(path = %path.display(), "merging global config");
            merge_config(config, &overlay);
            return;
        }
    }
}

#[cfg(test)]
mod tests;
