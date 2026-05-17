//! CLI argument definitions, subcommand enums, and arg preprocessing.

use std::ffi::OsString;
use std::path::PathBuf;

pub use clap::{CommandFactory, Parser};
use ops_core::stack::Stack;

#[derive(Parser, Debug)]
#[command(
    name = "ops",
    bin_name = "ops",
    about = "Batteries-included task runner for any stack",
    version,
    next_display_order = None
)]
pub struct Cli {
    /// Preview commands without executing (dry-run mode).
    ///
    /// Prints the resolved command(s) that would be run, including all
    /// arguments and environment variables. Useful for:
    /// - Verifying config changes before running
    /// - Auditing what commands are defined
    /// - Debugging composite command expansion
    #[arg(short, long, global = true)]
    pub dry_run: bool,

    /// Show full stderr output on failure (overrides stderr_tail_lines config).
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Capture raw command output to a file.
    #[arg(long, global = true, value_name = "FILE")]
    pub tap: Option<PathBuf>,

    /// Inherit child stdio directly and suppress ops' own output (like make/just).
    ///
    /// The child process writes straight to the terminal — colors, TUIs, and
    /// interactive prompts work natively. ops emits no step line, spinner,
    /// summary, or error box; only the child's output is visible. Exit code is
    /// propagated verbatim.
    ///
    /// Composite commands run sequentially under `--raw` (parallel is ignored).
    /// Cannot be combined with `--tap`.
    #[arg(long, global = true, conflicts_with = "tap")]
    pub raw: bool,

    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,
}

/// Core subcommands shared between direct invocation and `cargo ops` wrapper.
#[derive(clap::Subcommand, Debug, Clone)]
pub enum CoreSubcommand {
    /// Create a default `.ops.toml` in the current directory.
    ///
    /// Without section flags, generates a minimal config with output settings only.
    /// Use `--themes`, `--commands`, or `--output` to include specific sections.
    /// When any section flag is given, only the requested sections are included.
    Init {
        /// Overwrite existing `.ops.toml` if present.
        #[arg(short, long)]
        force: bool,
        /// Include output settings (theme, columns, error detail).
        #[arg(long)]
        output: bool,
        /// Include built-in theme definitions (classic, compact).
        #[arg(long)]
        themes: bool,
        /// Include stack-detected commands (e.g. build, test, verify).
        #[arg(long)]
        commands: bool,
    },
    /// Manage output themes.
    Theme {
        #[command(subcommand)]
        action: ThemeAction,
    },
    /// Manage extensions.
    Extension {
        #[command(subcommand)]
        action: ExtensionAction,
    },
    /// Display project identity card.
    About {
        /// Force re-collection of data (ignores cached results).
        #[arg(long)]
        refresh: bool,
        #[command(subcommand)]
        action: Option<AboutAction>,
    },
    /// Dependency health: upgrades, advisories, licenses, bans, sources.
    #[cfg(feature = "stack-rust")]
    Deps {
        /// Force re-collection of data (ignores cached results).
        #[arg(long)]
        refresh: bool,
    },
    /// Interactively add a new command to `.ops.toml`.
    NewCommand,
    /// Setup git pre-commit hook to run an ops command of your choice.
    ///
    /// Without a subcommand, runs the configured command. Pass `--changed-only`
    /// to skip the run when nothing is staged.
    RunBeforeCommit {
        /// Skip the hook when no files are staged for commit.
        ///
        /// This flag gates a preflight that short-circuits the run when
        /// `git diff --cached` reports no staged changes. It does *not*
        /// scope the user-configured command to staged paths.
        #[arg(long)]
        changed_only: bool,
        #[command(subcommand)]
        action: Option<RunBeforeCommitAction>,
    },
    /// Setup git pre-push hook to run an ops command of your choice.
    ///
    /// Without a subcommand, runs the configured command.
    //
    // No per-variant `next_help_heading` — `help::builtin_category` already
    // categorizes this command under "Setup". No `--changed-only` because
    // no pre-push preflight exists; carrying the flag here previously
    // silently no-op'd.
    RunBeforePush {
        #[command(subcommand)]
        action: Option<RunBeforePushAction>,
    },
    /// Install and manage cargo development tools.
    Tools {
        #[command(subcommand)]
        action: ToolsAction,
    },
    /// Summarized Terraform plans as two tables (actions + resource changes).
    ///
    /// The variant embeds a single clap-derived `PlanOptions` (defined in
    /// `ops_tfplan`) so a new plan flag is added in exactly one place
    /// rather than copied across the variant, the dispatch destructure,
    /// and the `PlanOptions` repack.
    #[cfg(feature = "stack-terraform")]
    Plans(ops_tfplan::PlanOptions),
    /// Catch-all for dynamic config-defined commands (e.g. `ops verify`).
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

/// Theme management subcommands.
#[derive(clap::Subcommand, Debug, Clone)]
pub enum ThemeAction {
    /// List available themes.
    List,
    /// Interactively select a theme.
    Select,
}

/// About subcommands.
#[derive(clap::Subcommand, Debug, Clone)]
pub enum AboutAction {
    /// Interactively choose which fields to show on the about card.
    Setup,
    /// Display detailed test coverage table.
    Coverage,
    /// `about code` renders DuckDB-backed code statistics
    /// (LOC by language). Gating the variant under the `duckdb` feature
    /// keeps the CLI surface honest — without DuckDB the binary has no way
    /// to compute the stats, so the subcommand simply doesn't exist in help
    /// / parse / tab completion instead of bailing at runtime after a
    /// successful parse.
    #[cfg(feature = "duckdb")]
    /// Display code statistics (lines of code, languages).
    Code,
    /// Display dependency tree.
    Dependencies,
    /// `crates` and `modules` render the same stack-aware
    /// project-units view via `ops_about::run_about_units`; the alias keeps
    /// the Go-idiomatic name working without duplicating dispatch.
    #[command(alias = "modules")]
    /// Display project units — crates (Rust) or modules (Go).
    Crates,
}

/// Extension management subcommands.
#[derive(clap::Subcommand, Debug, Clone)]
pub enum ExtensionAction {
    /// List compiled-in extensions and their status.
    List,
    /// Show details for a specific extension (interactive picker if omitted).
    Show { name: Option<String> },
}

/// Run-before-commit hook management subcommands.
#[derive(clap::Subcommand, Debug, Clone)]
pub enum RunBeforeCommitAction {
    /// Install the git pre-commit hook and add a default command to `.ops.toml`.
    Install,
}

/// Run-before-push hook management subcommands.
#[derive(clap::Subcommand, Debug, Clone)]
pub enum RunBeforePushAction {
    /// Install the git pre-push hook and add a default command to `.ops.toml`.
    Install,
}

/// Tools management subcommands.
#[derive(clap::Subcommand, Debug, Clone)]
pub enum ToolsAction {
    /// List configured tools and their installation status.
    List,
    /// Check if all tools are installed (exit 1 if missing).
    Check,
    /// Install missing tools.
    Install {
        /// Install a specific tool. If omitted, installs all missing tools.
        name: Option<String>,
    },
}

/// Flatten `CoreSubcommand` so `ops verify` works directly.
/// The `ops` prefix from `cargo ops ...` is stripped before parsing.
pub type Subcommand = CoreSubcommand;

/// Subcommand names that are only relevant to a specific stack.
/// Unlisted commands are always visible.
fn stack_specific_commands() -> &'static [(&'static str, Stack)] {
    &[
        #[cfg(feature = "stack-rust")]
        ("deps", Stack::Rust),
        #[cfg(feature = "stack-rust")]
        ("tools", Stack::Rust),
        #[cfg(feature = "stack-terraform")]
        ("plans", Stack::Terraform),
    ]
}

/// Hide subcommands whose required stack doesn't match the detected one.
pub(crate) fn hide_irrelevant_commands(
    mut cmd: clap::Command,
    stack: Option<Stack>,
) -> clap::Command {
    for &(name, required_stack) in stack_specific_commands() {
        let dominated = match stack {
            Some(s) => s != required_stack,
            None => true,
        };
        if dominated {
            cmd = cmd.mut_subcommand(name, |sub| sub.hide(true));
        }
    }
    cmd
}

pub(crate) fn preprocess_args(args: Vec<OsString>) -> Vec<OsString> {
    if args.len() > 1 && args[1] == "ops" {
        let mut it = args.into_iter();
        let program = it.next().expect("len > 1 implies a program arg");
        let _ops_prefix = it.next();
        std::iter::once(program).chain(it).collect()
    } else {
        args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Subcommand parsing --

    #[test]
    fn parse_direct_external_subcommand() {
        let cli = Cli::parse_from(["ops", "verify"]);
        assert!(matches!(cli.subcommand, Some(CoreSubcommand::External(_))));
    }

    #[test]
    fn parse_direct_init_subcommand() {
        let cli = Cli::parse_from(["ops", "init"]);
        assert!(matches!(
            cli.subcommand,
            Some(CoreSubcommand::Init { force: false, .. })
        ));
    }

    #[test]
    fn parse_extension_list_subcommand() {
        let cli = Cli::parse_from(["ops", "extension", "list"]);
        assert!(matches!(
            cli.subcommand,
            Some(CoreSubcommand::Extension {
                action: ExtensionAction::List
            })
        ));
    }

    #[test]
    fn parse_extension_show_subcommand() {
        let cli = Cli::parse_from(["ops", "extension", "show", "metadata"]);
        match cli.subcommand {
            Some(CoreSubcommand::Extension {
                action: ExtensionAction::Show { name },
            }) => assert_eq!(name, Some("metadata".to_string())),
            other => panic!("expected Extension Show, got {:?}", other),
        }
    }

    #[test]
    fn parse_extension_show_no_arg() {
        let cli = Cli::parse_from(["ops", "extension", "show"]);
        match cli.subcommand {
            Some(CoreSubcommand::Extension {
                action: ExtensionAction::Show { name },
            }) => assert_eq!(name, None),
            other => panic!("expected Extension Show with None, got {:?}", other),
        }
    }

    #[test]
    fn parse_no_subcommand() {
        let cli = Cli::parse_from(["ops"]);
        assert!(cli.subcommand.is_none());
    }

    #[test]
    fn parse_raw_flag() {
        let cli = Cli::try_parse_from(["ops", "--raw", "build"]).unwrap();
        assert!(cli.raw);
        assert!(cli.tap.is_none());
    }

    #[test]
    fn parse_raw_and_tap_conflicts() {
        let err = Cli::try_parse_from(["ops", "--raw", "--tap", "out.log", "build"])
            .expect_err("--raw and --tap must conflict");
        let msg = err.to_string();
        assert!(
            msg.contains("cannot be used with") || msg.contains("conflict"),
            "expected conflict error, got: {msg}"
        );
    }

    // -- preprocess_args --

    #[test]
    fn preprocess_args_strips_ops_prefix() {
        let args: Vec<OsString> = vec!["ops".into(), "ops".into(), "build".into()];
        let result = preprocess_args(args);
        assert_eq!(result, vec![OsString::from("ops"), OsString::from("build")]);
    }

    #[test]
    fn preprocess_args_preserves_all_after_ops() {
        let args: Vec<OsString> =
            vec!["ops".into(), "ops".into(), "run".into(), "mycommand".into()];
        let result = preprocess_args(args);
        assert_eq!(
            result,
            vec![
                OsString::from("ops"),
                OsString::from("run"),
                OsString::from("mycommand")
            ]
        );
    }

    #[test]
    fn preprocess_args_no_change_without_ops() {
        let args: Vec<OsString> = vec!["ops".into(), "build".into()];
        let result = preprocess_args(args);
        assert_eq!(result, vec![OsString::from("ops"), OsString::from("build")]);
    }

    #[test]
    fn preprocess_args_single_arg_no_change() {
        let args: Vec<OsString> = vec!["ops".into()];
        let result = preprocess_args(args);
        assert_eq!(result, vec![OsString::from("ops")]);
    }

    // -- hide_irrelevant_commands --

    #[test]
    fn hide_irrelevant_commands_preserves_non_stack_commands() {
        // TEST-25 (TASK-1374): build the clap command tree exactly once and
        // pre-compute the set of originally-hidden subcommand names instead
        // of calling `Cli::command()` inside the loop, which would
        // re-walk the entire derive metadata on every iteration.
        let original = Cli::command();
        let originally_hidden: std::collections::HashSet<String> = original
            .get_subcommands()
            .filter(|s| s.is_hide_set())
            .map(|s| s.get_name().to_string())
            .collect();
        let result = hide_irrelevant_commands(original.clone(), None);
        // Compute the set of stack-specific subcommand names once, then assert
        // that every *other* visible, non-hidden subcommand remains visible —
        // future non-stack built-ins are covered automatically without having
        // to edit this list.
        let stack_specific: std::collections::HashSet<&'static str> =
            stack_specific_commands().iter().map(|(n, _)| *n).collect();
        for sub in result.get_subcommands() {
            let name = sub.get_name();
            if stack_specific.contains(name) {
                continue;
            }
            // Skip subcommands that were already hidden before our call (clap
            // may ship internal hidden helpers); we only guarantee we don't
            // flip a previously-visible non-stack command to hidden.
            if originally_hidden.contains(name) {
                continue;
            }
            assert!(
                !sub.is_hide_set(),
                "{name} is not stack-specific and should remain visible"
            );
        }
    }

    #[cfg(feature = "stack-rust")]
    #[test]
    fn hide_irrelevant_commands_no_stack_hides_stack_specific() {
        let cmd = Cli::command();
        let hidden_cmd = hide_irrelevant_commands(cmd, None);
        for sub in hidden_cmd.get_subcommands() {
            let name = sub.get_name();
            if name == "deps" || name == "tools" {
                assert!(
                    sub.is_hide_set(),
                    "{name} should be hidden when no stack detected"
                );
            }
        }
    }

    #[cfg(feature = "stack-rust")]
    #[test]
    fn hide_irrelevant_commands_matching_stack_shows() {
        let cmd = Cli::command();
        let visible_cmd = hide_irrelevant_commands(cmd, Some(Stack::Rust));
        for sub in visible_cmd.get_subcommands() {
            let name = sub.get_name();
            if name == "deps" || name == "tools" {
                assert!(
                    !sub.is_hide_set(),
                    "{name} should be visible for Rust stack"
                );
            }
        }
    }

    #[cfg(feature = "stack-rust")]
    #[test]
    fn hide_irrelevant_commands_wrong_stack_hides() {
        let cmd = Cli::command();
        let hidden_cmd = hide_irrelevant_commands(cmd, Some(Stack::Go));
        for sub in hidden_cmd.get_subcommands() {
            let name = sub.get_name();
            if name == "deps" || name == "tools" {
                assert!(sub.is_hide_set(), "{name} should be hidden for Go stack");
            }
        }
    }

    // -- parse subcommand edge cases --

    #[test]
    fn parse_dry_run_flag() {
        let cli = Cli::parse_from(["ops", "-d", "build"]);
        assert!(cli.dry_run);
    }

    #[test]
    fn parse_tap_flag() {
        let cli = Cli::parse_from(["ops", "--tap", "out.log", "build"]);
        assert_eq!(cli.tap, Some(PathBuf::from("out.log")));
    }

    #[test]
    fn parse_no_tap_flag() {
        let cli = Cli::parse_from(["ops", "build"]);
        assert!(cli.tap.is_none());
    }

    #[test]
    fn parse_verbose_flag() {
        let cli = Cli::parse_from(["ops", "-v", "build"]);
        assert!(cli.verbose);
    }

    #[test]
    fn parse_about_with_refresh() {
        let cli = Cli::parse_from(["ops", "about", "--refresh"]);
        match cli.subcommand {
            Some(CoreSubcommand::About { refresh, action }) => {
                assert!(refresh);
                assert!(action.is_none());
            }
            other => panic!("expected About with refresh, got {:?}", other),
        }
    }

    /// `ops about modules` must continue to parse — it is
    /// the Go-idiomatic alias for the stack-aware project-units view
    /// (`AboutAction::Crates`).
    #[test]
    fn parse_about_modules_aliases_crates() {
        let cli = Cli::parse_from(["ops", "about", "modules"]);
        match cli.subcommand {
            Some(CoreSubcommand::About { action, .. }) => {
                assert!(matches!(action, Some(AboutAction::Crates)));
            }
            other => panic!("expected About::Crates via modules alias, got {:?}", other),
        }
    }

    /// Under a build without the `duckdb` feature the
    /// `about code` subcommand must not appear in `about`'s help output —
    /// the binary cannot compute the stats so the CLI surface must reflect
    /// that. Mirrors the `Tools` gating already in place.
    #[cfg(not(feature = "duckdb"))]
    #[test]
    fn about_code_not_in_help_without_duckdb_feature() {
        let cmd = Cli::command();
        let about = cmd
            .find_subcommand("about")
            .expect("about subcommand must exist");
        let names: Vec<&str> = about
            .get_subcommands()
            .map(clap::Command::get_name)
            .collect();
        assert!(
            !names.contains(&"code"),
            "about subcommands without duckdb must not include `code`: {names:?}"
        );
    }

    #[cfg(feature = "duckdb")]
    #[test]
    fn about_code_in_help_with_duckdb_feature() {
        let cmd = Cli::command();
        let about = cmd
            .find_subcommand("about")
            .expect("about subcommand must exist");
        let names: Vec<&str> = about
            .get_subcommands()
            .map(clap::Command::get_name)
            .collect();
        assert!(
            names.contains(&"code"),
            "about subcommands with duckdb must include `code`: {names:?}"
        );
    }

    #[test]
    fn parse_about_setup() {
        let cli = Cli::parse_from(["ops", "about", "setup"]);
        match cli.subcommand {
            Some(CoreSubcommand::About { action, .. }) => {
                assert!(matches!(action, Some(AboutAction::Setup)));
            }
            other => panic!("expected About Setup, got {:?}", other),
        }
    }

    #[test]
    fn parse_theme_list() {
        let cli = Cli::parse_from(["ops", "theme", "list"]);
        assert!(matches!(
            cli.subcommand,
            Some(CoreSubcommand::Theme {
                action: ThemeAction::List
            })
        ));
    }

    #[test]
    fn parse_run_before_commit_with_changed_only() {
        let cli = Cli::parse_from(["ops", "run-before-commit", "--changed-only"]);
        match cli.subcommand {
            Some(CoreSubcommand::RunBeforeCommit {
                changed_only,
                action,
            }) => {
                assert!(changed_only);
                assert!(action.is_none());
            }
            other => panic!("expected RunBeforeCommit, got {:?}", other),
        }
    }

    /// `run-before-push` does not accept `--changed-only`
    /// because no pre-push preflight exists. The flag previously parsed and
    /// silently no-op'd; clap now rejects it so the user is not misled.
    #[test]
    fn parse_run_before_push_changed_only_is_rejected() {
        let err = Cli::try_parse_from(["ops", "run-before-push", "--changed-only"])
            .expect_err("--changed-only must not parse for run-before-push");
        let msg = err.to_string();
        assert!(
            msg.contains("--changed-only") || msg.contains("unexpected"),
            "expected clap to reject the unknown flag, got: {msg}"
        );
    }

    #[test]
    fn parse_run_before_commit_install() {
        let cli = Cli::parse_from(["ops", "run-before-commit", "install"]);
        match cli.subcommand {
            Some(CoreSubcommand::RunBeforeCommit { action, .. }) => {
                assert!(matches!(action, Some(RunBeforeCommitAction::Install)));
            }
            other => panic!("expected RunBeforeCommit Install, got {:?}", other),
        }
    }

    #[test]
    fn preprocess_args_ops_only_at_second_position() {
        let args: Vec<OsString> = vec!["ops".into(), "build".into(), "ops".into()];
        let result = preprocess_args(args);
        assert_eq!(
            result,
            vec![
                OsString::from("ops"),
                OsString::from("build"),
                OsString::from("ops")
            ]
        );
    }
}
