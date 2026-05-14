//! Command specs and command identifiers.
//!
//! Extracted from `config/mod.rs` (ARCH-1 / TASK-0343) so that adding a
//! field to `ExecCommandSpec` or `CompositeCommandSpec` does not require
//! editing the same 600-line file as `Config` and the overlay structs.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::serde_defaults;

/// Command definition: either a single exec or a composite of multiple commands.
///
/// Custom `Deserialize` (ERR-1 / TASK-1430): picks the variant from the
/// presence of `program` (Exec) or `commands` (Composite) before delegating,
/// so a typo like `progam = "echo"` surfaces as the *Exec* error
/// ("unknown field `progam`") instead of the misleading Composite
/// ("missing field `commands`") that `#[serde(untagged)]` produced.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum CommandSpec {
    Exec(ExecCommandSpec),
    Composite(CompositeCommandSpec),
}

impl<'de> Deserialize<'de> for CommandSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        use serde::de::{Error, IntoDeserializer};
        let value = toml::Value::deserialize(deserializer)?;
        let table = value
            .as_table()
            .ok_or_else(|| D::Error::custom("command spec must be a table"))?;
        let has_program = table.contains_key("program");
        let has_commands = table.contains_key("commands");
        if has_program && has_commands {
            return Err(D::Error::custom(
                "command spec has both `program` (Exec) and `commands` (Composite); pick one",
            ));
        }
        // Classify by which variant's exclusive fields appear most often.
        // When neither variant's discriminating key is present (e.g. a typo
        // like `progam` instead of `program`) we still need to pick a
        // variant so the user sees an Exec/Composite-specific error
        // ("unknown field `progam`") rather than the misleading
        // "missing field `commands`" that `#[serde(untagged)]` produced.
        const COMPOSITE_KEYS: &[&str] = &["commands", "parallel", "fail_fast"];
        let composite_score = COMPOSITE_KEYS
            .iter()
            .filter(|k| table.contains_key(**k))
            .count();
        let pick_composite = has_commands || (!has_program && composite_score > 0);
        if pick_composite {
            CompositeCommandSpec::deserialize(value.into_deserializer())
                .map(CommandSpec::Composite)
                .map_err(D::Error::custom)
        } else {
            ExecCommandSpec::deserialize(value.into_deserializer())
                .map(CommandSpec::Exec)
                .map_err(D::Error::custom)
        }
    }
}

/// Shared metadata accessors implemented by every [`CommandSpec`] variant
/// (`ExecCommandSpec`, `CompositeCommandSpec`). Lets [`CommandSpec`] dispatch
/// `help` / `category` / `aliases` without one match arm per variant per
/// accessor — adding a variant only requires implementing this trait.
pub trait CommandMeta {
    fn help(&self) -> Option<&str>;
    fn category(&self) -> Option<&str>;
    fn aliases(&self) -> &[String];
}

impl CommandSpec {
    fn meta(&self) -> &dyn CommandMeta {
        match self {
            CommandSpec::Exec(e) => e,
            CommandSpec::Composite(c) => c,
        }
    }

    /// Return the help text for this command, if any.
    pub fn help(&self) -> Option<&str> {
        self.meta().help()
    }

    /// Return the category for this command, if any.
    pub fn category(&self) -> Option<&str> {
        self.meta().category()
    }

    /// Return the aliases for this command.
    pub fn aliases(&self) -> &[String] {
        self.meta().aliases()
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

impl CommandMeta for ExecCommandSpec {
    fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }
    fn category(&self) -> Option<&str> {
        self.category.as_deref()
    }
    fn aliases(&self) -> &[String] {
        &self.aliases
    }
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
    ///
    /// ERR-1 (TASK-1445): rejects NUL and other control characters
    /// (`< 0x20` except `\t`) in `program`, every `args` element, and `cwd`
    /// so a bad config fails at load with a named error instead of a
    /// cryptic `EINVAL` at spawn time.
    ///
    /// ERR-1 / SEC (TASK-1431): rejects relative `cwd` containing `..`
    /// components — the symmetric SEC-25 hardening for `ops run <cmd>`
    /// under a hostile workspace config.
    pub fn validate(&self, name: &str) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.program.is_empty(),
            "command '{name}': program must not be empty"
        );
        if let Some(0) = self.timeout_secs {
            anyhow::bail!("command '{name}': timeout_secs must be greater than 0");
        }
        check_control_chars(name, "program", &self.program)?;
        for (idx, arg) in self.args.iter().enumerate() {
            check_control_chars(name, &format!("args[{idx}]"), arg)?;
        }
        if let Some(cwd) = &self.cwd {
            let cwd_str = cwd.to_string_lossy();
            check_control_chars(name, "cwd", &cwd_str)?;
            if cwd
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            {
                anyhow::bail!(
                    "command '{name}': cwd must not contain '..' components (got {cwd_str:?})"
                );
            }
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
    pub fn display_cmd(&self) -> Cow<'_, str> {
        if self.args.is_empty() {
            Cow::Borrowed(&self.program)
        } else {
            Cow::Owned(format!(
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
    ///
    /// ERR-7 (TASK-0576): uses the strict [`Variables::try_expand`] so a
    /// non-UTF-8 / unparsable env var produces a visible diagnostic in the
    /// dry-run preview rather than silently rendering the literal `${VAR}`
    /// while a `tracing` event hides in the log buffer.
    pub fn expanded_args_display(
        &self,
        vars: &crate::expand::Variables,
    ) -> Result<Option<String>, crate::expand::ExpandError> {
        if self.args.is_empty() {
            return Ok(None);
        }
        let mut expanded = Vec::with_capacity(self.args.len());
        for arg in &self.args {
            expanded.push(vars.try_expand(arg)?.into_owned());
        }
        Ok(Some(join_shell_quoted(&expanded)))
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
pub(crate) fn shell_quote(value: &str) -> Cow<'_, str> {
    let safe = !value.is_empty()
        && value.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, '_' | '/' | '.' | ':' | '=' | '@' | '%' | '+' | ',' | '-')
        });
    if safe {
        Cow::Borrowed(value)
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
        Cow::Owned(out)
    }
}

/// ERR-1 (TASK-1445): reject embedded NUL or any C0 control byte
/// (`< 0x20`) other than horizontal tab. Catches typos like
/// `program = "\u{0}"`, embedded newlines, and CR/LF smuggling at load
/// time with a named field rather than a `EINVAL` at spawn.
fn check_control_chars(name: &str, field: &str, value: &str) -> anyhow::Result<()> {
    if let Some((idx, ch)) = value
        .chars()
        .enumerate()
        .find(|(_, c)| (*c as u32) < 0x20 && *c != '\t')
    {
        anyhow::bail!(
            "command '{name}': {field} contains control character U+{code:04X} at position {idx}",
            code = ch as u32,
        );
    }
    Ok(())
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

impl CommandMeta for CompositeCommandSpec {
    fn help(&self) -> Option<&str> {
        self.help.as_deref()
    }
    fn category(&self) -> Option<&str> {
        self.category.as_deref()
    }
    fn aliases(&self) -> &[String] {
        &self.aliases
    }
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

impl std::str::FromStr for CommandId {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_string()))
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
