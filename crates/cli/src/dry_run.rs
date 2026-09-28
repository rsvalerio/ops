//! TASK-2301: how each builtin subcommand treats the global `--dry-run`.
//!
//! `--dry-run` promises "preview without executing". A builtin either
//! honours it (previews, or has no side effect to preview) or is refused
//! with an explicit error before anything runs — it is never silently
//! ignored.

#[cfg(feature = "stack-rust")]
use crate::args::InitArgs;
use crate::args::{AboutAction, CoreSubcommand, LockAction, LockArgs, ThemeAction};

/// The builtin `sub` as the user typed it, when it cannot preview and must
/// refuse `--dry-run`: it writes files or config, or runs external tools
/// or commands, and has no preview mode. `None` means the builtin honours
/// the flag — it previews natively (`sec`, `clippy-findings`, the hooks,
/// config commands, `backlog`, which refuses its own unpreviewable actions)
/// or neither writes nor executes anything (`check-json`, `explain`, the
/// list/show/status views).
///
/// Every variant is matched explicitly, so a new builtin fails to compile
/// here until it is classified.
pub const fn unpreviewable_builtin(sub: &CoreSubcommand) -> Option<&'static str> {
    match sub {
        // `--rust --check` only reads and compares.
        #[cfg(feature = "stack-rust")]
        CoreSubcommand::Init(InitArgs { check: true, .. }) => None,
        CoreSubcommand::Init(_) => Some("init"),
        CoreSubcommand::NewCommand => Some("new-command"),
        CoreSubcommand::ImportMakefile { .. } => Some("import-makefile"),
        CoreSubcommand::TrailingWhitespace { .. } => Some("trailing-whitespace"),
        CoreSubcommand::EndOfFileFixer { .. } => Some("end-of-file-fixer"),
        CoreSubcommand::Theme { action } => match action {
            ThemeAction::Select => Some("theme select"),
            ThemeAction::List => None,
        },
        CoreSubcommand::Lock(LockArgs { action, .. }) => match action {
            None => Some("lock"),
            Some(LockAction::Break { .. }) => Some("lock break"),
            Some(LockAction::Status { .. }) => None,
        },
        // The about collectors run external tools (cargo, git, llvm-cov)
        // and write the data cache; `setup` writes `.ops.toml`. Only the
        // backlog overview is a pure read.
        CoreSubcommand::About { action, .. } => match action {
            Some(AboutAction::Backlog) => None,
            _ => Some("about"),
        },
        #[cfg(feature = "stack-rust")]
        CoreSubcommand::Deps { .. } => Some("deps"),
        // Without `--file`/stdin the plan JSON comes from running terraform.
        #[cfg(feature = "stack-terraform")]
        CoreSubcommand::Plans(_) => Some("plans"),
        CoreSubcommand::Extension { .. }
        | CoreSubcommand::CheckJson { .. }
        | CoreSubcommand::CheckYaml { .. }
        | CoreSubcommand::Explain { .. }
        | CoreSubcommand::Backlog { .. }
        | CoreSubcommand::RunBeforeCommit { .. }
        | CoreSubcommand::RunBeforePush { .. }
        | CoreSubcommand::Sec { .. }
        | CoreSubcommand::ClippyFindings(_)
        | CoreSubcommand::Msrv { .. }
        | CoreSubcommand::LintActions { .. }
        | CoreSubcommand::External(_) => None,
    }
}

/// Refuse `--dry-run` for a builtin that cannot preview, before it runs.
///
/// # Errors
///
/// `dry_run` is set and `sub` is an [`unpreviewable_builtin`].
pub fn refuse_unpreviewable(sub: &CoreSubcommand, dry_run: bool) -> anyhow::Result<()> {
    match unpreviewable_builtin(sub) {
        Some(name) if dry_run => anyhow::bail!(
            "--dry-run is not supported for `ops {name}`: it has no preview mode; \
             nothing was run — rerun without --dry-run"
        ),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::{Cli, CommandFactory, Parser};
    use std::collections::BTreeSet;

    /// One sample invocation per builtin (nested actions where they differ)
    /// with whether `--dry-run` must be refused.
    fn samples() -> Vec<(&'static [&'static str], bool)> {
        vec![
            (&["init"], true),
            #[cfg(feature = "stack-rust")]
            (&["init", "--rust"], true),
            #[cfg(feature = "stack-rust")]
            (&["init", "--rust", "--check"], false),
            (&["new-command"], true),
            (&["import-makefile"], true),
            (&["trailing-whitespace"], true),
            (&["tw"], true),
            (&["end-of-file-fixer"], true),
            (&["eof"], true),
            (&["theme", "select"], true),
            (&["theme", "list"], false),
            (&["lock", "x", "--", "true"], true),
            (&["lock", "break", "x"], true),
            (&["lock", "status"], false),
            (&["about"], true),
            (&["about", "setup"], true),
            (&["about", "coverage"], true),
            (&["about", "backlog"], false),
            #[cfg(feature = "stack-rust")]
            (&["deps"], true),
            #[cfg(feature = "stack-terraform")]
            (&["plans"], true),
            (&["extension", "list"], false),
            (&["check-json"], false),
            (&["check-yaml"], false),
            (&["explain", "build"], false),
            (&["backlog", "task", "list"], false),
            (&["run-before-commit"], false),
            (&["run-before-push"], false),
            (&["sec"], false),
            (&["clippy-findings"], false),
            (&["msrv"], false),
            (&["lint-actions"], false),
            (&["build"], false),
        ]
    }

    /// AC #2: every builtin clap registers must have a sample here, so a
    /// new builtin fails this test until its `--dry-run` behaviour is
    /// classified (the exhaustive match above catches it at compile time).
    #[test]
    fn every_builtin_is_classified() {
        let sampled: BTreeSet<&str> = samples().iter().map(|(argv, _)| argv[0]).collect();
        let missing: Vec<String> = Cli::command()
            .get_subcommands()
            .map(|c| c.get_name().to_owned())
            .filter(|name| !sampled.contains(name.as_str()))
            .collect();
        assert!(
            missing.is_empty(),
            "classify --dry-run for builtins {missing:?} in dry_run.rs"
        );
    }

    #[test]
    fn dry_run_is_refused_exactly_for_unpreviewable_builtins() {
        for (argv, refused) in samples() {
            let full: Vec<&str> = ["ops", "--dry-run"].iter().chain(argv).copied().collect();
            let cli = Cli::try_parse_from(&full).unwrap_or_else(|e| panic!("{full:?}: {e}"));
            assert!(cli.dry_run, "{full:?} must set the global flag");
            let sub = cli.subcommand.expect("a subcommand");
            let res = refuse_unpreviewable(&sub, cli.dry_run);
            assert_eq!(res.is_err(), refused, "{argv:?}: {res:?}");
            if let Err(e) = res {
                let msg = e.to_string();
                assert!(
                    msg.contains("--dry-run is not supported for `ops "),
                    "{msg}"
                );
            }
            refuse_unpreviewable(&sub, false).expect("never refused without --dry-run");
        }
    }
}
