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
//! Overlay merging concatenates the per-target lists across layers (global →
//! `.ops.toml` → `.ops.d`), and [`apply`] materializes the result into
//! `Config::commands` after every layer has merged: a config-defined target
//! is appended to in place (shadow semantics — a local `[commands.verify]`
//! wins over the stack default and is then extended), a stack-default target
//! is cloned, appended, and inserted into `Config::commands`. Every consumer
//! (runner resolution, hooks, help) already consults `Config::commands`
//! first, so no downstream changes are needed.
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

use super::{CommandSpec, Config};

/// One `[extend.<target>]` entry: what to append to `target`.
///
/// Exactly one field applies per target kind — `commands` for composites,
/// `args` for exec commands — and [`apply`] rejects the mismatched pair
/// naming the target. Both are serde-optional; an entry that sets neither
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

/// Apply one entry to a target spec of matching kind.
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
            c.commands.extend(entry.commands.iter().cloned());
        }
        CommandSpec::Exec(e) => {
            if !entry.commands.is_empty() {
                anyhow::bail!(
                    "[extend.{target}]: target is an exec command; \
                     `commands` extends composites — use `args`"
                );
            }
            append_exec_args(&mut e.args, &entry.args);
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
    Ok(())
}

/// Apply every `[extend.<target>]` entry to `config`.
///
/// The target is looked up in `config.commands` first, then in the detected
/// stack's default commands (resolved from `config.stack` + `workspace_root`).
/// Extending a name that is defined nowhere — or with neither `commands` nor
/// `args` — is an error: both are near-certain typos, and a silent no-op
/// would hide them behind a `verify` that quietly skips the intended step.
///
/// No-op (and free) when `config.extend` is empty.
///
/// # Errors
///
/// If a target is not a defined command, an entry sets neither `commands`
/// nor `args`, or the entry's field does not match the target's kind
/// (see [`apply_to_spec`]).
pub(super) fn apply(config: &mut Config, workspace_root: &Path) -> anyhow::Result<()> {
    if config.extend.is_empty() {
        return Ok(());
    }
    let stack = crate::stack::Stack::resolve(config.stack.as_deref(), workspace_root);
    let defaults = stack.map(|s| s.default_commands_ref());

    for (target, entry) in &config.extend {
        if entry.commands.is_empty() && entry.args.is_empty() {
            anyhow::bail!(
                "[extend.{target}]: entry sets neither `commands` nor `args` \
                 (a typo'd key would otherwise extend nothing)"
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
                args: Vec::new(),
            },
        );
        config
    }

    fn config_with_extend_args(target: &str, args: &[&str]) -> Config {
        let mut config = Config::empty();
        config.extend.insert(
            target.to_string(),
            ExtendEntry {
                commands: Vec::new(),
                args: args.iter().map(|s| (*s).to_string()).collect(),
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
            msg.contains("neither"),
            "error must say the entry sets nothing: {msg}"
        );
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
                args: Vec::new(),
            },
        );
        let overlay = super::super::ConfigOverlay {
            extend: Some(IndexMap::from([(
                "verify".to_string(),
                ExtendEntry {
                    commands: vec!["second".to_string()],
                    args: Vec::new(),
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
}
