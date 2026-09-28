//! `[cargo] locked = true`: run every lockfile-resolving cargo command with
//! `--locked` (TASK-2323).
//!
//! CI must build against the committed `Cargo.lock`, never one cargo resolved
//! on the fly. Without a switch, every repo that wants that re-declares each
//! stack default (or writes one `[extend.<name>] args = ["--locked"]` per
//! command) and has to keep the list current as the defaults grow. This is
//! the one switch instead:
//!
//! ```toml
//! [cargo]
//! locked = true
//! ```
//!
//! or `OPS__CARGO__LOCKED=true` in a CI job, so local runs keep resolving
//! freely. It is opt-in rather than the default because `--locked` refuses
//! to *create* a lockfile, so it would break every workspace that does not
//! commit one.
//!
//! [`apply`] runs after `[extend]` and adds `--locked` (before any `--`
//! separator, like an `[extend]` arg) to every exec command whose program is
//! `cargo` and whose cargo subcommand resolves the lockfile — config
//! commands in place, stack defaults materialized into `Config::commands`
//! the way `[extend]` materializes them. Commands that already pass
//! `--locked` or `--frozen` are left alone.

use std::path::Path;

use super::{CommandSpec, Config, ExecCommandSpec};

/// Cargo subcommands that resolve the lockfile and accept `--locked`.
/// `fmt`, `clean` and third-party tools that do not (or whose flag means
/// something else) are deliberately absent.
const LOCKFILE_SUBCOMMANDS: &[&str] = &[
    "build", "check", "clippy", "doc", "test", "nextest", "run", "bench",
];

/// Whether `spec` is a cargo invocation this switch applies to and does not
/// already carry `--locked` / `--frozen` among its cargo flags.
fn needs_locked(spec: &ExecCommandSpec) -> bool {
    if spec.program != "cargo" {
        return false;
    }
    // Skip a leading `+toolchain` (`cargo +nightly build`), as rustup does.
    let mut args = spec.args.iter();
    let Some(mut subcommand) = args.next() else {
        return false;
    };
    if subcommand.starts_with('+') {
        let Some(next) = args.next() else {
            return false;
        };
        subcommand = next;
    }
    if !LOCKFILE_SUBCOMMANDS.contains(&subcommand.as_str()) {
        return false;
    }
    !spec
        .args
        .iter()
        .take_while(|a| *a != "--")
        .any(|a| a == "--locked" || a == "--frozen")
}

fn lock(spec: &mut ExecCommandSpec) {
    super::extend::append_exec_args(&mut spec.args, &["--locked".to_string()]);
}

/// Apply `[cargo] locked` to `config`; a no-op unless it is `true`.
pub(super) fn apply(config: &mut Config, workspace_root: &Path) {
    if config.cargo.locked != Some(true) {
        return;
    }
    for spec in config.commands.values_mut() {
        if let CommandSpec::Exec(exec) = spec {
            if needs_locked(exec) {
                lock(exec);
            }
        }
    }
    let Some(stack) = crate::stack::Stack::resolve(config.stack.as_deref(), workspace_root) else {
        return;
    };
    for (name, spec) in stack.default_commands_ref() {
        if config.commands.contains_key(name) {
            continue;
        }
        let CommandSpec::Exec(exec) = spec else {
            continue;
        };
        if !needs_locked(exec) {
            continue;
        }
        let mut exec = exec.clone();
        lock(&mut exec);
        config
            .commands
            .insert(name.clone(), CommandSpec::Exec(exec));
        config.provenance.extended_stack_defaults.push(name.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rust_workspace() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        dir
    }

    fn exec_args<'a>(config: &'a Config, name: &str) -> &'a [String] {
        match config.commands.get(name) {
            Some(CommandSpec::Exec(e)) => &e.args,
            other => panic!("{name} must be a materialized exec command, got {other:?}"),
        }
    }

    #[test]
    fn off_by_default_leaves_commands_untouched() {
        let dir = rust_workspace();
        let mut config = Config::empty();
        apply(&mut config, dir.path());
        assert!(config.commands.is_empty());
        assert!(config.provenance.extended_stack_defaults.is_empty());
    }

    #[test]
    fn locks_every_lockfile_resolving_stack_default() {
        let dir = rust_workspace();
        let mut config = Config::empty();
        config.cargo.locked = Some(true);
        apply(&mut config, dir.path());

        for name in [
            "build",
            "check",
            "clippy",
            "doc",
            "test",
            "test-doc",
            "test-ignored",
            "next",
            "next-ignored",
        ] {
            let args = exec_args(&config, name);
            assert!(
                args.iter().any(|a| a == "--locked"),
                "{name} must be locked: {args:?}"
            );
            assert!(
                config
                    .provenance
                    .extended_stack_defaults
                    .iter()
                    .any(|n| n == name),
                "{name} must be recorded as a stack default"
            );
        }
        for untouched in ["fmt", "fmt-check", "clean"] {
            assert!(
                !config.commands.contains_key(untouched),
                "{untouched} does not resolve the lockfile"
            );
        }
    }

    /// `--locked` must stay a cargo flag: after `--` it would be a lint or
    /// test-binary argument.
    #[test]
    fn inserts_before_the_separator_and_never_twice() {
        let dir = rust_workspace();
        let mut config = Config::empty();
        config.cargo.locked = Some(true);
        config.commands.insert(
            "already".into(),
            CommandSpec::Exec(ExecCommandSpec::new("cargo", ["build", "--frozen"])),
        );
        config.commands.insert(
            "other".into(),
            CommandSpec::Exec(ExecCommandSpec::new("make", ["build"])),
        );
        apply(&mut config, dir.path());

        let clippy = exec_args(&config, "clippy");
        let sep = clippy.iter().position(|a| a == "--").unwrap();
        let locked = clippy.iter().position(|a| a == "--locked").unwrap();
        assert!(locked < sep, "{clippy:?}");
        assert_eq!(exec_args(&config, "already"), ["build", "--frozen"]);
        assert_eq!(exec_args(&config, "other"), ["build"]);

        let before = config.commands.clone();
        apply(&mut config, dir.path());
        for (name, spec) in &config.commands {
            if let (CommandSpec::Exec(a), Some(CommandSpec::Exec(b))) = (spec, before.get(name)) {
                assert_eq!(a.args, b.args, "{name} must not be locked twice");
            }
        }
    }

    /// A config command shadowing a default is locked in place; the default
    /// is not re-inserted over it.
    #[test]
    fn config_command_is_locked_in_place() {
        let dir = rust_workspace();
        let mut config = Config::empty();
        config.cargo.locked = Some(true);
        config.commands.insert(
            "build".into(),
            CommandSpec::Exec(ExecCommandSpec::new("cargo", ["build", "-p", "mine"])),
        );
        apply(&mut config, dir.path());
        assert_eq!(
            exec_args(&config, "build"),
            ["build", "-p", "mine", "--locked"]
        );
        assert!(!config
            .provenance
            .extended_stack_defaults
            .iter()
            .any(|n| n == "build"));
    }

    /// A `+toolchain` override does not hide the subcommand from the switch.
    #[test]
    fn toolchain_override_is_locked() {
        let dir = rust_workspace();
        let mut config = Config::empty();
        config.cargo.locked = Some(true);
        config.commands.insert(
            "nightly".into(),
            CommandSpec::Exec(ExecCommandSpec::new("cargo", ["+nightly", "build"])),
        );
        config.commands.insert(
            "bare".into(),
            CommandSpec::Exec(ExecCommandSpec::new("cargo", ["+nightly"])),
        );
        apply(&mut config, dir.path());
        assert_eq!(
            exec_args(&config, "nightly"),
            ["+nightly", "build", "--locked"]
        );
        assert_eq!(exec_args(&config, "bare"), ["+nightly"]);
    }
}
