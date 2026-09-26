//! `[extend.<name>]` sections: append commands to an existing composite, or
//! args to an existing exec command, at load time.
//!
//! Lets a workspace add steps to a stack default (or its own composite)
//! without copying the whole `commands = [...]` list into `.ops.toml`, where
//! it would go stale whenever the default evolves:
//!
//! ```toml
//! [extend.verify]
//! commands = ["my-new-command"]
//! ```
//!
//! or one flag to an exec command without re-declaring its `program`/`args`
//! (TASK-2272 — the exec twin of the same staleness problem):
//!
//! ```toml
//! [extend.clippy]
//! args = ["--locked"]
//! ```
//!
//! TASK-2274: an entry may also override the target's `help` (and
//! `category`, for symmetry) — and when a composite's command list grows
//! without a `help` override, the existing help is extended to name the
//! appended commands, so `ops --help` can never silently understate what
//! `ops --dry-run` runs:
//!
//! ```toml
//! [extend.verify]
//! commands = ["my-new-command"]
//! help = "Run fmt, clippy, build, doc in parallel, then my-new-command"
//! ```
//!
//! Overlay merging concatenates the per-target lists across layers (global →
//! `.ops.toml` → `.ops.d`), while `help`/`category` replace (the last layer
//! wins), and [`apply`] materializes the result into `Config::commands`
//! after every layer has merged: a config-defined target is appended to in
//! place (shadow semantics — a local `[commands.verify]` wins over the stack
//! default and is then extended), a stack-default target is cloned,
//! appended, and inserted into `Config::commands`. Every consumer (runner
//! resolution, hooks, help) already consults `Config::commands` first, so no
//! downstream changes are needed.
//!
//! Appended args land *before* the target's first `--` separator when one is
//! present (see [`append_exec_args`]): a cargo invocation like
//! `clippy ... -- -D warnings` treats everything after `--` as lint flags, so
//! a naive append would silently turn `--locked` into a lint selector.
//!
//! Appended names are not resolved here — like any composite entry they are
//! checked when the plan expands, so a stack default can be extended with a
//! name that only exists at runtime (an extension-registered `deps`, `sec`).

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{CommandSpec, Config, ExecCommandSpec, Matrix};

/// One `[extend.<target>]` entry: what to append to (or override on)
/// `target`.
///
/// Exactly one list field applies per target kind — `commands` for
/// composites, `args` for exec commands — and [`apply`] rejects the
/// mismatched pair naming the target. `help` / `category` apply to either
/// kind. All fields are serde-optional; an entry that sets none of them
/// (typically a typo'd key) is rejected at apply time rather than silently
/// extending nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExtendEntry {
    /// Command names appended to the target composite's `commands` list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<String>,
    /// Args appended to the target exec command's `args` list — before its
    /// first `--` separator when one is present (see [`append_exec_args`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Replaces the target's `help` (either kind). Without it, appending
    /// `commands` to a composite that has help extends the help to name the
    /// appended commands (see [`apply_to_spec`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    /// Replaces the target's `category` (either kind).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Appended to the target exec command's `strategy.matrix` (TASK-2277):
    /// `matrix.<key> = [...]` extends an existing axis, `include` /
    /// `exclude` entries extend those lists (see [`append_matrix`]).
    #[serde(default, skip_serializing_if = "Matrix::is_empty")]
    pub matrix: Matrix,
}

/// Splice `extra` into `args` before the first `--` separator, or append at
/// the end when there is none.
///
/// Cargo-style invocations (`clippy ... -- -D warnings`, `test -- --nocapture`)
/// pass everything after `--` to the wrapped tool, so a plain append would
/// silently reinterpret a cargo flag as a tool flag. Inserting before the
/// separator keeps the extension a cargo flag; targets without a separator
/// (build, fmt, plain programs) get a plain append.
fn append_exec_args(args: &mut Vec<String>, extra: &[String]) {
    match args.iter().position(|a| a == "--") {
        // Reverse insertion at a fixed index: each insert pushes the
        // previously-spliced elements up, leaving `extra` in order at `pos`
        // without index arithmetic (workspace denies arithmetic_side_effects).
        Some(pos) => {
            for arg in extra.iter().rev() {
                args.insert(pos, arg.clone());
            }
        }
        None => args.extend(extra.iter().cloned()),
    }
}

/// Append an `[extend.<target>] matrix` entry to the target's strategy
/// (TASK-2277).
///
/// Only existing axes can grow: a key the target's matrix does not declare
/// would silently multiply every cell by a new dimension and is far more
/// likely a typo (`matrix.crat`), so it is an error naming the key — as is
/// extending a command that has no strategy.
fn append_matrix(target: &str, spec: &mut ExecCommandSpec, extra: &Matrix) -> anyhow::Result<()> {
    let Some(strategy) = &mut spec.strategy else {
        anyhow::bail!(
            "[extend.{target}]: `matrix` extends a command's strategy, but '{target}' has \
             no [commands.{target}.strategy]"
        );
    };
    if let Some(key) = extra
        .axes
        .keys()
        .find(|key| !strategy.matrix.axes.contains_key(*key))
    {
        let axes: Vec<&str> = strategy.matrix.axes.keys().map(String::as_str).collect();
        anyhow::bail!(
            "[extend.{target}]: matrix.{key} names no axis of '{target}' (axes: {})",
            axes.join(", ")
        );
    }
    strategy.matrix.append(extra);
    Ok(())
}

/// Apply one entry to a target spec of matching kind.
///
/// TASK-2274: when a composite's `commands` list grows and the entry sets no
/// `help` override, an existing help text is extended to name the appended
/// commands (`"<old>; then <extras>"`) — the default outcome used to be a
/// stale help that understated the plan, and nobody noticed. A composite
/// without help needs nothing: the help fallback already renders the
/// materialized `commands` list.
///
/// # Errors
///
/// If the entry sets `commands` on an exec target or `args` on a composite
/// target, naming the target — both are near-certain copy-paste mistakes and
/// a silent no-op would hide them behind a command that quietly skips the
/// intended behaviour.
fn apply_to_spec(target: &str, spec: &mut CommandSpec, entry: &ExtendEntry) -> anyhow::Result<()> {
    match spec {
        CommandSpec::Composite(c) => {
            if !entry.args.is_empty() {
                anyhow::bail!(
                    "[extend.{target}]: target is a composite (commands = [...]); \
                     `args` extends exec commands — use `commands`"
                );
            }
            if !entry.matrix.is_empty() {
                anyhow::bail!(
                    "[extend.{target}]: target is a composite; `matrix` extends an exec \
                     command's [commands.{target}.strategy]"
                );
            }
            c.commands.extend(entry.commands.iter().cloned());
            if entry.help.is_none() && !entry.commands.is_empty() {
                if let Some(help) = &mut c.help {
                    help.push_str("; then ");
                    help.push_str(&entry.commands.join(", "));
                }
            }
        }
        CommandSpec::Exec(e) => {
            if !entry.commands.is_empty() {
                anyhow::bail!(
                    "[extend.{target}]: target is an exec command; \
                     `commands` extends composites — use `args`"
                );
            }
            append_exec_args(&mut e.args, &entry.args);
            if !entry.matrix.is_empty() {
                append_matrix(target, e, &entry.matrix)?;
            }
        }
        // The load path materializes clones (`clone::apply`) before this
        // runs; a Clone here means apply was called on a config that
        // bypassed the loader. Refuse it rather than extending a placeholder.
        CommandSpec::Clone(_) => {
            anyhow::bail!(
                "[extend.{target}]: target is an unmaterialized `clone` declaration; \
                 ops resolves clones at config load time"
            );
        }
    }
    apply_meta_overrides(spec, entry);
    Ok(())
}

/// Replace the target's `help` / `category` when the entry sets them
/// (TASK-2274). Applies to both composites and exec commands — the fields
/// exist on either kind, and an override-only entry (no `commands`/`args`)
/// is the one way to fix a stack default's help without restating its
/// command list.
fn apply_meta_overrides(spec: &mut CommandSpec, entry: &ExtendEntry) {
    let (help, category) = match spec {
        CommandSpec::Composite(c) => (&mut c.help, &mut c.category),
        CommandSpec::Exec(e) => (&mut e.help, &mut e.category),
        // `apply_to_spec` rejects clones before calling here.
        CommandSpec::Clone(_) => return,
    };
    if let Some(new_help) = &entry.help {
        *help = Some(new_help.clone());
    }
    if let Some(new_category) = &entry.category {
        *category = Some(new_category.clone());
    }
}

/// Apply every `[extend.<target>]` entry to `config`.
///
/// The target is looked up in `config.commands` first, then in the detected
/// stack's default commands (resolved from `config.stack` + `workspace_root`).
/// Extending a name that is defined nowhere — or with none of `commands`,
/// `args`, `help`, `category` — is an error: both are near-certain typos, and
/// a silent no-op would hide them behind a `verify` that quietly skips the
/// intended step.
///
/// No-op (and free) when `config.extend` is empty.
///
/// # Errors
///
/// If a target is not a defined command, an entry sets none of `commands`,
/// `args`, `help`, `category`, or the entry's list field does not match the
/// target's kind (see [`apply_to_spec`]).
pub(super) fn apply(config: &mut Config, workspace_root: &Path) -> anyhow::Result<()> {
    if config.extend.is_empty() {
        return Ok(());
    }
    let stack = crate::stack::Stack::resolve(config.stack.as_deref(), workspace_root);
    let defaults = stack.map(|s| s.default_commands_ref());

    for (target, entry) in &config.extend {
        if entry.commands.is_empty()
            && entry.args.is_empty()
            && entry.help.is_none()
            && entry.category.is_none()
            && entry.matrix.is_empty()
        {
            anyhow::bail!(
                "[extend.{target}]: entry sets none of `commands`, `args`, `help`, \
                 `category`, `matrix` (a typo'd key would otherwise extend nothing)"
            );
        }
        if let Some(spec) = config.commands.get_mut(target) {
            apply_to_spec(target, spec, entry)?;
        } else if let Some(default_spec) = defaults.and_then(|d| d.get(target)) {
            let mut extended = default_spec.clone();
            apply_to_spec(target, &mut extended, entry)?;
            config.commands.insert(target.clone(), extended);
        } else {
            anyhow::bail!(
                "[extend.{target}]: no command named '{target}' to extend \
                 (checked [commands] and the detected stack defaults)"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    fn config_with_extend(target: &str, commands: &[&str]) -> Config {
        let mut config = Config::empty();
        config.extend.insert(
            target.to_string(),
            ExtendEntry {
                commands: commands.iter().map(|s| (*s).to_string()).collect(),
                ..ExtendEntry::default()
            },
        );
        config
    }

    fn config_with_extend_args(target: &str, args: &[&str]) -> Config {
        let mut config = Config::empty();
        config.extend.insert(
            target.to_string(),
            ExtendEntry {
                args: args.iter().map(|s| (*s).to_string()).collect(),
                ..ExtendEntry::default()
            },
        );
        config
    }

    /// A workspace root that detects as the rust stack, so `verify` and
    /// friends exist as stack defaults.
    fn rust_workspace() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        dir
    }

    #[test]
    fn extends_stack_default_composite() {
        let dir = rust_workspace();
        let mut config = config_with_extend("verify", &["extra"]);
        apply(&mut config, dir.path()).expect("extend must apply");

        let Some(CommandSpec::Composite(verify)) = config.commands.get("verify") else {
            panic!("extended verify must be materialized as a config composite");
        };
        let default_len = crate::stack::Stack::Rust
            .default_commands_ref()
            .get("verify")
            .and_then(|s| match s {
                CommandSpec::Composite(c) => Some(c.commands.len()),
                CommandSpec::Exec(_) | CommandSpec::Clone(_) => None,
            })
            .unwrap_or(0);
        assert_eq!(
            verify.commands.len(),
            default_len + 1,
            "appended command must land at the end of the default list"
        );
        assert_eq!(verify.commands.last().map(String::as_str), Some("extra"));
    }

    /// The stack defaults live in a process-wide memoized cache; extending
    /// must clone-and-append, never mutate the cached spec. A mutation here
    /// would leak the appended step into every other test and every `ops`
    /// invocation in the same process.
    #[test]
    fn extending_does_not_mutate_the_stack_default_cache() {
        let dir = rust_workspace();
        let mut config = config_with_extend("verify", &["extra"]);
        apply(&mut config, dir.path()).unwrap();

        let cached = crate::stack::Stack::Rust.default_commands_ref();
        let Some(CommandSpec::Composite(verify)) = cached.get("verify") else {
            panic!("rust verify must be a composite");
        };
        assert_ne!(
            verify.commands.last().map(String::as_str),
            Some("extra"),
            "the memoized stack default must not be mutated by [extend]"
        );
    }

    #[test]
    fn config_defined_target_wins_and_is_appended_in_place() {
        let dir = rust_workspace();
        let mut config = config_with_extend("verify", &["extra"]);
        config.commands.insert(
            "verify".to_string(),
            CommandSpec::Composite(crate::config::CompositeCommandSpec::new(["fmt"])),
        );
        apply(&mut config, dir.path()).unwrap();

        let Some(CommandSpec::Composite(verify)) = config.commands.get("verify") else {
            panic!("verify must remain a composite");
        };
        assert_eq!(
            verify.commands,
            vec!["fmt".to_string(), "extra".to_string()],
            "the local shadow must be extended, not the stack default"
        );
    }

    #[test]
    fn missing_target_errors() {
        let dir = rust_workspace();
        let mut config = config_with_extend("nope", &["extra"]);
        let err = apply(&mut config, dir.path()).expect_err("unknown target must error");
        let msg = format!("{err:#}");
        assert!(msg.contains("nope"), "error must name the target: {msg}");
    }

    #[test]
    fn exec_target_errors() {
        let dir = rust_workspace();
        let mut config = config_with_extend("fmt", &["extra"]);
        let err = apply(&mut config, dir.path()).expect_err("exec target must error");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("exec command"),
            "error must say only composites extend: {msg}"
        );
        assert!(msg.contains("fmt"), "error must name the target: {msg}");
    }

    /// TASK-2272 #2: the motivating hazard. A naive append to the rust
    /// `clippy` default would land after the `--` separator and be read as a
    /// lint flag, not a cargo flag — the extension must splice in before it.
    #[test]
    fn exec_args_insert_before_separator() {
        let dir = rust_workspace();
        let mut config = config_with_extend_args("clippy", &["--locked"]);
        apply(&mut config, dir.path()).expect("extend must apply");

        let Some(CommandSpec::Exec(clippy)) = config.commands.get("clippy") else {
            panic!("extended clippy must be materialized as a config exec");
        };
        let sep = clippy
            .args
            .iter()
            .position(|a| a == "--")
            .expect("rust clippy default keeps its -- separator");
        assert!(
            clippy.args[..sep].contains(&"--locked".to_string()),
            "--locked must land before the -- separator, got {:?}",
            clippy.args
        );
        assert_eq!(
            clippy.args.last().map(String::as_str),
            Some("warnings"),
            "everything after -- must be untouched"
        );
    }

    #[test]
    fn exec_args_append_without_separator() {
        let dir = rust_workspace();
        let mut config = config_with_extend_args("build", &["--locked"]);
        apply(&mut config, dir.path()).expect("extend must apply");

        let Some(CommandSpec::Exec(build)) = config.commands.get("build") else {
            panic!("extended build must be materialized as a config exec");
        };
        assert_eq!(
            build.args.last().map(String::as_str),
            Some("--locked"),
            "no separator in the build default, so the args append at the end"
        );
    }

    /// Shadow semantics for exec targets: a local `[commands.<name>]` wins
    /// over the stack default and is then extended in place.
    #[test]
    fn config_defined_exec_wins_and_is_appended_in_place() {
        let dir = rust_workspace();
        let mut config = config_with_extend_args("doc", &["--document-private-items"]);
        config.commands.insert(
            "doc".to_string(),
            CommandSpec::Exec(crate::config::ExecCommandSpec::new(
                "cargo",
                ["doc", "--no-deps"],
            )),
        );
        apply(&mut config, dir.path()).unwrap();

        let Some(CommandSpec::Exec(doc)) = config.commands.get("doc") else {
            panic!("doc must remain an exec");
        };
        assert_eq!(
            doc.args,
            vec![
                "doc".to_string(),
                "--no-deps".to_string(),
                "--document-private-items".to_string(),
            ],
            "the local shadow must be extended, not the stack default"
        );
    }

    /// TASK-2272 #3: `args` on a composite is the exec/composite mismatch in
    /// its other form — same copy-paste mistake, same named error.
    #[test]
    fn composite_target_with_args_errors() {
        let dir = rust_workspace();
        let mut config = config_with_extend_args("verify", &["--flag"]);
        let err = apply(&mut config, dir.path()).expect_err("args on composite must error");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("composite"),
            "error must say only exec commands take args: {msg}"
        );
        assert!(msg.contains("verify"), "error must name the target: {msg}");
    }

    /// Both fields serde-default to empty, so a typo'd key (e.g.
    /// `comands = [...]`) would deserialize fine and silently extend
    /// nothing — the entry must be rejected at apply time instead.
    #[test]
    fn entry_with_neither_commands_nor_args_errors() {
        let dir = rust_workspace();
        let mut config = Config::empty();
        config
            .extend
            .insert("verify".to_string(), ExtendEntry::default());
        let err = apply(&mut config, dir.path()).expect_err("empty entry must error");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("none of"),
            "error must say the entry sets nothing: {msg}"
        );
    }

    /// TASK-2274 AC #1: `[extend.<name>] help = "..."` replaces the target's
    /// help verbatim — no auto-suffix, because the override is the user's
    /// statement of what the command now runs.
    #[test]
    fn help_override_replaces_the_targets_help() {
        let dir = rust_workspace();
        let mut config = config_with_extend("verify", &["extra"]);
        config.extend.insert(
            "verify".to_string(),
            ExtendEntry {
                commands: vec!["extra".to_string()],
                help: Some("Run the default gate, then extra".to_string()),
                ..ExtendEntry::default()
            },
        );
        apply(&mut config, dir.path()).expect("extend must apply");

        let Some(CommandSpec::Composite(verify)) = config.commands.get("verify") else {
            panic!("extended verify must be materialized as a config composite");
        };
        assert_eq!(
            verify.help.as_deref(),
            Some("Run the default gate, then extra"),
            "the override replaces the help verbatim"
        );
    }

    /// TASK-2274 AC #2: extending a composite's `commands` without a help
    /// override extends the help to name the appended commands, so the
    /// default rust `verify` help can never silently understate the plan
    /// (`ops --help` and `ops verify --dry-run` agree on what runs).
    #[test]
    fn extended_composite_help_names_the_appended_commands() {
        let dir = rust_workspace();
        let mut config = config_with_extend("verify", &["doc-default", "fuzz-fmt"]);
        apply(&mut config, dir.path()).expect("extend must apply");

        let Some(CommandSpec::Composite(verify)) = config.commands.get("verify") else {
            panic!("extended verify must be materialized as a config composite");
        };
        let default_help = crate::stack::Stack::Rust
            .default_commands_ref()
            .get("verify")
            .and_then(|s| s.help().map(str::to_string))
            .expect("rust verify default must carry help");
        assert_eq!(
            verify.help.as_deref(),
            Some(format!("{default_help}; then doc-default, fuzz-fmt").as_str()),
            "the help must name the appended commands"
        );
    }

    /// TASK-2274: a help-only entry overrides the text without appending
    /// anything — the one way to fix a stack default's help short of
    /// restating the composite (which `[extend]` exists to avoid).
    #[test]
    fn help_only_entry_overrides_without_appending() {
        let dir = rust_workspace();
        let mut config = Config::empty();
        config.extend.insert(
            "verify".to_string(),
            ExtendEntry {
                help: Some("The workspace verification gate".to_string()),
                ..ExtendEntry::default()
            },
        );
        apply(&mut config, dir.path()).expect("help-only entry must apply");

        let Some(CommandSpec::Composite(verify)) = config.commands.get("verify") else {
            panic!("verify must be materialized");
        };
        assert_eq!(
            verify.help.as_deref(),
            Some("The workspace verification gate")
        );
        let default_len = crate::stack::Stack::Rust
            .default_commands_ref()
            .get("verify")
            .and_then(|s| match s {
                CommandSpec::Composite(c) => Some(c.commands.len()),
                CommandSpec::Exec(_) | CommandSpec::Clone(_) => None,
            })
            .unwrap_or(0);
        assert_eq!(
            verify.commands.len(),
            default_len,
            "a help-only entry must not change the command list"
        );
    }

    /// TASK-2274: `category` overrides for symmetry, on an exec target —
    /// help/category apply to either kind.
    #[test]
    fn category_and_help_override_exec_target() {
        let dir = rust_workspace();
        let mut config = Config::empty();
        config.extend.insert(
            "clippy".to_string(),
            ExtendEntry {
                help: Some("Lint with the workspace's flags".to_string()),
                category: Some("Code Quality".to_string()),
                ..ExtendEntry::default()
            },
        );
        apply(&mut config, dir.path()).expect("extend must apply");

        let Some(CommandSpec::Exec(clippy)) = config.commands.get("clippy") else {
            panic!("clippy must be materialized as a config exec");
        };
        assert_eq!(
            clippy.help.as_deref(),
            Some("Lint with the workspace's flags")
        );
        assert_eq!(clippy.category.as_deref(), Some("Code Quality"));
    }

    /// TASK-2274: extending a composite that has *no* help leaves help
    /// unset — the help fallback already renders the materialized `commands`
    /// list, so there is nothing to keep accurate.
    #[test]
    fn extending_helpless_composite_leaves_help_unset() {
        let dir = rust_workspace();
        let mut config = Config::empty();
        config.commands.insert(
            "gate".to_string(),
            CommandSpec::Composite(crate::config::CompositeCommandSpec::new(["fmt"])),
        );
        config.extend.insert(
            "gate".to_string(),
            ExtendEntry {
                commands: vec!["build".to_string()],
                ..ExtendEntry::default()
            },
        );
        apply(&mut config, dir.path()).expect("extend must apply");

        let Some(CommandSpec::Composite(gate)) = config.commands.get("gate") else {
            panic!("gate must remain a composite");
        };
        assert_eq!(gate.help, None);
        assert_eq!(gate.commands, vec!["fmt".to_string(), "build".to_string()]);
    }

    #[test]
    fn empty_extend_section_is_a_no_op() {
        let dir = rust_workspace();
        let mut config = Config::empty();
        apply(&mut config, dir.path()).expect("empty [extend] must be a no-op");
        assert!(config.commands.is_empty());
    }

    /// The overlay merge concatenates per-target (merge.rs owns that rule);
    /// this pins the composed result end to end: both layers' appends land
    /// on the stack default, in layer order.
    #[test]
    fn entries_for_one_target_concatenate_in_order() {
        let dir = rust_workspace();
        let mut config = Config::empty();
        config.extend.insert(
            "verify".to_string(),
            ExtendEntry {
                commands: vec!["first".to_string()],
                ..ExtendEntry::default()
            },
        );
        let overlay = super::super::ConfigOverlay {
            extend: Some(IndexMap::from([(
                "verify".to_string(),
                ExtendEntry {
                    commands: vec!["second".to_string()],
                    ..ExtendEntry::default()
                },
            )])),
            ..Default::default()
        };
        super::super::merge_config(&mut config, overlay);
        apply(&mut config, dir.path()).unwrap();

        let Some(CommandSpec::Composite(verify)) = config.commands.get("verify") else {
            panic!("verify must be materialized");
        };
        let len = verify.commands.len();
        let tail: Vec<&str> = verify.commands[len.saturating_sub(2)..]
            .iter()
            .map(String::as_str)
            .collect();
        assert_eq!(tail, vec!["first", "second"]);
    }

    /// TASK-2274 regression (found in review of PR #63): an explicit `help`
    /// earlier layer must not mask the auto-suffix for commands a *later*
    /// layer appends without setting `help`. Before the merge-time suffix,
    /// the merged entry carried the earlier override, `apply_to_spec` saw
    /// `help` set and skipped its own suffix, and `ops --help` quietly
    /// understated the plan — the exact staleness `[extend]` exists to
    /// prevent. Each layer names only what it itself appended; the override
    /// author stays responsible for their own text.
    #[test]
    fn command_only_layer_after_help_override_still_names_its_commands() {
        let dir = rust_workspace();
        let mut config = Config::empty();
        config.extend.insert(
            "verify".to_string(),
            ExtendEntry {
                commands: vec!["first".to_string()],
                help: Some("Run the default gate, then first".to_string()),
                ..ExtendEntry::default()
            },
        );
        let overlay = super::super::ConfigOverlay {
            extend: Some(IndexMap::from([(
                "verify".to_string(),
                ExtendEntry {
                    commands: vec!["second".to_string()],
                    ..ExtendEntry::default()
                },
            )])),
            ..Default::default()
        };
        super::super::merge_config(&mut config, overlay);
        apply(&mut config, dir.path()).unwrap();

        let Some(CommandSpec::Composite(verify)) = config.commands.get("verify") else {
            panic!("verify must be materialized");
        };
        assert_eq!(
            verify.help.as_deref(),
            Some("Run the default gate, then first; then second"),
            "the later layer's appends must be named in the effective help"
        );
        let Some(last) = verify.commands.last() else {
            panic!("appended command must land");
        };
        assert_eq!(last, "second");
    }
}
