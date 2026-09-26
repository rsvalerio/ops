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

use super::strategy::{substitute, MatrixCell, MatrixRefError, Strategy};
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
    /// TASK-2273: a `[commands.<name>] clone = "<source>"` declaration. A
    /// load-time placeholder only — `config::clone::apply` materializes it
    /// into a concrete Exec/Composite copy of the source before anything
    /// downstream runs, so consumers only meet this variant through direct
    /// `Config` deserialization that bypassed the loader.
    Clone(CloneCommandSpec),
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
        // TASK-2273: `clone` is a third discriminator. It is mutually
        // exclusive with the copied payload fields — `program`/`args` come
        // from the source, and extras belong in `[extend.<name>]` — so the
        // combination is a named error rather than a serde "unknown field"
        // that leaves the user guessing where the override goes.
        if table.contains_key("clone") {
            for key in ["program", "args", "commands"] {
                if table.contains_key(key) {
                    return Err(D::Error::custom(format!(
                        "command spec sets both `clone` and `{key}`; a clone copies the \
                         source's `{key}` — add extras via [extend.<name>]"
                    )));
                }
            }
            return CloneCommandSpec::deserialize(value.into_deserializer())
                .map(CommandSpec::Clone)
                .map_err(D::Error::custom);
        }
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
/// Resolve the program every `ops`-re-invoking registration should spawn.
///
/// SEC-13 / TASK-2122: a bare `"ops"` is resolved through the invoking
/// environment's `PATH`, so an `ops` shim earlier on `PATH` (a stale
/// `~/.cargo/bin` entry, a direnv-injected dir, a vendored CI copy)
/// silently becomes the binary that runs — and `PATH` is not cleared on the
/// exec path. Anything that gates a pre-commit hook or drives the process
/// exit code must therefore spawn an absolute path instead.
///
/// Prefer [`std::env::current_exe`] (absolute, robust under renamed/aliased
/// shells) and fall back to the literal `"ops"` only when that lookup fails
/// (e.g. unusual sandboxing), where `PATH` resolves it. The single shared
/// helper is deliberate: the runner's builtin store and the extensions
/// register the same command ids, and two private copies of this resolution
/// are exactly how the extension half previously shipped the unsafe bare
/// name while the builtin half did not. Set
/// [`ExecCommandSpec::display_program`] to `"ops"` alongside it so the
/// rendered step line stays `ops <subcommand>`.
#[must_use]
pub fn current_ops_program() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.into_os_string().into_string().ok())
        .unwrap_or_else(|| "ops".to_string())
}

impl CommandSpec {
    fn meta(&self) -> &dyn CommandMeta {
        match self {
            Self::Exec(e) => e,
            Self::Composite(c) => c,
            Self::Clone(c) => c,
        }
    }

    /// Return the help text for this command, if any.
    #[must_use]
    pub fn help(&self) -> Option<&str> {
        self.meta().help()
    }

    /// Return the category for this command, if any.
    #[must_use]
    pub fn category(&self) -> Option<&str> {
        self.meta().category()
    }

    /// Return the aliases for this command.
    #[must_use]
    pub fn aliases(&self) -> &[String] {
        self.meta().aliases()
    }

    /// Fallback description when no `help` text is set.
    #[must_use]
    pub fn display_cmd_fallback(&self) -> String {
        match self {
            Self::Exec(e) => e.display_cmd().into_owned(),
            Self::Composite(c) => c.commands.join(", "),
            Self::Clone(c) => format!("clone of {}", c.clone_source()),
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
    /// Run this step alone when its plan runs in parallel.
    ///
    /// A parallel plan is split into ordered stages at every exclusive step:
    /// the exclusive step runs by itself, and each run of consecutive
    /// non-exclusive steps between them runs concurrently. Stages follow plan
    /// order, so a formatter listed first finishes before anything after it
    /// starts. No effect in a sequential plan.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exclusive: bool,
    /// Display-only program name that overrides `program` in rendered
    /// command lines (see [`Self::display_cmd`]). Spawn behaviour is
    /// unchanged — `program` is still what executes.
    ///
    /// Internal: set by the runner's builtin registrations and by extensions
    /// that re-invoke `ops` via [`current_ops_program`], both of which spawn
    /// through `current_exe()` (an absolute path that would otherwise render
    /// as `/home/…/bin/ops sec` instead of `ops sec`). Deliberately
    /// `serde(skip)` + `deny_unknown_fields`, so a `.ops.toml` cannot set
    /// it: a config-supplied display name diverging from the real program
    /// is exactly the misleading-render hazard SEC-21 guards against.
    #[serde(skip)]
    pub display_program: Option<String>,
    /// Run the command once per matrix cell (TASK-2277); see
    /// [`crate::config::Strategy`]. The matrix is still one step to the
    /// enclosing plan — [`Self::exclusive`] covers every cell.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy: Option<Strategy>,
}

/// One cell of a matrix command: the cell's values and the command with
/// every `${matrix.<key>}` substituted (and no strategy of its own).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct MatrixCellSpec {
    /// The cell's step id and progress label, `name [key=value, ...]`.
    pub id: String,
    pub cell: MatrixCell,
    pub spec: ExecCommandSpec,
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
    /// fields (`env`, `cwd`, `timeout_secs`, `help`, `aliases`, `category`,
    /// `exclusive`)
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

    /// Build a spec that re-invokes the running `ops` binary with `subcommand`.
    ///
    /// The one constructor for ops-internal commands (runner builtins and
    /// extensions), so they cannot drift on:
    ///
    /// - **program**: [`current_ops_program`], never a bare `"ops"` resolved
    ///   through `PATH`, where a shim could shadow it (SEC-13 / TASK-2122);
    /// - **display**: rendered as `ops <subcommand>`, not the absolute path;
    /// - **scheduling**: [`Self::exclusive`] defaults to `true`. An ops
    ///   subcommand may rewrite the worktree, so it opts in to overlapping
    ///   other steps with `exclusive = false`; forgetting to costs
    ///   parallelism, never a lost edit. Specs parsed from config keep the
    ///   `false` default.
    #[must_use]
    pub fn ops_subcommand(subcommand: &str) -> Self {
        let mut spec = Self::new(current_ops_program(), [subcommand]);
        spec.display_program = Some("ops".to_string());
        spec.exclusive = true;
        spec
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
    ///
    /// SEC-11 (TASK-1826): the `env` map is screened too — see
    /// [`Self::validate_env`]. Every string this spec hands to
    /// `std::process::Command` now passes through this function.
    ///
    /// # Errors
    ///
    /// If `program` is empty, `timeout_secs` is `Some(0)`, any field
    /// contains control characters, or an `env` key contains `=`.
    pub fn validate(&self, name: &str) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.program.is_empty(),
            "command '{name}': program must not be empty"
        );
        if self.timeout_secs == Some(0) {
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
        self.validate_env(name)?;
        self.validate_matrix(name)
    }

    /// TASK-2277: `${matrix.<key>}` references must resolve in every cell,
    /// and a command without a strategy must not use them — neither falls
    /// back to the environment, so both fail the load naming the command.
    /// `program` and `env` keys are never substituted, so a reference there
    /// is an error either way.
    fn validate_matrix(&self, name: &str) -> anyhow::Result<()> {
        let no_match = |_: &str| None::<&str>;
        if let Err(e) = substitute(&self.program, no_match) {
            anyhow::bail!(
                "command '{name}': program contains {e}; ${{matrix.<key>}} is substituted \
                 only in args, env values and cwd"
            );
        }
        for key in self.env.keys() {
            if let Err(e) = substitute(key, no_match) {
                anyhow::bail!(
                    "command '{name}': env key {key:?} contains {e}; ${{matrix.<key>}} is \
                     substituted only in args, env values and cwd"
                );
            }
        }
        if self.strategy.is_none() {
            for (field, value) in self.matrix_fields() {
                if let Err(e) = substitute(&value, no_match) {
                    anyhow::bail!(
                        "command '{name}': {field} references {e}, but the command has no \
                         [commands.{name}.strategy] matrix"
                    );
                }
            }
            return Ok(());
        }
        for cell in self.matrix_cells(name)? {
            cell.spec.validate(&cell.id)?;
        }
        Ok(())
    }

    /// The fields `${matrix.<key>}` is substituted into, as `(name, value)`,
    /// `env` in sorted key order so the first reported error is stable.
    fn matrix_fields(&self) -> Vec<(String, Cow<'_, str>)> {
        let mut fields: Vec<(String, Cow<'_, str>)> = self
            .args
            .iter()
            .enumerate()
            .map(|(i, a)| (format!("args[{i}]"), Cow::Borrowed(a.as_str())))
            .collect();
        let mut env: Vec<(&String, &String)> = self.env.iter().collect();
        env.sort_unstable();
        for (key, value) in env {
            fields.push((format!("env[{key}]"), Cow::Borrowed(value.as_str())));
        }
        if let Some(cwd) = &self.cwd {
            fields.push(("cwd".to_string(), cwd.to_string_lossy()));
        }
        fields
    }

    /// Expand a matrix command into one spec per cell (TASK-2277), in cell
    /// order. Each cell spec has every `${matrix.<key>}` substituted in
    /// `args`, `env` values and `cwd`, and no strategy. Empty for a command
    /// without a strategy.
    ///
    /// # Errors
    ///
    /// If the matrix is malformed (see [`crate::config::Matrix::cells`]),
    /// `max_parallel` is zero, or a reference names a key some cell does not
    /// define — each naming the command.
    pub fn matrix_cells(&self, name: &str) -> anyhow::Result<Vec<MatrixCellSpec>> {
        let Some(strategy) = &self.strategy else {
            return Ok(Vec::new());
        };
        if strategy.max_parallel == Some(0) {
            anyhow::bail!(
                "command '{name}': strategy.max_parallel must be at least 1 (1 runs the \
                 cells one at a time)"
            );
        }
        let cells = strategy
            .matrix
            .cells()
            .map_err(|e| anyhow::anyhow!("command '{name}': strategy.matrix: {e}"))?;
        let mut out = Vec::with_capacity(cells.len());
        for cell in cells {
            let fill = |field: &str, value: &str| -> anyhow::Result<String> {
                substitute(value, |k| cell.get(k))
                    .map(Cow::into_owned)
                    .map_err(|e| match e {
                        MatrixRefError::Unknown(_) => anyhow::anyhow!(
                            "command '{name}': {field} references {e}, which cell [{}] \
                             does not define",
                            cell.describe()
                        ),
                        MatrixRefError::Unterminated => {
                            anyhow::anyhow!("command '{name}': {field} contains {e}")
                        }
                    })
            };
            let mut spec = self.clone();
            spec.strategy = None;
            for (i, arg) in spec.args.iter_mut().enumerate() {
                *arg = fill(&format!("args[{i}]"), arg)?;
            }
            let mut env: Vec<(&String, &mut String)> = spec.env.iter_mut().collect();
            env.sort_unstable_by(|a, b| a.0.cmp(b.0));
            for (key, value) in env {
                *value = fill(&format!("env[{key}]"), value)?;
            }
            if let Some(cwd) = &mut spec.cwd {
                if let Some(raw) = cwd.to_str() {
                    *cwd = PathBuf::from(fill("cwd", raw)?);
                }
            }
            out.push(MatrixCellSpec {
                id: cell.label(name),
                cell,
                spec,
            });
        }
        Ok(out)
    }

    /// SEC-11 / TASK-1826: screen the `env` map with the same
    /// control-character policy the rest of [`Self::validate`] applies.
    ///
    /// `crates/runner/src/command/build.rs` hands `env` straight to
    /// `Command::env`, so an unscreened NUL surfaces as std's anonymous
    /// `InvalidInput` — "nul byte found in provided data" — with no command,
    /// no field, and no offending key: exactly the cryptic spawn failure the
    /// validator exists to prevent, on the one field it did not cover.
    ///
    /// Keys carry one rule values do not: an `=` inside a key produces a Unix
    /// environment entry the child's `getenv` can never retrieve, so it is
    /// rejected at load rather than silently spawned.
    ///
    /// Keys are visited in sorted order so a config with several bad entries
    /// always reports the same one; `HashMap` iteration order would otherwise
    /// make the diagnostic — and any test pinning it — nondeterministic.
    fn validate_env(&self, name: &str) -> anyhow::Result<()> {
        let mut keys: Vec<&str> = self.env.keys().map(String::as_str).collect();
        keys.sort_unstable();
        for key in keys {
            check_control_chars(name, &format!("env key {key:?}"), key)?;
            anyhow::ensure!(
                !key.contains('='),
                "command '{name}': env key {key:?} must not contain '='; \
                 the child process could never look such a variable up"
            );
            // `keys` was built from `self.env`, so this lookup always hits.
            if let Some(value) = self.env.get(key) {
                check_control_chars(name, &format!("env[{key}]"), value)?;
            }
        }
        Ok(())
    }

    pub fn timeout(&self) -> Option<Duration> {
        self.timeout_secs.map(Duration::from_secs)
    }

    /// Format as a display string for CLI step lines (e.g. "cargo build --all-targets").
    ///
    /// When [`Self::display_program`] is set, that name renders in place of
    /// `program` — spawn still uses `program`. Builtins use this so a
    /// `current_exe()`-spawned command displays as `ops sec`, matching the
    /// extension-registered `ops check-json` style.
    ///
    /// SEC-21: each argument is shell-quoted so an arg containing whitespace,
    /// quotes, `;`, newlines, or backticks renders unambiguously. The actual
    /// exec uses argv directly via `tokio::process::Command::args` (no shell
    /// involved), but this string is what users see in dry-run output, step
    /// lines, and TAP files when auditing `.ops.toml` — a misleading
    /// space-only join could lead an operator to greenlight a config they
    /// would otherwise reject.
    #[must_use]
    pub fn display_cmd(&self) -> Cow<'_, str> {
        let program = self.display_program.as_deref().unwrap_or(&self.program);
        if self.args.is_empty() {
            return shell_quote(program);
        }
        Cow::Owned(format!(
            "{} {}",
            shell_quote(program),
            join_shell_quoted(&self.args)
        ))
    }

    /// Expand and join args for display; returns None when args is empty.
    /// SEC-21: see `display_cmd`. Each expanded argument is shell-quoted so
    /// values containing whitespace or metacharacters cannot be confused
    /// with multiple separate arguments.
    ///
    /// ERR-7 (TASK-0576): uses the strict [`crate::expand::Variables::try_expand`] so a
    /// non-UTF-8 / unparsable env var produces a visible diagnostic in the
    /// dry-run preview rather than silently rendering the literal `${VAR}`
    /// while a `tracing` event hides in the log buffer.
    ///
    /// # Errors
    ///
    /// [`ExpandError`](crate::expand::ExpandError) if any argument references an
    /// undefined variable or a value that is not valid Unicode.
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
pub fn shell_quote(value: &str) -> Cow<'_, str> {
    let safe = !value.is_empty()
        && value.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(c, '_' | '/' | '.' | ':' | '=' | '@' | '%' | '+' | ',' | '-')
        });
    if safe {
        Cow::Borrowed(value)
    } else {
        let mut out = String::with_capacity(value.len().saturating_add(2));
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
        .find(|(_, c)| u32::from(*c) < 0x20 && *c != '\t')
    {
        anyhow::bail!(
            "command '{name}': {field} contains control character U+{code:04X} at position {idx}",
            code = u32::from(ch),
        );
    }
    Ok(())
}

/// PERF-3 / TASK-1412: render each arg via [`shell_quote`] and join with
/// spaces directly into a single pre-sized `String`, so the safe-arg fast
/// path can stay borrowed from [`shell_quote`]'s `Cow::Borrowed` instead
/// of paying for an intermediate `Vec<String>` plus per-arg
/// `Cow::into_owned()`.
fn join_shell_quoted(parts: &[String]) -> String {
    // Lower-bound capacity: every part contributes at least its raw bytes
    // plus a separating space. The unsafe-quoting path pushes a few more
    // bytes; treating that as a rare overflow keeps the common dry-run
    // render to a single allocation.
    let cap = parts
        .iter()
        .map(|p| p.len().saturating_add(1))
        .sum::<usize>();
    let mut out = String::with_capacity(cap);
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(&shell_quote(part));
    }
    out
}

/// Composite command: runs multiple commands (sequential or parallel).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct CompositeCommandSpec {
    pub commands: Vec<String>,
    /// Run this group's steps concurrently.
    ///
    /// A `parallel = true` group flattens its whole subtree into one leaf
    /// plan that the runner schedules as a single unit (split into stages at
    /// exclusive steps). A `parallel = false` group runs each entry as its
    /// own plan with that entry's own schedule (TASK-2275). A parallel group
    /// containing a `parallel = false` group is rejected at expansion time
    /// with `ExpandError::ConflictingSchedule`. Inside a parallel plan, keep
    /// a step from overlapping the others with [`ExecCommandSpec::exclusive`].
    /// See the "Command groups and scheduling" section of `README.md`.
    #[serde(default)]
    pub parallel: bool,
    /// When true (default), stop remaining steps on first failure. When false, run all steps.
    ///
    /// Within one parallel plan every group must declare the same value
    /// (TASK-1657: the plan is scheduled as one unit). Across a sequential
    /// group's entries the values may differ: each entry's own value governs
    /// its steps, and the group's own value governs whether the sequence
    /// stops after a failing entry (TASK-2275).
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

/// TASK-2273: a `[commands.<name>] clone = "<source>"` declaration — define
/// `name` as a copy of an existing command (typically a stack default) under
/// a new name, at load time.
///
/// The source is resolved against `[commands]` and the detected stack's
/// defaults (the same lookup `[extend.<target>]` uses), and the *resolved*
/// spec is copied. Scalar fields set beside `clone` override the copy;
/// `program`/`args`/`commands` are rejected at parse time (see
/// [`CommandSpec::deserialize`]) — extra args go through
/// `[extend.<name>] args = [...]`, so the two features compose and there is
/// one rule for where appended args land (before `--`).
///
/// Exec-only overrides (`env`, `cwd`, `timeout_secs`, `exclusive`, `strategy`) set beside
/// a composite source are load errors; a composite clone can only override
/// `help`, `category` and `aliases`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct CloneCommandSpec {
    /// Name of the command to copy: a `[commands]` entry or a stack default.
    pub clone: String,
    /// Short help text shown in `ops --help`; replaces the source's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    /// Alternative names for the clone, used verbatim (empty means no
    /// aliases). Never inherited from the source: aliases are identity, and
    /// a copy would either collide at `validate_aliases` or shadow the
    /// source's alias at dispatch.
    #[serde(default, alias = "alias", skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Category for grouping in help output; replaces the source's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Environment map; replaces the source's `env` when set (exec sources
    /// only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    /// Working directory; overrides the source's `cwd` when set (exec
    /// sources only). TOML has no null, so a source `cwd` cannot be cleared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,
    /// Timeout in seconds; overrides the source's `timeout_secs` when set
    /// (exec sources only). Cannot clear a source timeout.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    /// Parallel-scheduling flag; overrides the source's `exclusive` when set
    /// (exec sources only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclusive: Option<bool>,
    /// Matrix strategy; replaces the source's `strategy` wholesale when set
    /// (exec sources only). Without it the clone copies the source's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy: Option<Strategy>,
}

impl CommandMeta for CloneCommandSpec {
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

impl CloneCommandSpec {
    /// Build a bare clone declaration from its source name.
    ///
    /// Preferred over struct-literal syntax because [`CloneCommandSpec`] is
    /// `#[non_exhaustive]`. Adjust the override fields (`help`, `aliases`,
    /// `category`, `env`, `cwd`, `timeout_secs`, `exclusive`, `strategy`) via direct
    /// field access.
    #[must_use]
    pub fn new(clone: impl Into<String>) -> Self {
        Self {
            clone: clone.into(),
            help: None,
            aliases: Vec::new(),
            category: None,
            env: None,
            cwd: None,
            timeout_secs: None,
            exclusive: None,
            strategy: None,
        }
    }

    /// The name this declaration copies. Field is called `clone` to match
    /// the config key; the accessor keeps call sites that also handle
    /// `std::clone::Clone` readable.
    #[must_use]
    pub fn clone_source(&self) -> &str {
        &self.clone
    }
}
