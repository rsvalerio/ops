//! `[extend.<name>]` sections: append commands to an existing composite at
//! load time.
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
//! Overlay merging concatenates the per-target lists across layers (global →
//! `.ops.toml` → `.ops.d`), and [`apply`] materializes the result into
//! `Config::commands` after every layer has merged: a config-defined target
//! is appended to in place (shadow semantics — a local `[commands.verify]`
//! wins over the stack default and is then extended), a stack-default target
//! is cloned, appended, and inserted into `Config::commands`. Every consumer
//! (runner resolution, hooks, help) already consults `Config::commands`
//! first, so no downstream changes are needed.
//!
//! Appended names are not resolved here — like any composite entry they are
//! checked when the plan expands, so a stack default can be extended with a
//! name that only exists at runtime (an extension-registered `deps`, `sec`).

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{CommandSpec, Config};

/// One `[extend.<target>]` entry: the commands to append to `target`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExtendEntry {
    /// Command names appended to the target composite's `commands` list.
    ///
    /// Required (no serde default) so an entry like `[extend.verify]` with a
    /// typo'd key fails with "missing field `commands`" instead of silently
    /// extending nothing.
    pub commands: Vec<String>,
}

/// Apply every `[extend.<target>]` entry to `config`.
///
/// The target is looked up in `config.commands` first, then in the detected
/// stack's default commands (resolved from `config.stack` + `workspace_root`).
/// Extending an exec command or a name that is defined nowhere is an error:
/// both are near-certain typos, and a silent no-op would hide them behind a
/// `verify` that quietly skips the intended step.
///
/// No-op (and free) when `config.extend` is empty.
///
/// # Errors
///
/// If a target is not a defined command, or is defined as an exec command —
/// only composites (`commands = [...]`) can be extended.
pub(super) fn apply(config: &mut Config, workspace_root: &Path) -> anyhow::Result<()> {
    if config.extend.is_empty() {
        return Ok(());
    }
    let stack = crate::stack::Stack::resolve(config.stack.as_deref(), workspace_root);
    let defaults = stack.map(|s| s.default_commands_ref());

    for (target, entry) in &config.extend {
        if let Some(spec) = config.commands.get_mut(target) {
            match spec {
                CommandSpec::Composite(c) => c.commands.extend(entry.commands.iter().cloned()),
                CommandSpec::Exec(_) => anyhow::bail!(
                    "[extend.{target}]: target is an exec command; only composites \
                     (commands = [...]) can be extended"
                ),
            }
        } else if let Some(default_spec) = defaults.and_then(|d| d.get(target)) {
            match default_spec {
                CommandSpec::Composite(default) => {
                    let mut extended = default.clone();
                    extended.commands.extend(entry.commands.iter().cloned());
                    config
                        .commands
                        .insert(target.clone(), CommandSpec::Composite(extended));
                }
                CommandSpec::Exec(_) => anyhow::bail!(
                    "[extend.{target}]: target is an exec command; only composites \
                     (commands = [...]) can be extended"
                ),
            }
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
                CommandSpec::Exec(_) => None,
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
            },
        );
        let overlay = super::super::ConfigOverlay {
            extend: Some(IndexMap::from([(
                "verify".to_string(),
                ExtendEntry {
                    commands: vec!["second".to_string()],
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
