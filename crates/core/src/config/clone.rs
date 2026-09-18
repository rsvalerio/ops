//! `[commands.<name>] clone = "<source>"`: define a command as a copy of an
//! existing one (typically a stack default) at load time.
//!
//! TASK-2273: the motivating case is a workspace with a `fuzz/` root that
//! needs `fuzz-clippy`, `fuzz-build`, `fuzz-fmt` — the stack `clippy`/`build`
//! /`fmt` plus `--manifest-path fuzz/Cargo.toml`. Hand-writing each exec spec
//! silently diverges when the stack default's flags change; a clone tracks
//! the source, and the extra flag goes through `[extend.<clone>]`:
//!
//! ```toml
//! [commands.fuzz-clippy]
//! clone = "clippy"
//!
//! [extend.fuzz-clippy]
//! args = ["--manifest-path", "fuzz/Cargo.toml"]
//! ```
//!
//! [`apply`] runs **before** [`super::extend::apply`] (see
//! `loader::load_config_at`), so the copy is made from the source *before*
//! the source's own `[extend.<source>]` is applied: extends stay per-name,
//! and a clone never inherits its source's extends. `[extend.<clone>]` in
//! the same config then applies to the materialized copy — the two features
//! compose, and appended args keep their one rule (before `--`).
//!
//! The source resolves against `config.commands` and the detected stack's
//! defaults, the same lookup `[extend.<target>]` uses. Extension-registered
//! commands cannot be cloned: they register after config load, so a `clone`
//! naming one is an unknown-source load error (matching `[extend]`, which
//! cannot target them either).
//!
//! Clone-of-clone **chains** resolve in declaration order (a clone whose
//! source is another clone materializes once that source is concrete);
//! **cycles** — including self-clones — are load errors naming both names.

use std::path::Path;

use super::commands::{CloneCommandSpec, CommandSpec};
use super::Config;

/// Apply every `[commands.<name>] clone = "<source>"` declaration to
/// `config`, replacing each with a concrete copy of the resolved source.
///
/// Materializes into `config.commands` in place, so every downstream
/// consumer (runner resolution, hooks, help, dry-run) sees a normal
/// exec/composite spec with no clone awareness.
///
/// No-op (and free) when no command declares `clone`.
///
/// # Errors
///
/// If a clone source is defined nowhere, a clone cycle exists, the clone
/// target names an existing stack-default command, or an exec-only override
/// (`env`, `cwd`, `timeout_secs`, `exclusive`) is set beside a composite
/// source — each naming the clone and the source.
pub(super) fn apply(config: &mut Config, workspace_root: &Path) -> anyhow::Result<()> {
    let mut pending: Vec<String> = config
        .commands
        .iter()
        .filter(|(_, spec)| matches!(spec, CommandSpec::Clone(_)))
        .map(|(name, _)| name.clone())
        .collect();
    if pending.is_empty() {
        return Ok(());
    }
    let stack = crate::stack::Stack::resolve(config.stack.as_deref(), workspace_root);
    let defaults = stack.map(|s| s.default_commands_ref());

    // Fixpoint for clone-of-clone chains: each pass materializes the clones
    // whose source is already concrete (a config command or a stack
    // default); a source that is itself a pending clone retries next pass.
    // A pass with no progress means every remaining clone's source is a
    // pending clone — a cycle.
    while !pending.is_empty() {
        let mut progressed = false;
        // (clone name, its still-pending source), captured at push time so
        // the cycle error below needs no second lookup.
        let mut still_pending: Vec<(String, String)> = Vec::new();
        for name in &pending {
            let Some(CommandSpec::Clone(decl)) = config.commands.get(name) else {
                // Materialized earlier in this pass as another clone's
                // source; its own entry was already handled.
                continue;
            };
            let decl = decl.clone();
            match resolve_source(config, defaults, name, decl.clone_source())? {
                Resolved::Spec(spec) => {
                    let materialized = materialize(name, &decl, &spec)?;
                    // A clone must not silently shadow a stack default: the
                    // name collision is near-certainly a mistake about which
                    // names exist, unlike a plain `[commands.<name>]` shadow
                    // which is the documented way to override a default.
                    if defaults.is_some_and(|d| d.contains_key(name)) {
                        anyhow::bail!(
                            "command '{name}': clone target '{name}' is a stack-default \
                             command name; rename the clone or override the default with \
                             [commands.{name}] directly (clone of '{source}')",
                            source = decl.clone_source()
                        );
                    }
                    config.commands.insert(name.clone(), materialized);
                    progressed = true;
                }
                Resolved::PendingClone => {
                    still_pending.push((name.clone(), decl.clone_source().to_string()));
                }
            }
        }
        if still_pending.is_empty() {
            return Ok(());
        }
        if !progressed {
            // `still_pending` is non-empty here (returned above otherwise);
            // the `if let` keeps the fall-through total without an `expect`.
            if let Some((name, source)) = still_pending.first() {
                anyhow::bail!(
                    "command '{name}': clone cycle — '{name}' clones '{source}', which is \
                     itself a clone that never resolved"
                );
            }
        }
        pending = still_pending.into_iter().map(|(name, _)| name).collect();
    }
    Ok(())
}

/// Where a clone declaration's source resolved to.
enum Resolved {
    /// A concrete (non-clone) spec: a `[commands]` entry or a stack default.
    /// Boxed: `CommandSpec` is ~240 bytes and `PendingClone` is none, so an
    /// inline variant would pad every transient `Resolved` value.
    Spec(Box<CommandSpec>),
    /// The source is itself an unmaterialized clone; retry on a later pass.
    PendingClone,
}

/// Look up a clone's source: `config.commands` first, then the stack
/// defaults — the same precedence `[extend.<target>]` uses.
///
/// # Errors
///
/// If the source is defined nowhere, naming the clone and the source.
fn resolve_source(
    config: &Config,
    defaults: Option<&indexmap::IndexMap<String, CommandSpec>>,
    name: &str,
    source: &str,
) -> anyhow::Result<Resolved> {
    match config.commands.get(source) {
        Some(spec @ (CommandSpec::Exec(_) | CommandSpec::Composite(_))) => {
            Ok(Resolved::Spec(Box::new(spec.clone())))
        }
        Some(CommandSpec::Clone(_)) => Ok(Resolved::PendingClone),
        None => {
            if let Some(spec) = defaults.and_then(|d| d.get(source)) {
                Ok(Resolved::Spec(Box::new(spec.clone())))
            } else {
                anyhow::bail!(
                    "command '{name}': no command named '{source}' to clone \
                     (checked [commands] and the detected stack defaults)"
                );
            }
        }
    }
}

/// Copy `source` and apply the declaration's scalar overrides.
///
/// Given fields replace the copied field wholesale (`env` replaces, it does
/// not merge; `aliases` replace when non-empty). Fields left unset keep the
/// source's value. TOML has no null, so `cwd`/`timeout_secs` cannot be
/// cleared, only replaced.
///
/// # Errors
///
/// If an exec-only override (`env`, `cwd`, `timeout_secs`, `exclusive`) is
/// set beside a composite source — silently dropping it would hide a
/// copy-paste mistake behind a clone that quietly ignores it.
fn materialize(
    name: &str,
    decl: &CloneCommandSpec,
    source: &CommandSpec,
) -> anyhow::Result<CommandSpec> {
    let spec = match source {
        CommandSpec::Exec(e) => {
            let mut copy = e.clone();
            copy.help = decl.help.clone().or(copy.help);
            if !decl.aliases.is_empty() {
                copy.aliases.clone_from(&decl.aliases);
            }
            copy.category = decl.category.clone().or(copy.category);
            if let Some(env) = &decl.env {
                copy.env.clone_from(env);
            }
            if decl.cwd.is_some() {
                copy.cwd.clone_from(&decl.cwd);
            }
            if decl.timeout_secs.is_some() {
                copy.timeout_secs = decl.timeout_secs;
            }
            if let Some(exclusive) = decl.exclusive {
                copy.exclusive = exclusive;
            }
            CommandSpec::Exec(copy)
        }
        CommandSpec::Composite(c) => {
            for (field, is_set) in [
                ("env", decl.env.is_some()),
                ("cwd", decl.cwd.is_some()),
                ("timeout_secs", decl.timeout_secs.is_some()),
                ("exclusive", decl.exclusive.is_some()),
            ] {
                if is_set {
                    anyhow::bail!(
                        "command '{name}': clone of composite '{source_name}' sets \
                         `{field}`, which only exec commands take",
                        source_name = decl.clone_source()
                    );
                }
            }
            let mut copy = c.clone();
            copy.help = decl.help.clone().or(copy.help);
            if !decl.aliases.is_empty() {
                copy.aliases.clone_from(&decl.aliases);
            }
            copy.category = decl.category.clone().or(copy.category);
            CommandSpec::Composite(copy)
        }
        // `apply` only passes specs `resolve_source` deemed concrete;
        // reaching here means those two functions have drifted apart.
        CommandSpec::Clone(_) => {
            anyhow::bail!(
                "command '{name}': internal error — clone source '{}' was not \
                 materialized before copying",
                decl.clone_source()
            );
        }
    };
    Ok(spec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        CloneCommandSpec, CommandSpec, CompositeCommandSpec, Config, ExecCommandSpec,
    };

    /// A workspace root that detects as the rust stack, so `clippy`, `fmt`
    /// and `verify` exist as stack defaults.
    fn rust_workspace() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        dir
    }

    fn config_with_clone(name: &str, decl: CloneCommandSpec) -> Config {
        let mut config = Config::empty();
        config
            .commands
            .insert(name.to_string(), CommandSpec::Clone(decl));
        config
    }

    #[test]
    fn clones_stack_default_exec() {
        let dir = rust_workspace();
        let mut config = config_with_clone("fuzz-clippy", CloneCommandSpec::new("clippy"));
        apply(&mut config, dir.path()).expect("clone must apply");

        let Some(CommandSpec::Exec(fuzz_clippy)) = config.commands.get("fuzz-clippy") else {
            panic!("clone must materialize as a config exec");
        };
        let default = crate::stack::Stack::Rust
            .default_commands_ref()
            .get("clippy")
            .and_then(|s| match s {
                CommandSpec::Exec(e) => Some(e.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            fuzz_clippy.program, default.program,
            "clone copies the source program"
        );
        assert_eq!(
            fuzz_clippy.args, default.args,
            "clone copies the source args"
        );
    }

    #[test]
    fn clones_stack_default_composite() {
        let dir = rust_workspace();
        let mut config = config_with_clone("my-verify", CloneCommandSpec::new("verify"));
        apply(&mut config, dir.path()).expect("clone must apply");

        let Some(CommandSpec::Composite(my_verify)) = config.commands.get("my-verify") else {
            panic!("clone must materialize as a config composite");
        };
        assert!(
            !my_verify.commands.is_empty(),
            "clone copies the source command list"
        );
    }

    /// The stack defaults live in a process-wide memoized cache; cloning
    /// must copy, never hand out (or mutate) the cached spec itself.
    #[test]
    fn cloning_does_not_mutate_the_stack_default_cache() {
        let dir = rust_workspace();
        let mut config = config_with_clone("fuzz-fmt", CloneCommandSpec::new("fmt"));
        apply(&mut config, dir.path()).unwrap();

        let cached = crate::stack::Stack::Rust.default_commands_ref();
        let Some(CommandSpec::Exec(fmt)) = cached.get("fmt") else {
            panic!("rust fmt must be an exec");
        };
        assert_ne!(
            fmt.args.last().map(String::as_str),
            Some("--manifest-path"),
            "the memoized stack default must not be mutated by clone"
        );
    }

    /// Scalar overrides replace the copy; unset fields keep the source's.
    #[test]
    fn scalar_overrides_apply_to_the_copy() {
        let dir = rust_workspace();
        let mut decl = CloneCommandSpec::new("fmt");
        decl.help = Some("Format the fuzz targets".to_string());
        decl.timeout_secs = Some(120);
        decl.exclusive = Some(true);
        let mut config = config_with_clone("fuzz-fmt", decl);
        apply(&mut config, dir.path()).unwrap();

        let Some(CommandSpec::Exec(fmt)) = config.commands.get("fuzz-fmt") else {
            panic!("clone must materialize");
        };
        assert_eq!(fmt.help.as_deref(), Some("Format the fuzz targets"));
        assert_eq!(fmt.timeout_secs, Some(120));
        assert!(fmt.exclusive);
    }

    /// AC #2: `program`/`args`/`commands` beside `clone` are parse errors
    /// pointing at `[extend.<name>]` — pinned here at the [`CommandSpec`]
    /// deserialization layer, where the discriminator lives.
    #[test]
    fn payload_fields_beside_clone_are_parse_errors() {
        for toml in [
            "[commands.x]\nclone = \"clippy\"\nprogram = \"cargo\"\n",
            "[commands.x]\nclone = \"clippy\"\nargs = [\"--locked\"]\n",
            "[commands.x]\nclone = \"verify\"\ncommands = [\"fmt\"]\n",
        ] {
            let err = toml::from_str::<crate::config::Config>(toml)
                .expect_err("payload beside clone must be a parse error");
            let msg = err.to_string();
            assert!(
                msg.contains("[extend.<name>]"),
                "error must point at [extend.<name>]: {msg}"
            );
        }
    }

    /// AC #4: unknown source is a load error naming both names.
    #[test]
    fn unknown_source_errors_naming_both_names() {
        let dir = rust_workspace();
        let mut config = config_with_clone("fuzz-clippy", CloneCommandSpec::new("nope"));
        let err = apply(&mut config, dir.path()).expect_err("unknown source must error");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("fuzz-clippy"),
            "error must name the clone: {msg}"
        );
        assert!(msg.contains("nope"), "error must name the source: {msg}");
    }

    /// AC #4: cloning into an existing stack-default name is a load error
    /// naming both names.
    #[test]
    fn cloning_into_a_stack_default_name_errors() {
        let dir = rust_workspace();
        let mut config = config_with_clone("fmt", CloneCommandSpec::new("clippy"));
        let err = apply(&mut config, dir.path()).expect_err("clone over a default must error");
        let msg = format!("{err:#}");
        assert!(msg.contains("stack-default"), "error must say why: {msg}");
        assert!(msg.contains("'fmt'"), "error must name the target: {msg}");
        assert!(
            msg.contains("'clippy'"),
            "error must name the source: {msg}"
        );
    }

    /// AC #4: clone cycles (self and mutual) are load errors naming both
    /// names.
    #[test]
    fn clone_cycles_error() {
        let dir = rust_workspace();
        let mut config = config_with_clone("a", CloneCommandSpec::new("a"));
        let err = apply(&mut config, dir.path()).expect_err("self-clone must error");
        assert!(
            format!("{err:#}").contains("cycle"),
            "self-clone must be reported as a cycle: {err:#}"
        );

        let mut config = Config::empty();
        config.commands.insert(
            "a".to_string(),
            CommandSpec::Clone(CloneCommandSpec::new("b")),
        );
        config.commands.insert(
            "b".to_string(),
            CommandSpec::Clone(CloneCommandSpec::new("a")),
        );
        let err = apply(&mut config, dir.path()).expect_err("clone cycle must error");
        let msg = format!("{err:#}");
        assert!(msg.contains("cycle"), "error must say cycle: {msg}");
        assert!(msg.contains("'a'"), "error must name one clone: {msg}");
        assert!(msg.contains("'b'"), "error must name the other: {msg}");
    }

    /// Non-cyclic clone-of-clone chains resolve: `b` copies the stack
    /// default, then `a` copies the now-concrete `b`.
    #[test]
    fn clone_chain_resolves_in_order() {
        let dir = rust_workspace();
        let mut config = Config::empty();
        config.commands.insert(
            "b".to_string(),
            CommandSpec::Clone(CloneCommandSpec::new("clippy")),
        );
        config.commands.insert(
            "a".to_string(),
            CommandSpec::Clone(CloneCommandSpec::new("b")),
        );
        apply(&mut config, dir.path()).expect("chain must resolve");

        assert!(
            matches!(config.commands.get("b"), Some(CommandSpec::Exec(_))),
            "b must materialize"
        );
        assert!(
            matches!(config.commands.get("a"), Some(CommandSpec::Exec(_))),
            "a must materialize from the concrete b"
        );
    }

    /// Exec-only overrides beside a composite source are load errors — a
    /// silent no-op would hide the copy-paste mistake.
    #[test]
    fn exec_overrides_on_composite_clone_error() {
        let dir = rust_workspace();
        let mut decl = CloneCommandSpec::new("verify");
        decl.timeout_secs = Some(60);
        let mut config = config_with_clone("my-verify", decl);
        let err = apply(&mut config, dir.path()).expect_err("exec override must error");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("timeout_secs"),
            "error must name the field: {msg}"
        );
        assert!(msg.contains("verify"), "error must name the source: {msg}");
    }

    /// AC #5: the clone copies the source *before* the source's own
    /// `[extend]` applies — extends stay per-name. Here `clippy` is extended
    /// with `--locked` in the same config; the clone made by `apply` must
    /// not carry it (`extend::apply` has not run yet).
    #[test]
    fn clone_copies_the_pre_extend_source() {
        let dir = rust_workspace();
        let mut config = config_with_clone("fuzz-clippy", CloneCommandSpec::new("clippy"));
        config.extend.insert(
            "clippy".to_string(),
            crate::config::ExtendEntry {
                commands: Vec::new(),
                args: vec!["--locked".to_string()],
            },
        );
        apply(&mut config, dir.path()).expect("clone must apply");

        let Some(CommandSpec::Exec(fuzz_clippy)) = config.commands.get("fuzz-clippy") else {
            panic!("clone must materialize");
        };
        assert!(
            !fuzz_clippy.args.contains(&"--locked".to_string()),
            "the clone must copy the pre-extend source, got {:?}",
            fuzz_clippy.args
        );
    }

    /// Config-defined sources shadow the stack default, matching the
    /// resolution precedence every other consumer uses.
    #[test]
    fn config_defined_source_wins_over_stack_default() {
        let dir = rust_workspace();
        let mut config = config_with_clone("my-doc", CloneCommandSpec::new("doc"));
        config.commands.insert(
            "doc".to_string(),
            CommandSpec::Exec(ExecCommandSpec::new("echo", ["from-config"])),
        );
        apply(&mut config, dir.path()).unwrap();

        let Some(CommandSpec::Exec(doc)) = config.commands.get("my-doc") else {
            panic!("clone must materialize");
        };
        assert_eq!(doc.program, "echo", "the local doc must win as the source");
    }

    /// No clone declarations: a free no-op.
    #[test]
    fn apply_without_clones_is_a_no_op() {
        let dir = rust_workspace();
        let mut config = Config::empty();
        config.commands.insert(
            "plain".to_string(),
            CommandSpec::Composite(CompositeCommandSpec::new(["fmt"])),
        );
        apply(&mut config, dir.path()).expect("no clones must be a no-op");
        assert_eq!(config.commands.len(), 1);
    }
}
