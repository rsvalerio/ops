//! CLI entry point and orchestration for ops.
//! Step display uses indicatif MultiProgress (one spinner per step).

mod extension_cmd;
mod new_command_cmd;
mod registry;
mod theme_cmd;
#[cfg(feature = "stack-rust")]
mod tools_cmd;

#[cfg(test)]
mod test_utils;

/// Process-wide mutex for tests that change the current working directory.
/// Rust tests run in parallel by default; `std::env::set_current_dir` is
/// process-global, so CWD-dependent tests must serialize on this lock.
///
/// # Mutex Poisoning Recovery
///
/// If a test panics while holding this lock, the mutex becomes "poisoned".
/// We intentionally recover from poisoned state (rather than propagating
/// the panic) because:
/// 1. The panic has already been reported by the test framework
/// 2. Subsequent tests should be allowed to run
/// 3. CWD restoration failure is non-critical (test isolation is best-effort)
#[cfg(test)]
pub(crate) static CWD_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// RAII guard that acquires CWD_MUTEX, switches to a target directory,
/// and restores the original CWD on drop.
///
/// # Test Isolation Note
///
/// This guard serializes CWD-dependent tests. While this prevents race
/// conditions, it means these tests cannot run in parallel with each other.
/// Prefer using `tempfile::tempdir()` and passing paths explicitly when
/// possible to avoid CWD mutations entirely.
///
/// # Rust 2024 Compatibility (E104)
///
/// `std::env::set_current_dir` is `unsafe` in Rust 2024 edition.
/// All calls are wrapped in `unsafe` blocks with SAFETY comments.
#[cfg(test)]
pub(crate) struct CwdGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    original_dir: std::path::PathBuf,
}

#[cfg(test)]
impl CwdGuard {
    pub fn new(target: &std::path::Path) -> Result<Self, std::io::Error> {
        let lock = CWD_MUTEX.lock().unwrap_or_else(|poisoned| {
            tracing::warn!("CWD_MUTEX poisoned by previous test panic, recovering");
            poisoned.into_inner()
        });
        let original_dir = std::env::current_dir()?;
        // SAFETY: Test-only. CWD_MUTEX serializes all CWD-dependent tests.
        // unsafe required in Rust 2024 edition; allow unused_unsafe for 2021.
        #[allow(unused_unsafe)]
        unsafe {
            std::env::set_current_dir(target)?
        };
        Ok(Self {
            _lock: lock,
            original_dir,
        })
    }
}

#[cfg(test)]
impl Drop for CwdGuard {
    #[allow(unused_unsafe)]
    fn drop(&mut self) {
        // SAFETY: Test-only. CWD_MUTEX serializes all CWD-dependent tests.
        if let Err(e) = unsafe { std::env::set_current_dir(&self.original_dir) } {
            tracing::warn!("CwdGuard: failed to restore original directory: {}", e);
        }
    }
}

#[cfg(test)]
mod cwd_guard_tests {
    use super::*;

    #[test]
    fn cwd_guard_changes_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");
        let current = std::env::current_dir().expect("current cwd");
        let current_canonical = current.canonicalize().unwrap_or(current);
        let dir_canonical = dir
            .path()
            .canonicalize()
            .unwrap_or(dir.path().to_path_buf());
        assert_eq!(
            current_canonical, dir_canonical,
            "should change to target directory"
        );
    }

    #[test]
    fn cwd_guard_mutex_is_recoverable() {
        let _lock = CWD_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
    }
}

pub use clap::{CommandFactory, Parser};
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::registry::{as_ext_refs, builtin_extensions, register_extension_commands};
use ops_core::config::CommandSpec;
use ops_extension::CommandRegistry;
use ops_runner::command::is_sensitive_env_key;
use ops_runner::command::StepResult;
use ops_runner::display::ProgressDisplay;

#[derive(Parser, Debug)]
#[command(
    name = "ops",
    bin_name = "ops",
    about = "Batteries-included task runner for any stack",
    version
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
        /// Skip dependency update check.
        #[arg(long)]
        skip_updates: bool,
        /// Force re-collection of data (ignores cached results).
        #[arg(long)]
        refresh: bool,
    },
    /// Interactively add a new command to `.ops.toml`.
    NewCommand,
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

fn preprocess_args(args: Vec<OsString>) -> Vec<OsString> {
    if args.len() > 1 && args[1] == "ops" {
        std::iter::once(args[0].clone())
            .chain(args.into_iter().skip(2))
            .collect()
    } else {
        args
    }
}

fn run() -> anyhow::Result<ExitCode> {
    init_logging();

    let args: Vec<OsString> = std::env::args_os().collect();
    let effective_args = preprocess_args(args);
    let cli = Cli::parse_from(effective_args);

    match cli.subcommand {
        Some(CoreSubcommand::Init {
            force,
            output,
            themes,
            commands,
        }) => {
            let sections = ops_core::config::InitSections::from_flags(output, themes, commands);
            run_init(force, sections)?;
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
        Some(CoreSubcommand::External(args)) => return run_external_command(&args, cli.dry_run),
        None => print_help()?,
    }

    Ok(ExitCode::SUCCESS)
}

fn run_external_command(args: &[OsString], dry_run: bool) -> anyhow::Result<ExitCode> {
    let name = args
        .first()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("missing command name"))?;
    run_command(name, dry_run)
}

fn print_help() -> anyhow::Result<()> {
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(config) = ops_core::config::load_config() {
            let mut runner = ops_runner::command::CommandRunner::new(config, cwd);
            if let Err(e) = setup_extensions(&mut runner) {
                tracing::debug!("failed to setup extensions for help: {}", e);
            } else {
                let commands = runner.list_command_ids();
                if !commands.is_empty() {
                    let mut err = std::io::stderr();
                    let _ = writeln!(err, "Available commands: {}", commands.join(", "));
                    let _ = writeln!(err);
                }
            }
        }
    } else {
        tracing::debug!("could not get current directory for help listing");
    }
    let mut cmd = Cli::command();
    cmd.print_help()?;
    Ok(())
}

fn run_init(force: bool, sections: ops_core::config::InitSections) -> anyhow::Result<()> {
    run_init_to(force, sections, &mut std::io::stdout())
}

fn run_init_to(
    force: bool,
    sections: ops_core::config::InitSections,
    w: &mut dyn Write,
) -> anyhow::Result<()> {
    let path = PathBuf::from(".ops.toml");
    if path.exists() && !force {
        tracing::warn!(
            "{} already exists; not overwriting (use --force to overwrite)",
            path.display()
        );
        return Ok(());
    }
    let cwd = std::env::current_dir()?;
    let content = ops_core::config::init_template(&cwd, &sections)?;
    std::fs::write(&path, content)?;
    tracing::info!("created {}", path.display());
    if sections.commands {
        let stack = ops_core::stack::Stack::detect(&cwd);
        if stack.is_some() {
            writeln!(
                w,
                "Created .ops.toml with default commands for the detected stack. Run `cargo ops <command>` (e.g. cargo ops build, cargo ops verify)."
            )?;
        } else {
            writeln!(w, "Created .ops.toml. Add commands in [commands.<name>] or run in a project with a detected stack, then run `cargo ops <command>`.")?;
        }
    } else {
        writeln!(w, "Created .ops.toml with output settings. Use `ops init --commands --themes` to include more sections.")?;
    }
    Ok(())
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

fn setup_extensions(runner: &mut ops_runner::command::CommandRunner) -> anyhow::Result<()> {
    let exts = builtin_extensions(runner.config(), runner.working_directory())?;
    let ext_refs = as_ext_refs(&exts);
    let mut cmd_registry = CommandRegistry::new();
    register_extension_commands(&ext_refs, &mut cmd_registry);
    runner.register_commands(cmd_registry);
    let mut data_registry = ops_extension::DataRegistry::new();
    crate::registry::register_extension_data_providers(&ext_refs, &mut data_registry);
    runner.register_data_providers(data_registry);
    Ok(())
}

pub(crate) fn load_config_and_cwd() -> anyhow::Result<(ops_core::config::Config, PathBuf)> {
    let config = ops_core::config::load_config()?;
    let cwd = std::env::current_dir()?;
    Ok((config, cwd))
}

fn display_cmd_for(runner: &ops_runner::command::CommandRunner, id: &str) -> String {
    match runner.resolve(id) {
        Some(CommandSpec::Exec(e)) => e.display_cmd().into_owned(),
        _ => id.to_string(),
    }
}

/// Build a display map from command IDs to their display strings.
fn build_display_map(
    runner: &ops_runner::command::CommandRunner,
    leaf_ids: &[String],
) -> std::collections::HashMap<String, String> {
    leaf_ids
        .iter()
        .map(|id| (id.clone(), display_cmd_for(runner, id)))
        .collect()
}

/// Log step results at debug level.
fn log_step_results(results: &[StepResult]) {
    for r in results {
        tracing::debug!(
            id = %r.id,
            success = r.success,
            duration_ms = r.duration.as_millis() as u64,
            stdout_len = r.stdout.len(),
            stderr_len = r.stderr.len(),
            message = ?r.message,
            "step result",
        );
    }
}

#[tracing::instrument(skip_all, fields(command = %name))]
fn run_command(name: &str, dry_run: bool) -> anyhow::Result<ExitCode> {
    let (config, cwd) = load_config_and_cwd()?;
    let mut runner = ops_runner::command::CommandRunner::new(config, cwd);
    setup_extensions(&mut runner)?;

    if dry_run {
        return run_command_dry_run(&runner, name);
    }

    let success = run_command_cli(&mut runner, name)?;

    if success {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

/// SEC-001: Preview commands without executing.
///
/// Prints the resolved command(s) that would be run, including all
/// arguments and environment variables. This is useful for:
/// - Verifying config changes before running
/// - Auditing what commands are defined
/// - Debugging composite command expansion
fn run_command_dry_run(
    runner: &ops_runner::command::CommandRunner,
    name: &str,
) -> anyhow::Result<ExitCode> {
    run_command_dry_run_to(runner, name, &mut std::io::stdout())
}

fn run_command_dry_run_to(
    runner: &ops_runner::command::CommandRunner,
    name: &str,
    w: &mut dyn Write,
) -> anyhow::Result<ExitCode> {
    let leaf_ids = runner
        .expand_to_leaves(name)
        .ok_or_else(|| anyhow::anyhow!("unknown command: {}", name))?;

    writeln!(w, "Command: {}", name)?;
    writeln!(w, "Resolved to {} step(s):", leaf_ids.len())?;

    for (i, id) in leaf_ids.iter().enumerate() {
        writeln!(w, "\n  [{}] {}", i + 1, id)?;
        match runner.resolve(id) {
            Some(CommandSpec::Exec(e)) => {
                writeln!(w, "      program: {}", e.program)?;
                if !e.args.is_empty() {
                    writeln!(w, "      args:    {}", e.args.join(" "))?;
                }
                if !e.env.is_empty() {
                    writeln!(w, "      env:")?;
                    for (k, v) in &e.env {
                        let display_val = if is_sensitive_env_key(k) {
                            "***REDACTED***"
                        } else {
                            v
                        };
                        writeln!(w, "        {}={}", k, display_val)?;
                    }
                }
                if let Some(cwd) = &e.cwd {
                    writeln!(w, "      cwd:     {}", cwd.display())?;
                }
                if let Some(timeout) = e.timeout_secs {
                    writeln!(w, "      timeout: {}s", timeout)?;
                }
            }
            Some(CommandSpec::Composite(_)) => {
                writeln!(w, "      (composite - should have been expanded)")?;
            }
            None => {
                writeln!(w, "      (unknown command)")?;
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn run_command_cli(
    runner: &mut ops_runner::command::CommandRunner,
    name: &str,
) -> anyhow::Result<bool> {
    let leaf_ids = runner
        .expand_to_leaves(name)
        .ok_or_else(|| anyhow::anyhow!("unknown command: {}", name))?;

    let display_map = build_display_map(runner, &leaf_ids);

    let mut display =
        ProgressDisplay::new(runner.output_config(), display_map, &runner.config().themes)?;

    let rt = tokio::runtime::Runtime::new()?;
    let results: Vec<StepResult> = rt.block_on(async {
        runner
            .run(name, &mut |event| display.handle_event(event))
            .await
    })?;

    log_step_results(&results);

    let success = results.iter().all(|r| r.success);
    Ok(success)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{exec_spec, TestConfigBuilder};

    // -- TQ-011: Subcommand parsing --

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

    // -- TQ-011: run_init (using CwdGuard) --

    fn all_sections() -> ops_core::config::InitSections {
        ops_core::config::InitSections::from_flags(true, true, true)
    }

    fn default_sections() -> ops_core::config::InitSections {
        ops_core::config::InitSections::from_flags(false, false, false)
    }

    #[test]
    fn run_init_creates_minimal_ops_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");
        run_init(false, default_sections()).expect("run_init should succeed");
        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(
            content.contains("[output]"),
            "should contain output section"
        );
        assert!(
            !content.contains("[themes.classic]"),
            "default init should not contain themes"
        );
    }

    #[test]
    fn run_init_all_sections_includes_themes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");
        run_init(false, all_sections()).expect("run_init should succeed");
        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(
            content.contains("[output]"),
            "should contain output section"
        );
        assert!(
            content.contains("[themes.classic]"),
            "should contain classic theme"
        );
    }

    #[test]
    fn run_init_no_overwrite_without_force() {
        let (dir, _guard) = crate::test_utils::with_temp_config("existing");
        run_init(false, default_sections()).expect("run_init should succeed (noop)");
        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert_eq!(content, "existing", "file should not be overwritten");
    }

    #[test]
    fn run_init_force_overwrites() {
        let (dir, _guard) = crate::test_utils::with_temp_config("existing");
        run_init(true, default_sections()).expect("run_init should succeed");
        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(
            content.contains("[output]"),
            "file should be overwritten with defaults"
        );
    }

    #[test]
    fn run_init_to_output_message_no_flags() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");
        let mut buf = Vec::new();
        run_init_to(false, default_sections(), &mut buf).expect("run_init_to");
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("Created .ops.toml with output settings"),
            "expected minimal message, got: {output}"
        );
    }

    #[test]
    fn run_init_to_output_message_with_commands_and_rust_stack() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Write a Cargo.toml so Stack::detect returns Some(Rust)
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");
        let mut buf = Vec::new();
        let sections = ops_core::config::InitSections::from_flags(true, false, true);
        run_init_to(false, sections, &mut buf).expect("run_init_to");
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("detected stack"),
            "expected stack message, got: {output}"
        );
    }

    // -- TQ-011: display_cmd_for --

    #[test]
    fn display_cmd_for_exec_command() {
        let config = TestConfigBuilder::new()
            .exec("build", "cargo", &["build", "--all"])
            .build();
        let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
        assert_eq!(display_cmd_for(&runner, "build"), "cargo build --all");
    }

    #[test]
    fn display_cmd_for_unknown_returns_id() {
        let config = ops_core::config::Config::default();
        let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
        assert_eq!(display_cmd_for(&runner, "missing"), "missing");
    }

    // -- TQ-011: display_cmd_for with composite command --

    #[test]
    fn display_cmd_for_composite_returns_id() {
        let config = TestConfigBuilder::new()
            .composite("verify", &["build", "test"])
            .build();
        let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
        assert_eq!(display_cmd_for(&runner, "verify"), "verify");
    }

    // -- TQ-005: Tests for preprocess_args --

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

    /// TQ-007: Full lifecycle integration test.
    ///
    /// This test validates the complete command execution path:
    /// - Config loading
    /// - Extension setup
    /// - Command resolution and execution
    /// - Event emission
    /// - Result aggregation
    ///
    /// It is ignored because it:
    /// - Spawns real subprocesses
    /// - Writes to stderr (visible in test output)
    /// - Requires `echo` to be available
    ///
    /// **Re-enable criteria:**
    /// - Run with `cargo test -- --ignored` in environments with echo available
    /// - Or mock subprocess execution using a trait-based approach
    ///
    /// **Tracking:** Run periodically in CI to validate full integration.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "TQ-007: spawns real subprocesses; run with --ignored. Validates full CLI lifecycle."]
    async fn run_command_cli_full_lifecycle() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[output]
theme = "compact"
columns = 80

[commands.echo_test]
program = "echo"
args = ["integration_test"]
"#,
        );

        let cwd = std::env::current_dir().expect("cwd");
        let config = ops_core::config::load_config().expect("load_config");
        let mut runner = ops_runner::command::CommandRunner::new(config, cwd);
        setup_extensions(&mut runner).expect("setup_extensions");

        let mut events = Vec::new();
        let results = runner
            .run("echo_test", &mut |e| events.push(e))
            .await
            .expect("run should succeed");

        assert!(
            results.iter().all(|r| r.success),
            "all steps should succeed"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, ops_runner::command::RunnerEvent::PlanStarted { .. })),
            "should emit PlanStarted"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, ops_runner::command::RunnerEvent::StepFinished { .. })),
            "should emit StepFinished"
        );
        assert!(
            events.iter().any(|e| matches!(
                e,
                ops_runner::command::RunnerEvent::RunFinished { success: true, .. }
            )),
            "should emit RunFinished with success"
        );
    }

    mod run_command_tests {
        use super::*;

        #[test]
        fn run_command_returns_error_for_unknown_command() {
            let (_dir, _guard) = crate::test_utils::with_temp_config(
                r#"
[commands.echo_test]
program = "echo"
args = ["test"]
"#,
            );

            let result = run_command("nonexistent", false);
            assert!(
                result.is_err(),
                "run_command should return error for unknown command"
            );
        }

        #[test]
        fn run_command_returns_success_for_valid_command() {
            let (_dir, _guard) = crate::test_utils::with_temp_config(
                r#"
[commands.echo_test]
program = "echo"
args = ["test"]
"#,
            );

            let result = run_command("echo_test", false);
            assert!(result.is_ok(), "run_command should not error");
            let exit_code = result.unwrap();
            assert_eq!(
                exit_code,
                ExitCode::SUCCESS,
                "valid command should return SUCCESS"
            );
        }

        #[test]
        fn run_command_returns_failure_for_failing_command() {
            let fail_cmd = if cfg!(windows) {
                r#"program = "cmd"
args = ["/C", "exit", "1"]"#
            } else {
                r#"program = "false"
args = []"#
            };
            let (_dir, _guard) = crate::test_utils::with_temp_config(&format!(
                "[commands.fail_cmd]\n{}\n",
                fail_cmd
            ));

            let result = run_command("fail_cmd", false);
            assert!(result.is_ok(), "run_command should not error");
            let exit_code = result.unwrap();
            assert_eq!(
                exit_code,
                ExitCode::FAILURE,
                "failing command should return FAILURE"
            );
        }

        #[test]
        fn run_command_returns_error_for_cycle() {
            let (_dir, _guard) = crate::test_utils::with_temp_config(
                r#"
[commands.a]
commands = ["b"]

[commands.b]
commands = ["a"]
"#,
            );

            let result = run_command("a", false);
            assert!(result.is_err(), "run_command should return error for cycle");
        }
    }

    mod build_display_map_tests {
        use super::*;

        #[test]
        fn build_display_map_with_config_commands() {
            let config = crate::test_utils::TestConfigBuilder::new()
                .exec("build", "cargo", &["build"])
                .exec("test", "cargo", &["test"])
                .build();
            let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
            let display_map = build_display_map(&runner, &["build".into(), "test".into()]);

            assert_eq!(display_map.get("build"), Some(&"cargo build".to_string()));
            assert_eq!(display_map.get("test"), Some(&"cargo test".to_string()));
        }

        #[test]
        fn build_display_map_with_unknown_command() {
            let config = ops_core::config::Config::default();
            let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
            let display_map = build_display_map(&runner, &["unknown".into()]);

            assert_eq!(display_map.get("unknown"), Some(&"unknown".to_string()));
        }

        #[test]
        fn build_display_map_with_composite_command() {
            let config = crate::test_utils::TestConfigBuilder::new()
                .composite("verify", &["build", "test"])
                .build();
            let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
            let display_map = build_display_map(&runner, &["verify".into()]);

            assert_eq!(display_map.get("verify"), Some(&"verify".to_string()));
        }

        #[test]
        fn display_cmd_for_with_extension_command() {
            let mut runner = ops_runner::command::CommandRunner::new(
                ops_core::config::Config::default(),
                PathBuf::from("."),
            );
            runner.register_commands(vec![(
                "ext_cmd".into(),
                ops_core::config::CommandSpec::Exec(ops_core::config::ExecCommandSpec {
                    program: "echo".into(),
                    args: vec!["ext".into()],
                    ..Default::default()
                }),
            )]);

            assert_eq!(display_cmd_for(&runner, "ext_cmd"), "echo ext");
        }
    }

    mod run_command_dry_run_tests {
        use super::*;

        fn build_test_runner() -> ops_runner::command::CommandRunner {
            let config = TestConfigBuilder::new()
                .exec("build", "cargo", &["build", "--all"])
                .exec("test", "cargo", &["test"])
                .command(
                    "verify",
                    ops_core::config::CommandSpec::Composite(
                        ops_core::config::CompositeCommandSpec {
                            commands: vec!["build".into(), "test".into()],
                            parallel: false,
                            fail_fast: true,
                        },
                    ),
                )
                .build();
            ops_runner::command::CommandRunner::new(config, PathBuf::from("."))
        }

        #[test]
        fn dry_run_returns_success_for_known_command() {
            let runner = build_test_runner();
            let result = run_command_dry_run(&runner, "build");
            assert!(result.is_ok(), "dry_run should succeed for known command");
            assert_eq!(result.unwrap(), ExitCode::SUCCESS);
        }

        #[test]
        fn dry_run_returns_error_for_unknown_command() {
            let runner = build_test_runner();
            let result = run_command_dry_run(&runner, "nonexistent");
            assert!(result.is_err(), "dry_run should fail for unknown command");
        }

        #[test]
        fn dry_run_expands_composite_commands() {
            let runner = build_test_runner();
            let result = run_command_dry_run(&runner, "verify");
            assert!(result.is_ok());
        }

        #[test]
        fn dry_run_shows_program_and_args() {
            let config = TestConfigBuilder::new()
                .exec("echo_test", "echo", &["hello", "world"])
                .build();
            let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
            let mut buf = Vec::new();
            let result = run_command_dry_run_to(&runner, "echo_test", &mut buf);
            assert!(result.is_ok());
            let output = String::from_utf8(buf).unwrap();
            assert!(output.contains("program: echo"), "got: {output}");
            assert!(output.contains("args:    hello world"), "got: {output}");
        }

        #[test]
        fn dry_run_shows_env_vars() {
            let mut env = std::collections::HashMap::new();
            env.insert("MY_VAR".to_string(), "my_value".to_string());
            let spec = ops_core::config::ExecCommandSpec {
                program: "echo".to_string(),
                env,
                ..Default::default()
            };
            let config = TestConfigBuilder::new()
                .command("with_env", ops_core::config::CommandSpec::Exec(spec))
                .build();
            let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
            let mut buf = Vec::new();
            let result = run_command_dry_run_to(&runner, "with_env", &mut buf);
            assert!(result.is_ok());
            let output = String::from_utf8(buf).unwrap();
            assert!(output.contains("env:"), "got: {output}");
            assert!(output.contains("MY_VAR=my_value"), "got: {output}");
        }

        #[test]
        fn dry_run_redacts_sensitive_env_vars() {
            let mut env = std::collections::HashMap::new();
            env.insert("API_KEY".to_string(), "secret123".to_string());
            env.insert("PASSWORD".to_string(), "hunter2".to_string());
            let spec = ops_core::config::ExecCommandSpec {
                program: "echo".to_string(),
                env,
                ..Default::default()
            };
            let config = TestConfigBuilder::new()
                .command("with_secrets", ops_core::config::CommandSpec::Exec(spec))
                .build();
            let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
            let mut buf = Vec::new();
            let result = run_command_dry_run_to(&runner, "with_secrets", &mut buf);
            assert!(result.is_ok());
            let output = String::from_utf8(buf).unwrap();
            assert!(
                !output.contains("secret123"),
                "should not leak secret: {output}"
            );
            assert!(
                !output.contains("hunter2"),
                "should not leak password: {output}"
            );
            assert!(
                output.contains("***REDACTED***"),
                "should show redaction: {output}"
            );
        }

        #[test]
        fn dry_run_shows_cwd_if_set() {
            let mut spec = exec_spec("echo", &[]);
            spec.cwd = Some(PathBuf::from("/custom/path"));
            let config = TestConfigBuilder::new()
                .command("with_cwd", ops_core::config::CommandSpec::Exec(spec))
                .build();
            let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
            let mut buf = Vec::new();
            let result = run_command_dry_run_to(&runner, "with_cwd", &mut buf);
            assert!(result.is_ok());
            let output = String::from_utf8(buf).unwrap();
            assert!(output.contains("cwd:     /custom/path"), "got: {output}");
        }

        #[test]
        fn dry_run_shows_timeout_if_set() {
            let mut spec = exec_spec("echo", &[]);
            spec.timeout_secs = Some(30);
            let config = TestConfigBuilder::new()
                .command("with_timeout", ops_core::config::CommandSpec::Exec(spec))
                .build();
            let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
            let mut buf = Vec::new();
            let result = run_command_dry_run_to(&runner, "with_timeout", &mut buf);
            assert!(result.is_ok());
            let output = String::from_utf8(buf).unwrap();
            assert!(output.contains("timeout: 30s"), "got: {output}");
        }

        #[test]
        fn dry_run_composite_shows_all_steps() {
            let runner = build_test_runner();
            let mut buf = Vec::new();
            let result = run_command_dry_run_to(&runner, "verify", &mut buf);
            assert!(result.is_ok());
            let output = String::from_utf8(buf).unwrap();
            assert!(output.contains("Command: verify"), "got: {output}");
            assert!(output.contains("Resolved to 2 step(s)"), "got: {output}");
            assert!(output.contains("[1] build"), "got: {output}");
            assert!(output.contains("[2] test"), "got: {output}");
            assert!(output.contains("program: cargo"), "got: {output}");
        }

        #[test]
        fn dry_run_no_args_omits_args_line() {
            let config = TestConfigBuilder::new().exec("simple", "echo", &[]).build();
            let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
            let mut buf = Vec::new();
            run_command_dry_run_to(&runner, "simple", &mut buf).unwrap();
            let output = String::from_utf8(buf).unwrap();
            assert!(output.contains("program: echo"), "got: {output}");
            assert!(!output.contains("args:"), "should omit args line: {output}");
        }
    }
}
