//! CLI argument definitions, subcommand enums, and arg preprocessing.

use std::ffi::OsString;

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
    /// Display workspace/project identity card.
    #[cfg(feature = "stack-rust")]
    About {
        /// Force re-collection of data (ignores cached results).
        #[arg(long)]
        refresh: bool,
    },
    /// Display comprehensive project health dashboard.
    #[cfg(feature = "stack-rust")]
    Dashboard {
        /// Skip test coverage collection.
        #[arg(long)]
        skip_coverage: bool,
        /// Force re-collection of data (ignores cached results).
        #[arg(long)]
        refresh: bool,
    },
    /// Interactively add a new command to `.ops.toml`.
    NewCommand,
    /// Install and manage git pre-commit hooks.
    ///
    /// Without a subcommand, runs the configured `pre-commit` command from `.ops.toml`.
    PreCommit {
        #[command(subcommand)]
        action: Option<PreCommitAction>,
    },
    /// Install and manage cargo development tools.
    Tools {
        #[command(subcommand)]
        action: ToolsAction,
    },
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

/// Extension management subcommands.
#[derive(clap::Subcommand, Debug, Clone)]
pub enum ExtensionAction {
    /// List compiled-in extensions and their status.
    List,
    /// Show details for a specific extension (interactive picker if omitted).
    Show { name: Option<String> },
}

/// Pre-commit hook management subcommands.
#[derive(clap::Subcommand, Debug, Clone)]
pub enum PreCommitAction {
    /// Install the git pre-commit hook and add a default command to `.ops.toml`.
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
        ("about", Stack::Rust),
        #[cfg(feature = "stack-rust")]
        ("dashboard", Stack::Rust),
        #[cfg(feature = "stack-rust")]
        ("tools", Stack::Rust),
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
        std::iter::once(args[0].clone())
            .chain(args.into_iter().skip(2))
            .collect()
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
