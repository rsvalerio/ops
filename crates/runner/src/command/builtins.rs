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
/// `trailing-whitespace` / `tw`) and their `--check` twins
/// (`end-of-file-fixer-check`, `trailing-whitespace-check`, which every
/// stack's `verify` names), the config checkers (`check-json`,
/// `check-yaml`), `sec`, `lint-actions` and `msrv`. Add new entries here whenever a clap-level
/// subcommand should also be referenceable from composite `commands = [...]`.
///
/// The fixers rewrite files, so they keep `ops_subcommand`'s exclusive
/// default; their twins and the checkers (`lint-actions` included) only read and are marked
/// [`read_only`]. `msrv` compiles into `target/`, so it keeps the exclusive
/// default like any build step. `sec` also
/// never writes the worktree, but it stays exclusive (TASK-2263): Trivy
/// reads the *whole* tree, build outputs included, and aborts the scan when
/// a file vanishes mid-walk — which is exactly what a concurrent build or
/// test step does to `target/`. Overlapping `sec` with those steps is a
/// race, not a safe read-only overlap.
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
    for fixer in ["end-of-file-fixer", "trailing-whitespace"] {
        let mut check = ExecCommandSpec::ops_subcommand_check(fixer);
        check.category = Some("Code Quality".to_string());
        map.insert(
            CommandId::from(format!("{fixer}-check")),
            CommandSpec::Exec(check),
        );
    }
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
        CommandSpec::Exec(builtin_exec("sec", &[])),
    );
    map.insert(
        CommandId::from("lint-actions"),
        CommandSpec::Exec(read_only(builtin_exec("lint-actions", &[]))),
    );
    map.insert(
        CommandId::from("msrv"),
        CommandSpec::Exec(builtin_exec("msrv", &[])),
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
///
/// `sec` is deliberately excluded (TASK-2263): although it never *writes*,
/// its Trivy scan reads the entire tree — build outputs included — and
/// aborts with `fs scan error … no such file or directory` when a
/// concurrent build or test step deletes a file mid-walk. A scan that races
/// the steps around it is not a safe overlap, so `sec` keeps the exclusive
/// default and runs alone, in list order, in any parallel plan.
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

    /// Every stack's `verify` names these twins, so they must resolve even
    /// where the text-fixers extension is not registered.
    #[test]
    fn registers_text_fixer_check_twins_as_read_only() {
        let map = builtin_commands();
        for fixer in ["end-of-file-fixer", "trailing-whitespace"] {
            let Some(CommandSpec::Exec(check)) = map.get(format!("{fixer}-check").as_str()) else {
                panic!("{fixer}-check must be registered as an exec builtin");
            };
            assert_eq!(check.args, [fixer, "--check"]);
            assert!(!check.exclusive, "{fixer}-check never writes");
        }
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
    /// read-only checkers may overlap other steps. `sec` is exclusive too —
    /// see [`read_only`] for why a whole-tree Trivy scan is not a safe
    /// overlap even though it never writes.
    #[test]
    fn only_file_rewriting_builtins_are_exclusive() {
        let map = builtin_commands();
        for (name, expected) in [
            ("end-of-file-fixer", true),
            ("trailing-whitespace", true),
            ("check-json", false),
            ("check-yaml", false),
            ("sec", true),
            ("lint-actions", false),
            ("msrv", true),
        ] {
            let Some(CommandSpec::Exec(exec)) = map.get(name) else {
                panic!("{name} must be an exec builtin");
            };
            assert_eq!(exec.exclusive, expected, "{name}.exclusive");
        }
    }

    /// TASK-2263 AC #3: pin `sec` as exclusive on its own, so a future
    /// refactor that reintroduces the `read_only` wrapper for it fails here
    /// rather than resurrecting the Trivy-vs-build race in a parallel plan.
    #[test]
    fn sec_is_registered_exclusive() {
        let map = builtin_commands();
        let Some(CommandSpec::Exec(exec)) = map.get("sec") else {
            panic!("sec must be an exec builtin");
        };
        assert!(
            exec.exclusive,
            "sec must be exclusive: Trivy walks the whole tree, build outputs included, \
             and aborts when a concurrent step deletes a file mid-walk"
        );
    }
}
