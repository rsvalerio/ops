//! CLI entry point and orchestration for ops.

mod args;
mod extension_cmd;
mod init_cmd;
mod new_command_cmd;
mod pre_commit_cmd;
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

    // Load config early so stack detection and help output can use it.
    let early_config = ops_core::config::load_config().unwrap_or_default();
    let detected_stack = {
        let cwd = std::env::current_dir().unwrap_or_default();
        ops_core::stack::Stack::resolve(early_config.stack.as_deref(), &cwd)
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
        Some(CoreSubcommand::PreCommit { action }) => return run_pre_commit(action),
        #[cfg(feature = "stack-rust")]
        Some(CoreSubcommand::About { refresh }) => {
            let (config, cwd) = load_config_and_cwd()?;
            let registry = crate::registry::build_data_registry(&config, &cwd)?;
            let opts = ops_about::AboutOptions { refresh };
            ops_about::run_about(&registry, &opts)?;
        }
        #[cfg(feature = "stack-rust")]
        Some(CoreSubcommand::Deps { refresh }) => {
            let (config, cwd) = load_config_and_cwd()?;
            let registry = crate::registry::build_data_registry(&config, &cwd)?;
            let opts = ops_deps::DepsOptions { refresh };
            ops_deps::run_deps(&registry, &opts)?;
        }
        #[cfg(feature = "stack-rust")]
        Some(CoreSubcommand::Dashboard {
            skip_coverage,
            refresh,
        }) => {
            let (config, cwd) = load_config_and_cwd()?;
            let registry = crate::registry::build_data_registry(&config, &cwd)?;
            let tools = ops_tools::collect_tools(&config.tools);
            let opts = ops_about::DashboardOptions {
                skip_coverage,
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
            let cmd = hide_irrelevant_commands(Cli::command(), detected_stack);
            let mut cmd = inject_dynamic_commands(cmd, &early_config, detected_stack);
            cmd.print_help()?;
        }
    }

    Ok(ExitCode::SUCCESS)
}

/// Inject dynamic commands (from config and stack defaults) into the clap Command for help display.
fn inject_dynamic_commands(
    mut cmd: clap::Command,
    config: &ops_core::config::Config,
    stack: Option<ops_core::stack::Stack>,
) -> clap::Command {
    use std::collections::HashSet;

    // Built-in subcommand names to skip.
    let builtins: HashSet<&str> = [
        "init",
        "theme",
        "extension",
        "new-command",
        "about",
        "dashboard",
        "deps",
        "tools",
        "pre-commit",
        "help",
    ]
    .into_iter()
    .collect();

    let mut seen = HashSet::new();

    // Helper: leak a String into a &'static str.
    // Safe here because this runs once at process exit (help display).
    fn leak(s: String) -> &'static str {
        Box::leak(s.into_boxed_str())
    }

    // Config commands first (higher priority).
    for (name, spec) in &config.commands {
        if builtins.contains(name.as_str()) || !seen.insert(name.clone()) {
            continue;
        }
        let about = spec
            .help()
            .map(|s| s.to_string())
            .unwrap_or_else(|| spec.display_cmd_fallback());
        cmd = cmd.subcommand(clap::Command::new(leak(name.clone())).about(leak(about)));
    }

    // Stack default commands.
    if let Some(stack) = stack {
        for (name, spec) in stack.default_commands() {
            if builtins.contains(name.as_str()) || !seen.insert(name.clone()) {
                continue;
            }
            let about = spec
                .help()
                .map(|s| s.to_string())
                .unwrap_or_else(|| spec.display_cmd_fallback());
            cmd = cmd.subcommand(clap::Command::new(leak(name)).about(leak(about)));
        }
    }

    cmd
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

fn run_pre_commit(action: Option<PreCommitAction>) -> anyhow::Result<ExitCode> {
    match action {
        Some(PreCommitAction::Install) => {
            pre_commit_cmd::run_pre_commit_install()?;
            Ok(ExitCode::SUCCESS)
        }
        None => {
            if ops_pre_commit::should_skip() {
                let _ = writeln!(
                    std::io::stderr(),
                    "[pre-commit] {}=1 — skipping",
                    ops_pre_commit::SKIP_ENV_VAR
                );
                return Ok(ExitCode::SUCCESS);
            }
            // No subcommand: run the configured `pre-commit` command from .ops.toml
            let args = vec![std::ffi::OsString::from("pre-commit")];
            run_cmd::run_external_command(&args, false)
        }
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
