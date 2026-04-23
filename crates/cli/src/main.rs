//! CLI entry point and orchestration for ops.

// Force the linker to retain extension crates that only register via linkme
// distributed slices (no other symbols are referenced from the main binary).
#[cfg(feature = "stack-go")]
extern crate ops_about_go;
#[cfg(any(feature = "stack-java-maven", feature = "stack-java-gradle"))]
extern crate ops_about_java;
#[cfg(feature = "stack-node")]
extern crate ops_about_node;
#[cfg(feature = "stack-python")]
extern crate ops_about_python;
#[cfg(feature = "stack-rust")]
extern crate ops_about_rust;
#[cfg(feature = "stack-rust")]
extern crate ops_cargo_toml;
#[cfg(feature = "stack-rust")]
extern crate ops_cargo_update;
extern crate ops_git;
#[cfg(feature = "stack-rust")]
extern crate ops_metadata;
#[cfg(feature = "coverage")]
extern crate ops_test_coverage;
#[cfg(feature = "tokei")]
extern crate ops_tokei;

mod about_cmd;
mod args;
mod extension_cmd;
mod help;
mod hook_shared;
mod init_cmd;
mod new_command_cmd;
mod registry;
mod run_before_commit_cmd;
mod run_before_push_cmd;
mod run_cmd;
mod subcommands;
mod theme_cmd;
#[cfg(feature = "stack-rust")]
mod tools_cmd;
mod tty;

#[cfg(test)]
mod test_utils;
#[cfg(test)]
pub(crate) use test_utils::CwdGuard;

use clap::FromArgMatches;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use args::*;
use help::{is_toplevel_help, print_categorized_help};
use subcommands::{run_about, run_before_commit, run_before_push, run_extension, run_theme};
#[cfg(feature = "stack-rust")]
use subcommands::{run_deps, run_tools};

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            ops_core::ui::error(format!("{e:#}"));
            ExitCode::FAILURE
        }
    }
}

fn init_logging() {
    let log_level = std::env::var("OPS_LOG_LEVEL")
        .map(|v| {
            v.parse().unwrap_or_else(|e| {
                tracing::debug!(
                    value = %v,
                    error = %e,
                    "EFF-002: invalid OPS_LOG_LEVEL, falling back to info"
                );
                tracing_subscriber::filter::LevelFilter::INFO.into()
            })
        })
        .unwrap_or_else(|_| {
            tracing::trace!("EFF-002: OPS_LOG_LEVEL not set, using default info");
            tracing_subscriber::filter::LevelFilter::INFO.into()
        });
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(io::stderr))
        .with(
            EnvFilter::from_default_env()
                .add_directive(log_level)
                .add_directive("tokei=error".parse().expect("static directive is valid")),
        )
        .init();
}

fn run() -> anyhow::Result<ExitCode> {
    init_logging();

    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let effective_args = preprocess_args(args);

    // Load config early so stack detection and help output can use it.
    //
    // ERR-4: a malformed `.ops.toml` previously degraded silently to
    // `Config::default()`, hiding the real cause behind downstream
    // "unknown command" errors. Print the actionable error to stderr so the
    // user sees the file path and parse diagnostic, then keep going with
    // defaults so `ops --help` and `ops init` still work.
    let early_config = match ops_core::config::load_config() {
        Ok(c) => c,
        Err(e) => {
            ops_core::ui::warn(format!(
                "failed to load config: {e:#}\n  \
                 using built-in defaults; fix the config file above to restore your commands"
            ));
            ops_core::config::Config::default()
        }
    };
    let detected_stack = {
        let cwd = match std::env::current_dir() {
            Ok(d) => d,
            Err(e) => {
                ops_core::ui::error(format!("could not determine working directory: {e}"));
                return Ok(ExitCode::FAILURE);
            }
        };
        ops_core::stack::Stack::resolve(early_config.stack.as_deref(), &cwd)
    };

    // If the user asked for top-level help (`ops -h` / `ops --help`), show
    // help with dynamic commands included and exit.  We intercept before clap
    // parsing because dynamic subcommands cannot be registered at parse time
    // (they would shadow the `External` catch-all).
    if is_toplevel_help(&effective_args) {
        let cmd = hide_irrelevant_commands(Cli::command(), detected_stack);
        let long = effective_args.iter().any(|a| a == "--help");
        print_categorized_help(cmd, &early_config, detected_stack, long);
        return Ok(ExitCode::SUCCESS);
    }

    let cmd = hide_irrelevant_commands(Cli::command(), detected_stack);
    let mut matches = cmd.get_matches_from(effective_args);
    let cli = Cli::from_arg_matches_mut(&mut matches).unwrap_or_else(|e: clap::Error| e.exit());

    dispatch(cli, &early_config, detected_stack)
}

fn dispatch(
    cli: Cli,
    early_config: &ops_core::config::Config,
    detected_stack: Option<ops_core::stack::Stack>,
) -> anyhow::Result<ExitCode> {
    match cli.subcommand {
        Some(CoreSubcommand::Init {
            force,
            output,
            themes,
            commands,
        }) => {
            let sections = ops_core::config::InitSections::from_flags(output, themes, commands);
            init_cmd::run_init(force, sections)?;
        }
        Some(CoreSubcommand::Theme { action }) => run_theme(action)?,
        Some(CoreSubcommand::Extension { action }) => run_extension(action)?,
        Some(CoreSubcommand::NewCommand) => new_command_cmd::run_new_command()?,
        Some(CoreSubcommand::RunBeforeCommit {
            changed_only,
            action,
        }) => return run_before_commit(action, changed_only),
        Some(CoreSubcommand::RunBeforePush {
            changed_only,
            action,
        }) => return run_before_push(action, changed_only),
        Some(CoreSubcommand::About { refresh, action }) => run_about(refresh, action)?,
        #[cfg(feature = "stack-rust")]
        Some(CoreSubcommand::Deps { refresh }) => run_deps(refresh)?,
        Some(CoreSubcommand::Tools { action }) => {
            #[cfg(feature = "stack-rust")]
            {
                return run_tools(action);
            }
            #[cfg(not(feature = "stack-rust"))]
            {
                let _ = action;
                anyhow::bail!("tools subcommand requires the stack-rust feature");
            }
        }
        Some(CoreSubcommand::External(args)) => {
            return run_cmd::run_external_command(&args, cli.dry_run, cli.verbose, cli.tap, cli.raw)
        }
        None => {
            let cmd = hide_irrelevant_commands(Cli::command(), detected_stack);
            print_categorized_help(cmd, early_config, detected_stack, false);
        }
    }

    Ok(ExitCode::SUCCESS)
}

pub(crate) fn load_config_and_cwd() -> anyhow::Result<(ops_core::config::Config, PathBuf)> {
    let config = ops_core::config::load_config()?;
    let cwd = std::env::current_dir()?;
    Ok((config, cwd))
}
