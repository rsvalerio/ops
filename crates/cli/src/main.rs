//! CLI entry point and orchestration for ops.

mod args;
mod extension_cmd;
mod init_cmd;
mod new_command_cmd;
mod registry;
mod run_cmd;
mod theme_cmd;
#[cfg(feature = "stack-rust")]
mod tools_cmd;

#[cfg(test)]
mod test_utils;
#[cfg(test)]
pub(crate) use test_utils::CwdGuard;

use clap::FromArgMatches;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use args::*;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "Error: {:#}", e);
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

    // Detect stack before parsing so help output hides irrelevant commands.
    let detected_stack = {
        let config = ops_core::config::load_config().unwrap_or_default();
        let cwd = std::env::current_dir().unwrap_or_default();
        ops_core::stack::Stack::resolve(config.stack.as_deref(), &cwd)
    };

    let cmd = hide_irrelevant_commands(Cli::command(), detected_stack);
    let mut matches = cmd.get_matches_from(effective_args);
    let cli = Cli::from_arg_matches_mut(&mut matches)
        .map_err(|e: clap::Error| e.exit())
        .unwrap();

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
        #[cfg(feature = "stack-rust")]
        Some(CoreSubcommand::About { refresh }) => {
            let (config, cwd) = load_config_and_cwd()?;
            let registry = crate::registry::build_data_registry(&config, &cwd)?;
            let opts = ops_about::AboutOptions { refresh };
            ops_about::run_about(&registry, &opts)?;
        }
        #[cfg(feature = "stack-rust")]
        Some(CoreSubcommand::Dashboard {
            skip_coverage,
            skip_updates,
            refresh,
        }) => {
            let (config, cwd) = load_config_and_cwd()?;
            let registry = crate::registry::build_data_registry(&config, &cwd)?;
            let tools = ops_tools::collect_tools(&config.tools);
            let opts = ops_about::DashboardOptions {
                skip_coverage,
                skip_updates,
                refresh,
            };
            ops_about::run_dashboard(&registry, &opts, &tools)?;
        }
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
            return run_cmd::run_external_command(&args, cli.dry_run)
        }
        None => {
            hide_irrelevant_commands(Cli::command(), detected_stack).print_help()?;
        }
    }

    Ok(ExitCode::SUCCESS)
}

pub(crate) fn load_config_and_cwd() -> anyhow::Result<(ops_core::config::Config, PathBuf)> {
    let config = ops_core::config::load_config()?;
    let cwd = std::env::current_dir()?;
    Ok((config, cwd))
}

fn run_theme(action: ThemeAction) -> anyhow::Result<()> {
    match action {
        ThemeAction::List => theme_cmd::run_theme_list(),
        ThemeAction::Select => theme_cmd::run_theme_select(),
    }
}

fn run_extension(action: ExtensionAction) -> anyhow::Result<()> {
    match action {
        ExtensionAction::List => extension_cmd::run_extension_list(),
        ExtensionAction::Show { name } => extension_cmd::run_extension_show(name.as_deref()),
    }
}

#[cfg(feature = "stack-rust")]
fn run_tools(action: ToolsAction) -> anyhow::Result<ExitCode> {
    match action {
        ToolsAction::List => {
            tools_cmd::run_tools_list()?;
            Ok(ExitCode::SUCCESS)
        }
        ToolsAction::Check => tools_cmd::run_tools_check(),
        ToolsAction::Install { name } => tools_cmd::run_tools_install(name.as_deref()),
    }
}
