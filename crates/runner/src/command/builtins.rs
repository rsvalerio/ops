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
//! Path resolution: prefer [`std::env::current_exe`] (absolute, robust under
//! renamed/aliased shells) and fall back to `"ops"` when the current-exe
//! lookup fails (e.g. unusual sandboxing). `PATH` will then resolve it.

use indexmap::IndexMap;
use ops_core::config::{CommandId, CommandSpec, ExecCommandSpec};

/// Build the always-available builtin command store.
///
/// Currently registers the text fixers (`end-of-file-fixer` / `eof`,
/// `trailing-whitespace` / `tw`) and the config checkers (`check-json`,
/// `check-yaml`). Add new entries here whenever a clap-level subcommand
/// should also be referenceable from composite `commands = [...]`.
pub(super) fn builtin_commands() -> IndexMap<CommandId, CommandSpec> {
    let ops_bin = std::env::current_exe()
        .ok()
        .and_then(|p| p.into_os_string().into_string().ok())
        .unwrap_or_else(|| "ops".to_string());

    let mut map = IndexMap::new();
    map.insert(
        CommandId::from("end-of-file-fixer"),
        CommandSpec::Exec(builtin_exec(&ops_bin, "end-of-file-fixer", &["eof"])),
    );
    map.insert(
        CommandId::from("trailing-whitespace"),
        CommandSpec::Exec(builtin_exec(&ops_bin, "trailing-whitespace", &["tw"])),
    );
    map.insert(
        CommandId::from("check-json"),
        CommandSpec::Exec(builtin_exec(&ops_bin, "check-json", &[])),
    );
    map.insert(
        CommandId::from("check-yaml"),
        CommandSpec::Exec(builtin_exec(&ops_bin, "check-yaml", &[])),
    );
    map.insert(
        CommandId::from("sec"),
        CommandSpec::Exec(builtin_exec(&ops_bin, "sec", &[])),
    );
    map
}

fn builtin_exec(
    ops_bin: &str,
    subcommand: &'static str,
    aliases: &[&'static str],
) -> ExecCommandSpec {
    let mut spec = ExecCommandSpec::new(ops_bin.to_string(), [subcommand.to_string()]);
    spec.aliases = aliases.iter().map(|a| (*a).to_string()).collect();
    spec.category = Some("Code Quality".to_string());
    // Spawn via `ops_bin` (current_exe, robust) but render as `ops <subcommand>`
    // — the same display the extension-registered commands get from their
    // literal `program = "ops"`. Without this, step lines show the full
    // absolute path (`/home/…/bin/ops sec`).
    spec.display_program = Some("ops".to_string());
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
}
