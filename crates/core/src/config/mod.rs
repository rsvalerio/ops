//! Hierarchical configuration parsing and command resolution.
//!
//! Resolution order: internal default → global config → local `.ops.toml` → env vars.

mod edit;
mod loader;
pub(crate) mod merge;
pub mod theme_types;
pub mod tools;

pub use edit::{edit_ops_toml, read_ops_toml, write_ops_toml};

#[cfg(test)]
pub(crate) use loader::global_config_path;
pub use loader::{load_config, read_config_file};
pub use merge::merge_config;

use crate::config::theme_types::ThemeConfig;
use crate::config::tools::ToolSpec;
use crate::serde_defaults;
use anyhow::Context;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

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
    pub about: Option<AboutConfigOverlay>,
    #[serde(default)]
    pub stack: Option<String>,
    #[serde(default)]
    pub tools: Option<IndexMap<String, ToolSpec>>,
}

/// Generate a single-field overlay struct (DUP-3 collapse).
///
/// `ExtensionConfigOverlay`, `AboutConfigOverlay`, and `DataConfigOverlay`
/// all followed the same shape: one `Option<T>` field plus
/// `serde(deny_unknown_fields)`. Adding another single-field overlay used to
/// mean copy-pasting the entire struct + derives + doc comment; the macro
/// keeps the surface identical across all three so drift can't creep in.
macro_rules! single_field_overlay {
    ($( #[$meta:meta] )* $name:ident, $field:ident : $ty:ty) => {
        $( #[$meta] )*
        #[derive(Debug, Clone, Default, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            pub $field: Option<$ty>,
        }
    };
}

single_field_overlay!(
    /// Overlay for extension settings.
    ExtensionConfigOverlay, enabled: Vec<String>
);

single_field_overlay!(
    /// Overlay for about settings.
    AboutConfigOverlay, fields: Vec<String>
);

single_field_overlay!(
    /// Overlay for data settings.
    DataConfigOverlay, path: PathBuf
);

/// Overlay for output settings — each field is optional so partial configs
/// don't overwrite intentional base values with defaults.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputConfigOverlay {
    pub theme: Option<String>,
    pub columns: Option<u16>,
    pub show_error_detail: Option<bool>,
    pub stderr_tail_lines: Option<usize>,
    pub category_order: Option<Vec<String>>,
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

/// Command definition: either a single exec or a composite of multiple commands.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CommandSpec {
    Exec(ExecCommandSpec),
    Composite(CompositeCommandSpec),
}

impl CommandSpec {
    /// Return the help text for this command, if any.
    pub fn help(&self) -> Option<&str> {
        match self {
            CommandSpec::Exec(e) => e.help.as_deref(),
            CommandSpec::Composite(c) => c.help.as_deref(),
        }
    }

    /// Return the category for this command, if any.
    pub fn category(&self) -> Option<&str> {
        match self {
            CommandSpec::Exec(e) => e.category.as_deref(),
            CommandSpec::Composite(c) => c.category.as_deref(),
        }
    }

    /// Return the aliases for this command.
    pub fn aliases(&self) -> &[String] {
        match self {
            CommandSpec::Exec(e) => &e.aliases,
            CommandSpec::Composite(c) => &c.aliases,
        }
    }

    /// Fallback description when no `help` text is set.
    pub fn display_cmd_fallback(&self) -> String {
        match self {
            CommandSpec::Exec(e) => e.display_cmd().into_owned(),
            CommandSpec::Composite(c) => c.commands.join(", "),
        }
    }
}

/// Single executable command.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
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
    /// Short help text shown in `ops --help`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    /// Alternative names that can be used to invoke this command.
    #[serde(default, alias = "alias", skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Category for grouping in help output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

impl ExecCommandSpec {
    /// Build a minimal [`ExecCommandSpec`] from `program` and `args`.
    ///
    /// Preferred over struct-literal syntax because [`ExecCommandSpec`] is
    /// `#[non_exhaustive]`: downstream crates cannot use `..Default::default()`
    /// syntax and must go through this constructor. Adjust the remaining
    /// fields (`env`, `cwd`, `timeout_secs`, `help`, `aliases`, `category`)
    /// via direct field access — they remain `pub`.
    #[must_use]
    pub fn new(
        program: impl Into<String>,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    /// Validate fields that would cause confusing errors at execution time.
    pub fn validate(&self, name: &str) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.program.is_empty(),
            "command '{name}': program must not be empty"
        );
        if let Some(0) = self.timeout_secs {
            anyhow::bail!("command '{name}': timeout_secs must be greater than 0");
        }
        Ok(())
    }

    pub fn timeout(&self) -> Option<Duration> {
        self.timeout_secs.map(Duration::from_secs)
    }

    /// Format as a display string for CLI step lines (e.g. "cargo build --all-targets").
    ///
    /// SEC-21: each argument is shell-quoted so an arg containing whitespace,
    /// quotes, `;`, newlines, or backticks renders unambiguously. The actual
    /// exec uses argv directly via `tokio::process::Command::args` (no shell
    /// involved), but this string is what users see in dry-run output, step
    /// lines, and TAP files when auditing `.ops.toml` — a misleading
    /// space-only join could lead an operator to greenlight a config they
    /// would otherwise reject.
    pub fn display_cmd(&self) -> std::borrow::Cow<'_, str> {
        if self.args.is_empty() {
            std::borrow::Cow::Borrowed(&self.program)
        } else {
            std::borrow::Cow::Owned(format!(
                "{} {}",
                shell_quote(&self.program),
                join_shell_quoted(&self.args)
            ))
        }
    }

    /// Expand and join args for display; returns None when args is empty.
    /// SEC-21: see `display_cmd`. Each expanded argument is shell-quoted so
    /// values containing whitespace or metacharacters cannot be confused
    /// with multiple separate arguments.
    pub fn expanded_args_display(&self, vars: &crate::expand::Variables) -> Option<String> {
        if self.args.is_empty() {
            None
        } else {
            let expanded: Vec<String> = self
                .args
                .iter()
                .map(|a| vars.expand(a).into_owned())
                .collect();
            Some(join_shell_quoted(&expanded))
        }
    }
}

/// SEC-21: render `value` for display so the result is an unambiguous
/// single shell word.
///
/// - Strings of the safe set `[A-Za-z0-9_/.:=@%+,-]` (no whitespace, no
///   quotes, no shell metacharacters) are returned verbatim.
/// - Anything else is wrapped in single quotes; embedded single quotes are
///   escaped using the standard `'\''` close-escape-reopen sequence.
///
/// This is POSIX-shell-correct: the resulting string round-trips through
/// `sh -c` as one word identical to `value`. Keeps the common case (flags,
/// paths) uncluttered while ensuring `cargo build --config evil="; rm -rf /"`
/// renders as a single word in dry-run output.
pub(crate) fn shell_quote(value: &str) -> std::borrow::Cow<'_, str> {
    let safe = !value.is_empty()
        && value.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, '_' | '/' | '.' | ':' | '=' | '@' | '%' | '+' | ',' | '-')
        });
    if safe {
        std::borrow::Cow::Borrowed(value)
    } else {
        let mut out = String::with_capacity(value.len() + 2);
        out.push('\'');
        for c in value.chars() {
            if c == '\'' {
                out.push_str("'\\''");
            } else {
                out.push(c);
            }
        }
        out.push('\'');
        std::borrow::Cow::Owned(out)
    }
}

fn join_shell_quoted(parts: &[String]) -> String {
    parts
        .iter()
        .map(|p| shell_quote(p).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Composite command: runs multiple commands (sequential or parallel).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct CompositeCommandSpec {
    pub commands: Vec<String>,
    #[serde(default)]
    pub parallel: bool,
    /// When true (default), stop remaining steps on first failure. When false, run all steps.
    #[serde(default = "serde_defaults::default_true")]
    pub fail_fast: bool,
    /// Short help text shown in `ops --help`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    /// Alternative names that can be used to invoke this command.
    #[serde(default, alias = "alias", skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Category for grouping in help output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}

impl CompositeCommandSpec {
    /// Build a sequential, fail-fast composite from a list of command names.
    ///
    /// Preferred over struct-literal syntax because [`CompositeCommandSpec`]
    /// is `#[non_exhaustive]`. Adjust `parallel`, `fail_fast`, `help`,
    /// `aliases`, `category` via direct field access.
    #[must_use]
    pub fn new(commands: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            commands: commands.into_iter().map(Into::into).collect(),
            parallel: false,
            fail_fast: true,
            help: None,
            aliases: Vec::new(),
            category: None,
        }
    }
}

/// Command identifier (name used in config and CLI).
///
/// Newtype wrapper around `String` for compile-time type safety: prevents
/// accidentally passing display labels, program names, or error messages
/// where a command ID is expected.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct CommandId(String);

impl CommandId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for CommandId {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for CommandId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for CommandId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CommandId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<String> for CommandId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for CommandId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl PartialEq<str> for CommandId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for CommandId {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<String> for CommandId {
    fn eq(&self, other: &String) -> bool {
        self.0 == *other
    }
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
