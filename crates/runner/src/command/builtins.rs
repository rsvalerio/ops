//! Built-in commands that always resolve, regardless of stack/config/extensions.
//!
//! These mirror clap-level subcommands (e.g. `ops end-of-file-fixer`) so that
//! the composite resolver in [`super::resolve`] sees them by the same name.
//! Without this store, `commands = ["end-of-file-fixer"]` in a composite
//! resolves to `unknown command` even though `ops end-of-file-fixer` works
//! at the top level.
//!
//! Each builtin is registered as an [`ExecCommandSpec`] that re-invokes the
//! current `ops` binary with the matching subcommand. This keeps the
//! execution path identical to every other exec leaf and avoids a parallel
//! dispatch table.
//!
//! Program resolution, display and scheduling defaults come from
//! [`ExecCommandSpec::ops_subcommand`], shared with the extension
//! registrations of the same command ids so the two cannot diverge
//! (SEC-13 / TASK-2122).

use indexmap::IndexMap;
use ops_core::config::{CommandId, CommandSpec, ExecCommandSpec};

/// Build the always-available builtin command store.
///
/// Currently registers the text fixers (`end-of-file-fixer` / `eof`,
/// `trailing-whitespace` / `tw`), the config checkers (`check-json`,
/// `check-yaml`) and `sec`. Add new entries here whenever a clap-level
/// subcommand should also be referenceable from composite `commands = [...]`.
///
/// The fixers rewrite files, so they keep `ops_subcommand`'s exclusive
/// default; the checkers and `sec` only read and are marked [`read_only`].
pub(super) fn builtin_commands() -> IndexMap<CommandId, CommandSpec> {
    let mut map = IndexMap::new();
    map.insert(
        CommandId::from("end-of-file-fixer"),
        CommandSpec::Exec(builtin_exec("end-of-file-fixer", &["eof"])),
    );
    map.insert(
        CommandId::from("trailing-whitespace"),
        CommandSpec::Exec(builtin_exec("trailing-whitespace", &["tw"])),
    );
    map.insert(
        CommandId::from("check-json"),
        CommandSpec::Exec(read_only(builtin_exec("check-json", &[]))),
    );
    map.insert(
        CommandId::from("check-yaml"),
        CommandSpec::Exec(read_only(builtin_exec("check-yaml", &[]))),
    );
    map.insert(
        CommandId::from("sec"),
        CommandSpec::Exec(read_only(builtin_exec("sec", &[]))),
    );
    map
}

fn builtin_exec(subcommand: &'static str, aliases: &[&'static str]) -> ExecCommandSpec {
    let mut spec = ExecCommandSpec::ops_subcommand(subcommand);
    spec.aliases = aliases.iter().map(|a| (*a).to_string()).collect();
    spec.category = Some("Code Quality".to_string());
    spec
}

/// Let a builtin that never writes to the worktree overlap other steps.
const fn read_only(mut spec: ExecCommandSpec) -> ExecCommandSpec {
    spec.exclusive = false;
    spec
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_text_fixers_with_aliases() {
        let map = builtin_commands();
        let eof = map
            .get("end-of-file-fixer")
            .expect("end-of-file-fixer registered");
        assert!(eof.aliases().iter().any(|a| a == "eof"));
        let tw = map
            .get("trailing-whitespace")
            .expect("trailing-whitespace registered");
        assert!(tw.aliases().iter().any(|a| a == "tw"));
    }

    #[test]
    fn registers_config_checkers() {
        let map = builtin_commands();
        assert!(
            map.get("check-json").is_some(),
            "check-json must be registered as a builtin"
        );
        assert!(
            map.get("check-yaml").is_some(),
            "check-yaml must be registered as a builtin"
        );
    }

    #[test]
    fn registers_sec_scanner() {
        let map = builtin_commands();
        assert!(
            map.get("sec").is_some(),
            "sec must be registered as a builtin so composites can reference it"
        );
    }

    #[test]
    fn builtin_exec_invokes_current_binary_with_subcommand() {
        let map = builtin_commands();
        let CommandSpec::Exec(exec) = map.get("end-of-file-fixer").unwrap() else {
            panic!("expected exec spec");
        };
        assert_eq!(exec.args, vec!["end-of-file-fixer".to_string()]);
        assert!(!exec.program.is_empty());
    }

    /// Builtins spawn via `current_exe()` (often an absolute path) but must
    /// render like the extension-registered commands: `ops sec`, not
    /// `/home/…/bin/ops sec`.
    #[test]
    fn builtin_exec_displays_as_ops_not_absolute_path() {
        let map = builtin_commands();
        let CommandSpec::Exec(exec) = map.get("sec").unwrap() else {
            panic!("expected exec spec");
        };
        assert_eq!(exec.display_cmd(), "ops sec");
    }

    /// The fixers rewrite files and must run alone in a parallel plan; the
    /// read-only checkers and `sec` may overlap other steps.
    #[test]
    fn only_file_rewriting_builtins_are_exclusive() {
        let map = builtin_commands();
        for (name, expected) in [
            ("end-of-file-fixer", true),
            ("trailing-whitespace", true),
            ("check-json", false),
            ("check-yaml", false),
            ("sec", false),
        ] {
            let Some(CommandSpec::Exec(exec)) = map.get(name) else {
                panic!("{name} must be an exec builtin");
            };
            assert_eq!(exec.exclusive, expected, "{name}.exclusive");
        }
    }
}
