//! CLI entry point and orchestration for ops.

// Force the linker to retain extension crates that only register via linkme
// distributed slices (no other symbols are referenced from the main binary).
#[cfg(feature = "stack-go")]
extern crate ops_about_go;
#[cfg(any(feature = "stack-java-maven", feature = "stack-java-gradle"))]
extern crate ops_about_java;
#[cfg(feature = "stack-rust")]
extern crate ops_cargo_update;
#[cfg(feature = "stack-rust")]
extern crate ops_metadata;
#[cfg(feature = "coverage")]
extern crate ops_test_coverage;
#[cfg(feature = "tokei")]
extern crate ops_tokei;

mod args;
mod extension_cmd;
mod hook_shared;
mod init_cmd;
mod new_command_cmd;
mod registry;
mod run_before_commit_cmd;
mod run_before_push_cmd;
mod run_cmd;
mod theme_cmd;
#[cfg(feature = "stack-rust")]
mod tools_cmd;
mod tty;

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

    // If the user asked for top-level help (`ops -h` / `ops --help`), show
    // help with dynamic commands included and exit.  We intercept before clap
    // parsing because dynamic subcommands cannot be registered at parse time
    // (they would shadow the `External` catch-all).
    if is_toplevel_help(&effective_args) {
        let mut cmd = hide_irrelevant_commands(Cli::command(), detected_stack);
        cmd = inject_dynamic_commands(cmd, &early_config, detected_stack);
        let long = effective_args.iter().any(|a| a == "--help");
        if long {
            cmd.print_long_help()?;
        } else {
            cmd.print_help()?;
        }
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
        Some(CoreSubcommand::About { refresh }) => {
            let (config, cwd) = load_config_and_cwd()?;
            let registry = crate::registry::build_data_registry(&config, &cwd)?;
            let columns = config.output.columns;
            let opts = ops_about::AboutOptions { refresh };
            ops_about::run_about(&registry, &opts, columns)?;
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
            let opts = ops_about_rust::DashboardOptions {
                skip_coverage,
                refresh,
            };
            ops_about_rust::run_dashboard(&registry, &opts, &tools)?;
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
            return run_cmd::run_external_command(&args, cli.dry_run, cli.verbose)
        }
        None => {
            let mut cmd = hide_irrelevant_commands(Cli::command(), detected_stack);
            cmd = inject_dynamic_commands(cmd, early_config, detected_stack);
            cmd.print_help()?;
        }
    }

    Ok(ExitCode::SUCCESS)
}

/// Returns true when the effective args request top-level help (no subcommand).
/// E.g. `ops -h`, `ops --help`, `ops -d --help`, but NOT `ops build -h`.
fn is_toplevel_help(args: &[std::ffi::OsString]) -> bool {
    // Skip argv[0].  If any non-flag argument appears before -h/--help, the
    // user is asking for subcommand help, not top-level help.
    let mut saw_help = false;
    for a in args.iter().skip(1) {
        if a == "-h" || a == "--help" {
            saw_help = true;
        } else if !a.to_string_lossy().starts_with('-') {
            // A positional (subcommand) appeared — not top-level help.
            return false;
        }
    }
    saw_help
}

/// Inject dynamic commands (from config and stack defaults) into the clap Command for help display.
fn inject_dynamic_commands(
    mut cmd: clap::Command,
    config: &ops_core::config::Config,
    stack: Option<ops_core::stack::Stack>,
) -> clap::Command {
    use std::collections::HashSet;

    let builtins: HashSet<&str> = [
        "init",
        "theme",
        "extension",
        "new-command",
        "about",
        "dashboard",
        "deps",
        "tools",
        "run-before-commit",
        "run-before-push",
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

    // Collect all command sources: config first (higher priority), then stack defaults.
    let stack_commands = stack.map(|s| s.default_commands()).unwrap_or_default();
    let sources: Vec<(&str, &ops_core::config::CommandSpec)> = config
        .commands
        .iter()
        .map(|(n, s)| (n.as_str(), s))
        .chain(stack_commands.iter().map(|(n, s)| (n.as_str(), s)))
        .collect();

    for (name, spec) in sources {
        if builtins.contains(name) || !seen.insert(name.to_string()) {
            continue;
        }
        let about = spec
            .help()
            .map(|s| s.to_string())
            .unwrap_or_else(|| spec.display_cmd_fallback());
        let mut sub = clap::Command::new(leak(name.to_string())).about(leak(about));
        for alias in spec.aliases() {
            sub = sub.visible_alias(leak(alias.clone()));
        }
        cmd = cmd.subcommand(sub);
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

/// Prompt the user to run `ops <hook> install` when the hook command is not configured.
fn prompt_hook_install(hook_name: &str) -> anyhow::Result<ExitCode> {
    let _ = writeln!(
        std::io::stderr(),
        "No '{hook_name}' command configured in .ops.toml."
    );
    if !crate::tty::is_stdout_tty() {
        let _ = writeln!(
            std::io::stderr(),
            "Run `ops {hook_name} install` to set it up."
        );
        return Ok(ExitCode::FAILURE);
    }
    let answer = inquire::Confirm::new(&format!("Run `ops {hook_name} install` now?"))
        .with_default(true)
        .prompt()?;
    if answer {
        let status = std::process::Command::new(std::env::current_exe()?)
            .args([hook_name, "install"])
            .status()?;
        if status.success() {
            return Ok(ExitCode::SUCCESS);
        }
        return Ok(ExitCode::FAILURE);
    }
    Ok(ExitCode::SUCCESS)
}

fn run_before_commit(
    action: Option<RunBeforeCommitAction>,
    changed_only: bool,
) -> anyhow::Result<ExitCode> {
    match action {
        Some(RunBeforeCommitAction::Install) => {
            run_before_commit_cmd::run_before_commit_install()?;
            Ok(ExitCode::SUCCESS)
        }
        None => {
            let config = ops_core::config::load_config().unwrap_or_default();
            if !config.commands.contains_key("run-before-commit") {
                return prompt_hook_install("run-before-commit");
            }
            if ops_run_before_commit::should_skip() {
                let _ = writeln!(
                    std::io::stderr(),
                    "[run-before-commit] {}=1 — skipping",
                    ops_run_before_commit::SKIP_ENV_VAR
                );
                return Ok(ExitCode::SUCCESS);
            }
            if changed_only && !ops_run_before_commit::has_staged_files()? {
                let _ = writeln!(
                    std::io::stderr(),
                    "[run-before-commit] no staged files — skipping"
                );
                return Ok(ExitCode::SUCCESS);
            }
            let args = vec![std::ffi::OsString::from("run-before-commit")];
            run_cmd::run_external_command(&args, false, false)
        }
    }
}

fn run_before_push(
    action: Option<RunBeforePushAction>,
    _changed_only: bool,
) -> anyhow::Result<ExitCode> {
    match action {
        Some(RunBeforePushAction::Install) => {
            run_before_push_cmd::run_before_push_install()?;
            Ok(ExitCode::SUCCESS)
        }
        None => {
            let config = ops_core::config::load_config().unwrap_or_default();
            if !config.commands.contains_key("run-before-push") {
                return prompt_hook_install("run-before-push");
            }
            if ops_run_before_push::should_skip() {
                let _ = writeln!(
                    std::io::stderr(),
                    "[run-before-push] {}=1 — skipping",
                    ops_run_before_push::SKIP_ENV_VAR
                );
                return Ok(ExitCode::SUCCESS);
            }
            let args = vec![std::ffi::OsString::from("run-before-push")];
            run_cmd::run_external_command(&args, false, false)
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
