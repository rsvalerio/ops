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
