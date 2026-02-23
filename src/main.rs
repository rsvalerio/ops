//! CLI entry point and orchestration for ops.
//! Step display uses indicatif MultiProgress (one spinner per step).

mod command;
mod config;
mod data_cmd;
mod display;
mod extension;
mod extension_cmd;
mod extensions;
mod output;
mod serde_defaults;
mod stack;
mod style;
mod theme;
mod theme_cmd;

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
        std::env::set_current_dir(target)?;
        Ok(Self {
            _lock: lock,
            original_dir,
        })
    }
}

#[cfg(test)]
impl Drop for CwdGuard {
    fn drop(&mut self) {
        if let Err(e) = std::env::set_current_dir(&self.original_dir) {
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
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use crate::command::is_sensitive_env_key;
use crate::config::CommandSpec;
use crate::display::ProgressDisplay;
use crate::extension::{CommandRegistry, DataRegistry};
use crate::extensions::{
    builtin_extensions, register_extension_commands, register_extension_data_providers,
};
use command::StepResult;

#[derive(Parser, Debug)]
#[command(name = "ops", bin_name = "ops", about, version)]
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
    Init {
        /// Overwrite existing `.ops.toml` if present.
        #[arg(short, long)]
        force: bool,
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
    /// Manage data providers.
    Data {
        #[command(subcommand)]
        action: DataAction,
    },
    /// Manage cargo metadata collection and storage.
    #[cfg(feature = "stack-rust")]
    Metadata {
        #[command(subcommand)]
        action: MetadataAction,
    },
    /// Display workspace/project information.
    #[cfg(feature = "stack-rust")]
    About,
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
}

/// Data provider management subcommands.
#[derive(clap::Subcommand, Debug, Clone)]
pub enum DataAction {
    /// List all data providers.
    List,
    /// Show details for a specific data provider.
    Info { name: String },
}

/// Metadata management subcommands.
#[cfg(feature = "stack-rust")]
#[derive(clap::Subcommand, Debug, Clone)]
pub enum MetadataAction {
    /// Run cargo metadata and save JSON to staging directory.
    Collect,
    /// Load staged JSON into DuckDB and delete the file.
    Load,
    /// Collect + load in sequence.
    Refresh,
}

/// Flatten `CoreSubcommand` so `ops verify` works directly.
/// The `ops` prefix from `cargo ops ...` is stripped before parsing.
pub type Subcommand = CoreSubcommand;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {:#}", e);
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
        .with(EnvFilter::from_default_env().add_directive(log_level))
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
        Some(CoreSubcommand::Init { force }) => run_init(force)?,
        Some(CoreSubcommand::Theme { action }) => run_theme(action)?,
        Some(CoreSubcommand::Extension { action }) => run_extension(action)?,
        Some(CoreSubcommand::Data { action }) => run_data(action)?,
        #[cfg(feature = "stack-rust")]
        Some(CoreSubcommand::Metadata { action }) => run_metadata(action)?,
        #[cfg(feature = "stack-rust")]
        Some(CoreSubcommand::About) => extensions::about::run_about()?,
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
        if let Ok(config) = config::load_config() {
            let mut runner = command::CommandRunner::new(config, cwd);
            if let Err(e) = setup_extensions(&mut runner) {
                tracing::debug!("failed to setup extensions for help: {}", e);
            } else {
                let commands = runner.list_command_ids();
                if !commands.is_empty() {
                    eprintln!("Available commands: {}", commands.join(", "));
                    eprintln!();
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

fn run_init(force: bool) -> anyhow::Result<()> {
    let path = PathBuf::from(".ops.toml");
    if path.exists() && !force {
        tracing::warn!(
            "{} already exists; not overwriting (use --force to overwrite)",
            path.display()
        );
        return Ok(());
    }
    let content = config::default_ops_toml();
    std::fs::write(&path, content)?;
    tracing::info!("created {}", path.display());
    println!("Created .ops.toml. Edit it to add commands, then run `cargo ops <command>` to run a command.");
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
    }
}

fn run_data(action: DataAction) -> anyhow::Result<()> {
    match action {
        DataAction::List => data_cmd::run_data_list(),
        DataAction::Info { name } => data_cmd::run_data_info(&name),
    }
}

#[cfg(feature = "stack-rust")]
fn run_metadata(action: MetadataAction) -> anyhow::Result<()> {
    use crate::extension::Context;
    use crate::extensions::metadata::{
        collect_metadata, default_data_dir, default_db_path, load_metadata, refresh_metadata,
    };
    use crate::extensions::ops_db::OpsDb;
    use std::sync::Arc;

    let cwd = std::env::current_dir()?;
    let db_path = default_db_path(&cwd);
    let data_dir = default_data_dir(&cwd);
    let db = OpsDb::open(&db_path)?;
    let config = Arc::new(crate::config::Config::default());
    let ctx = Context::new(config, cwd);

    match action {
        MetadataAction::Collect => {
            collect_metadata(&ctx, &data_dir)?;
            println!(
                "Collected metadata to {}",
                data_dir.join("metadata.json").display()
            );
        }
        MetadataAction::Load => {
            load_metadata(&data_dir, &db)?;
            println!("Loaded metadata into DuckDB");
        }
        MetadataAction::Refresh => {
            refresh_metadata(&ctx, &data_dir, &db)?;
            println!("Refreshed metadata in DuckDB");
        }
    }
    Ok(())
}

fn setup_extensions(runner: &mut command::CommandRunner) -> anyhow::Result<()> {
    let exts = builtin_extensions(runner.config(), runner.working_directory())?;
    let ext_refs: Vec<&dyn crate::extension::Extension> = exts.iter().map(|b| b.as_ref()).collect();
    let mut cmd_registry = CommandRegistry::new();
    register_extension_commands(&ext_refs, &mut cmd_registry);
    runner.register_commands(cmd_registry);
    let mut data_registry = DataRegistry::new();
    register_extension_data_providers(&ext_refs, &mut data_registry);
    runner.register_data_providers(data_registry);
    Ok(())
}

fn display_cmd_for(runner: &command::CommandRunner, id: &str) -> String {
    match runner.resolve(id) {
        Some(CommandSpec::Exec(e)) => e.display_cmd().into_owned(),
        _ => id.to_string(),
    }
}

/// Build a display map from command IDs to their display strings.
fn build_display_map(
    runner: &command::CommandRunner,
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
    let cwd = std::env::current_dir()?;
    let config = config::load_config()?;
    let mut runner = command::CommandRunner::new(config, cwd);
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
fn run_command_dry_run(runner: &command::CommandRunner, name: &str) -> anyhow::Result<ExitCode> {
    let leaf_ids = runner
        .expand_to_leaves(name)
        .ok_or_else(|| anyhow::anyhow!("unknown command: {}", name))?;

    println!("Command: {}", name);
    println!("Resolved to {} step(s):", leaf_ids.len());

    for (i, id) in leaf_ids.iter().enumerate() {
        println!("\n  [{}] {}", i + 1, id);
        match runner.resolve(id) {
            Some(CommandSpec::Exec(e)) => {
                println!("      program: {}", e.program);
                if !e.args.is_empty() {
                    println!("      args:    {}", e.args.join(" "));
                }
                if !e.env.is_empty() {
                    println!("      env:");
                    for (k, v) in &e.env {
                        let display_val = if is_sensitive_env_key(k) {
                            "***REDACTED***"
                        } else {
                            v
                        };
                        println!("        {}={}", k, display_val);
                    }
                }
                if let Some(cwd) = &e.cwd {
                    println!("      cwd:     {}", cwd.display());
                }
                if let Some(timeout) = e.timeout_secs {
                    println!("      timeout: {}s", timeout);
                }
            }
            Some(CommandSpec::Composite(_)) => {
                println!("      (composite - should have been expanded)");
            }
            None => {
                println!("      (unknown command)");
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn run_command_cli(runner: &mut command::CommandRunner, name: &str) -> anyhow::Result<bool> {
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
    use crate::test_utils::TestConfigBuilder;

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
            Some(CoreSubcommand::Init { force: false })
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
    fn parse_no_subcommand() {
        let cli = Cli::parse_from(["ops"]);
        assert!(cli.subcommand.is_none());
    }

    // -- TQ-011: run_init (using CwdGuard) --

    #[test]
    fn run_init_creates_ops_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");
        run_init(false).expect("run_init should succeed");
        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(content.contains("[output]"), "should contain output section");
        assert!(content.contains("[themes.classic]"), "should contain classic theme");
    }

    #[test]
    fn run_init_no_overwrite_without_force() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".ops.toml"), "existing").unwrap();
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");
        run_init(false).expect("run_init should succeed (noop)");
        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert_eq!(content, "existing", "file should not be overwritten");
    }

    #[test]
    fn run_init_force_overwrites() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".ops.toml"), "existing").unwrap();
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");
        run_init(true).expect("run_init should succeed");
        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(
            content.contains("[output]"),
            "file should be overwritten with defaults"
        );
    }

    // -- TQ-011: display_cmd_for --

    #[test]
    fn display_cmd_for_exec_command() {
        let config = TestConfigBuilder::new()
            .exec("build", "cargo", &["build", "--all"])
            .build();
        let runner = command::CommandRunner::new(config, PathBuf::from("."));
        assert_eq!(display_cmd_for(&runner, "build"), "cargo build --all");
    }

    #[test]
    fn display_cmd_for_unknown_returns_id() {
        let config = config::Config::default();
        let runner = command::CommandRunner::new(config, PathBuf::from("."));
        assert_eq!(display_cmd_for(&runner, "missing"), "missing");
    }

    // -- TQ-011: display_cmd_for with composite command --

    #[test]
    fn display_cmd_for_composite_returns_id() {
        let config = TestConfigBuilder::new()
            .composite("verify", &["build", "test"])
            .build();
        let runner = command::CommandRunner::new(config, PathBuf::from("."));
        assert_eq!(display_cmd_for(&runner, "verify"), "verify");
    }

    // -- TQ-005: Tests for preprocess_args --

    #[test]
    fn preprocess_args_strips_ops_prefix() {
        let args: Vec<OsString> = vec!["ops".into(), "ops".into(), "build".into()];
        let result = preprocess_args(args);
        assert_eq!(
            result,
            vec![OsString::from("ops"), OsString::from("build")]
        );
    }

    #[test]
    fn preprocess_args_preserves_all_after_ops() {
        let args: Vec<OsString> = vec![
            "ops".into(),
            "ops".into(),
            "run".into(),
            "mycommand".into(),
        ];
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
        assert_eq!(
            result,
            vec![OsString::from("ops"), OsString::from("build")]
        );
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
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join(".ops.toml"),
            r#"
[output]
theme = "compact"
columns = 80

[commands.echo_test]
program = "echo"
args = ["integration_test"]
"#,
        )
        .unwrap();
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");

        let cwd = std::env::current_dir().expect("cwd");
        let config = config::load_config().expect("load_config");
        let mut runner = command::CommandRunner::new(config, cwd);
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
                .any(|e| matches!(e, command::RunnerEvent::PlanStarted { .. })),
            "should emit PlanStarted"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, command::RunnerEvent::StepFinished { .. })),
            "should emit StepFinished"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, command::RunnerEvent::RunFinished { success: true, .. })),
            "should emit RunFinished with success"
        );
    }

    mod run_command_tests {
        use super::*;

        #[test]
        fn run_command_returns_error_for_unknown_command() {
            let dir = tempfile::tempdir().expect("tempdir");
            std::fs::write(
                dir.path().join(".ops.toml"),
                r#"
[commands.echo_test]
program = "echo"
args = ["test"]
"#,
            )
            .unwrap();
            let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");

            let result = run_command("nonexistent", false);
            assert!(
                result.is_err(),
                "run_command should return error for unknown command"
            );
        }

        #[test]
        fn run_command_returns_success_for_valid_command() {
            let dir = tempfile::tempdir().expect("tempdir");
            std::fs::write(
                dir.path().join(".ops.toml"),
                r#"
[commands.echo_test]
program = "echo"
args = ["test"]
"#,
            )
            .unwrap();
            let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");

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
            let dir = tempfile::tempdir().expect("tempdir");
            let fail_cmd = if cfg!(windows) {
                r#"program = "cmd"
args = ["/C", "exit", "1"]"#
            } else {
                r#"program = "false"
args = []"#
            };
            std::fs::write(
                dir.path().join(".ops.toml"),
                format!(
                    r#"
[commands.fail_cmd]
{}
"#,
                    fail_cmd
                ),
            )
            .unwrap();
            let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");

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
            let dir = tempfile::tempdir().expect("tempdir");
            std::fs::write(
                dir.path().join(".ops.toml"),
                r#"
[commands.a]
commands = ["b"]

[commands.b]
commands = ["a"]
"#,
            )
            .unwrap();
            let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");

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
            let runner = command::CommandRunner::new(config, PathBuf::from("."));
            let display_map = build_display_map(&runner, &["build".into(), "test".into()]);

            assert_eq!(display_map.get("build"), Some(&"cargo build".to_string()));
            assert_eq!(display_map.get("test"), Some(&"cargo test".to_string()));
        }

        #[test]
        fn build_display_map_with_unknown_command() {
            let config = crate::config::Config::default();
            let runner = command::CommandRunner::new(config, PathBuf::from("."));
            let display_map = build_display_map(&runner, &["unknown".into()]);

            assert_eq!(display_map.get("unknown"), Some(&"unknown".to_string()));
        }

        #[test]
        fn build_display_map_with_composite_command() {
            let config = crate::test_utils::TestConfigBuilder::new()
                .composite("verify", &["build", "test"])
                .build();
            let runner = command::CommandRunner::new(config, PathBuf::from("."));
            let display_map = build_display_map(&runner, &["verify".into()]);

            assert_eq!(display_map.get("verify"), Some(&"verify".to_string()));
        }

        #[test]
        fn display_cmd_for_with_extension_command() {
            let mut runner =
                command::CommandRunner::new(crate::config::Config::default(), PathBuf::from("."));
            runner.register_commands(vec![(
                "ext_cmd".into(),
                crate::config::CommandSpec::Exec(crate::config::ExecCommandSpec {
                    program: "echo".into(),
                    args: vec!["ext".into()],
                    env: std::collections::HashMap::new(),
                    cwd: None,
                    timeout_secs: None,
                }),
            )]);

            assert_eq!(display_cmd_for(&runner, "ext_cmd"), "echo ext");
        }
    }

    mod run_command_dry_run_tests {
        use super::*;

        fn build_test_runner() -> command::CommandRunner {
            let config = TestConfigBuilder::new()
                .exec("build", "cargo", &["build", "--all"])
                .exec("test", "cargo", &["test"])
                .command(
                    "verify",
                    crate::config::CommandSpec::Composite(crate::config::CompositeCommandSpec {
                        commands: vec!["build".into(), "test".into()],
                        parallel: false,
                        fail_fast: true,
                    }),
                )
                .build();
            command::CommandRunner::new(config, PathBuf::from("."))
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
            let runner = command::CommandRunner::new(config, PathBuf::from("."));
            let result = run_command_dry_run(&runner, "echo_test");
            assert!(result.is_ok());
        }

        #[test]
        fn dry_run_shows_env_vars() {
            let mut env = std::collections::HashMap::new();
            env.insert("MY_VAR".to_string(), "my_value".to_string());
            let spec = crate::config::ExecCommandSpec {
                program: "echo".to_string(),
                args: vec![],
                env,
                cwd: None,
                timeout_secs: None,
            };
            let config = TestConfigBuilder::new()
                .command("with_env", crate::config::CommandSpec::Exec(spec))
                .build();
            let runner = command::CommandRunner::new(config, PathBuf::from("."));
            let result = run_command_dry_run(&runner, "with_env");
            assert!(result.is_ok());
        }

        #[test]
        fn dry_run_redacts_sensitive_env_vars() {
            let mut env = std::collections::HashMap::new();
            env.insert("API_KEY".to_string(), "secret123".to_string());
            env.insert("PASSWORD".to_string(), "hunter2".to_string());
            let spec = crate::config::ExecCommandSpec {
                program: "echo".to_string(),
                args: vec![],
                env,
                cwd: None,
                timeout_secs: None,
            };
            let config = TestConfigBuilder::new()
                .command("with_secrets", crate::config::CommandSpec::Exec(spec))
                .build();
            let runner = command::CommandRunner::new(config, PathBuf::from("."));
            let result = run_command_dry_run(&runner, "with_secrets");
            assert!(result.is_ok());
        }

        #[test]
        fn dry_run_shows_cwd_if_set() {
            let spec = crate::config::ExecCommandSpec {
                program: "echo".to_string(),
                args: vec![],
                env: std::collections::HashMap::new(),
                cwd: Some(PathBuf::from("/custom/path")),
                timeout_secs: None,
            };
            let config = TestConfigBuilder::new()
                .command("with_cwd", crate::config::CommandSpec::Exec(spec))
                .build();
            let runner = command::CommandRunner::new(config, PathBuf::from("."));
            let result = run_command_dry_run(&runner, "with_cwd");
            assert!(result.is_ok());
        }

        #[test]
        fn dry_run_shows_timeout_if_set() {
            let spec = crate::config::ExecCommandSpec {
                program: "echo".to_string(),
                args: vec![],
                env: std::collections::HashMap::new(),
                cwd: None,
                timeout_secs: Some(30),
            };
            let config = TestConfigBuilder::new()
                .command("with_timeout", crate::config::CommandSpec::Exec(spec))
                .build();
            let runner = command::CommandRunner::new(config, PathBuf::from("."));
            let result = run_command_dry_run(&runner, "with_timeout");
            assert!(result.is_ok());
        }
    }
}
