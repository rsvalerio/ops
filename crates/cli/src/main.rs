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
#[cfg(feature = "stack-terraform")]
extern crate ops_about_terraform;
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
mod pre_hook_cmd;
mod registry;
mod row;
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

/// Error sentinel that lets a fallible code path bubble a specific exit
/// code through `anyhow::Error`. Attach it via
/// `anyhow::Error::context(ExitCodeOverride(code))` (or `err.context(...)`)
/// and `main` will surface that code instead of the generic
/// `ExitCode::FAILURE`. Without this, e.g. an `Err(...)` that wants SIGINT
/// semantics (130) or SIGPIPE (141) silently collapses to exit 1, which
/// breaks shell scripts that distinguish cancellation from real failure.
/// Named constant for the SIGINT exit convention so the literal `130` is
/// not repeated at user-cancel call sites.
pub(crate) const SIGINT_EXIT: u8 = 130;

#[derive(Debug)]
pub(crate) struct ExitCodeOverride(pub u8);

impl std::fmt::Display for ExitCodeOverride {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "exit code override: {}", self.0)
    }
}

impl std::error::Error for ExitCodeOverride {}

fn extract_exit_code_override(err: &anyhow::Error) -> Option<u8> {
    // anyhow surfaces context values via `downcast_ref` on the error itself,
    // not via the `chain()` iterator (which only walks `std::error::Error`
    // sources). Check the top-level error first, then walk any nested
    // anyhow::Error sources for completeness.
    if let Some(o) = err.downcast_ref::<ExitCodeOverride>() {
        return Some(o.0);
    }
    err.chain()
        .find_map(|cause| cause.downcast_ref::<ExitCodeOverride>())
        .map(|o| o.0)
}

fn exit_code_for_error(err: &anyhow::Error) -> ExitCode {
    extract_exit_code_override(err).map_or(ExitCode::FAILURE, ExitCode::from)
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            ops_core::ui::error(format!("{e:#}"));
            exit_code_for_error(&e)
        }
    }
}

fn parse_log_level<W: io::Write>(
    raw: Option<&str>,
    warn: &mut W,
) -> tracing_subscriber::filter::Directive {
    // Treat unset and empty/whitespace-only the same.
    // `OPS_LOG_LEVEL=` (e.g. `${LEVEL:-}` with `LEVEL` unset) maps to
    // `Ok("")`, which would otherwise trip the directive parser and print a
    // spurious "invalid OPS_LOG_LEVEL=''" warning on every CI run. Mirrors
    // the empty-string handling in `env_flag_enabled` (subcommands.rs).
    let Some(v) = raw.filter(|s| !s.trim().is_empty()) else {
        return tracing_subscriber::filter::LevelFilter::INFO.into();
    };
    // Bare `info`/`debug`/etc. is the documented form. Anything else (target
    // directives like `ops=debug`) falls through to Directive parse so we
    // do not narrow accepted syntax.
    if let Ok(level) = v.parse::<tracing_subscriber::filter::LevelFilter>() {
        return level.into();
    }
    match v.parse::<tracing_subscriber::filter::Directive>() {
        Ok(d) => d,
        Err(e) => {
            // The tracing subscriber is not yet registered when init_logging
            // runs, so any tracing::* event is dropped — write directly.
            //
            // The writeln result is intentionally discarded: if stderr is
            // unwritable (closed pipe, broken consumer), the user has already
            // lost the diagnostic channel — propagating the write error up
            // would abort startup over a secondary concern (the log level is
            // still correctly defaulted to INFO below).
            let _ = writeln!(
                warn,
                "ops: warning: invalid OPS_LOG_LEVEL='{v}': {e}; falling back to info"
            );
            tracing_subscriber::filter::LevelFilter::INFO.into()
        }
    }
}

fn init_logging() {
    let raw = std::env::var("OPS_LOG_LEVEL").ok();
    let log_level = parse_log_level(raw.as_deref(), &mut io::stderr());
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
    // A malformed `.ops.toml` previously degraded silently to
    // `Config::default()`, hiding the real cause behind downstream
    // "unknown command" errors. The shared helper logs to `tracing::warn!`
    // *and* surfaces a user-visible note so `ops --help` / `ops init` still
    // work without hiding the parse diagnostic.
    //
    // Wrap the loaded config in an Arc once at the top-level entry so
    // `dispatch` and the run-path (build_runner →
    // CommandRunner::from_arc_config) can share one allocation. Eliminates
    // the per-invocation deep clone in build_runner.
    // READ-5 / TASK-1446: capture the workspace root at the CLI boundary so
    // the config loader is not implicitly coupled to the live process cwd.
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            ops_core::ui::error(format!("could not determine working directory: {e}"));
            return Ok(ExitCode::FAILURE);
        }
    };
    let early_config: std::sync::Arc<ops_core::config::Config> =
        std::sync::Arc::new(ops_core::config::load_config_or_default_at(&cwd, "early"));
    let detected_stack = ops_core::stack::Stack::resolve(early_config.stack.as_deref(), &cwd);

    // PERF-1 (TASK-1368): `Cli::command()` walks the full derive metadata
    // and rebuilds the clap command tree from scratch on every call.
    // Build it exactly once for this invocation; cheap `clap::Command::clone`
    // covers the rare case where both `run()` and `dispatch()` need a
    // consumed copy (parse path followed by a `None`-subcommand help
    // render).
    let built_cmd = hide_irrelevant_commands(Cli::command(), detected_stack);

    // If the user asked for top-level help (`ops -h` / `ops --help`), show
    // help with dynamic commands included and exit.  We intercept before clap
    // parsing because dynamic subcommands cannot be registered at parse time
    // (they would shadow the `External` catch-all).
    if is_toplevel_help(&effective_args) {
        let long = effective_args.iter().any(|a| a == "--help");
        print_categorized_help(built_cmd, &early_config, detected_stack, long);
        return Ok(ExitCode::SUCCESS);
    }

    let parse_cmd = built_cmd.clone();
    let mut matches = parse_cmd.get_matches_from(effective_args);
    let cli = Cli::from_arg_matches_mut(&mut matches).unwrap_or_else(|e: clap::Error| e.exit());

    dispatch(cli, &early_config, detected_stack, built_cmd)
}

fn dispatch(
    cli: Cli,
    early_config: &std::sync::Arc<ops_core::config::Config>,
    detected_stack: Option<ops_core::stack::Stack>,
    built_cmd: clap::Command,
) -> anyhow::Result<ExitCode> {
    // The same Config loaded once in `run()` is threaded
    // through every handler so a single CLI invocation reads `.ops.toml`
    // exactly once. Previously dispatch -> handler -> `load_config_and_cwd`
    // re-loaded the file with a stricter (hard-error) policy than the
    // early `load_config_or_default("early")`, so a malformed manifest
    // could succeed `--help` and bail later in the same run.
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
        Some(CoreSubcommand::Theme { action }) => run_theme(early_config, action)?,
        Some(CoreSubcommand::Extension { action }) => run_extension(early_config, action)?,
        Some(CoreSubcommand::NewCommand) => {
            let cwd = cwd()?;
            new_command_cmd::run_new_command(&cwd)?;
        }
        Some(CoreSubcommand::RunBeforeCommit {
            changed_only,
            action,
        }) => return run_before_commit(std::sync::Arc::clone(early_config), action, changed_only),
        Some(CoreSubcommand::RunBeforePush { action }) => {
            return run_before_push(std::sync::Arc::clone(early_config), action);
        }
        Some(CoreSubcommand::About { refresh, action }) => {
            run_about(early_config, refresh, action)?
        }
        #[cfg(feature = "stack-rust")]
        Some(CoreSubcommand::Deps { refresh }) => run_deps(early_config, refresh)?,
        Some(CoreSubcommand::Tools { action }) => {
            #[cfg(feature = "stack-rust")]
            {
                return run_tools(early_config, action);
            }
            #[cfg(not(feature = "stack-rust"))]
            {
                let _ = action;
                anyhow::bail!("tools subcommand requires the stack-rust feature");
            }
        }
        #[cfg(feature = "stack-terraform")]
        Some(CoreSubcommand::Plans(opts)) => return ops_tfplan::run_plan_pipeline(opts),
        Some(CoreSubcommand::External(args)) => {
            return run_cmd::run_external_command(
                std::sync::Arc::clone(early_config),
                &args,
                run_cmd::RunOptions {
                    dry_run: cli.dry_run,
                    verbose: cli.verbose,
                    tap: cli.tap,
                    raw: cli.raw,
                    ..Default::default()
                },
            )
        }
        None => {
            // PERF-1 (TASK-1368): reuse the pre-built command tree from
            // `run()` instead of re-walking the clap derive metadata.
            print_categorized_help(built_cmd, early_config, detected_stack, false);
        }
    }

    Ok(ExitCode::SUCCESS)
}

/// CLI-level cwd lookup. The pre-resolved `Config` is threaded by the
/// caller — we only need the current directory at the handler boundary.
///
/// Attach an anyhow context so a failing `current_dir()`
/// (deleted cwd, permission denied on a parent component, very long path)
/// surfaces with the operation name rather than a bare
/// `No such file or directory (os error 2)`. This helper is routed through
/// by every CLI subcommand that needs cwd, so the context applies workspace-
/// wide.
pub(crate) fn cwd() -> anyhow::Result<PathBuf> {
    use anyhow::Context as _;
    std::env::current_dir().context("failed to read current working directory")
}

#[cfg(test)]
mod cwd_tests {
    use super::cwd;

    /// When `std::env::current_dir()` fails, the error surfaced to the
    /// user must carry the `failed to read current working directory`
    /// context rather than a bare `No such file or directory (os error 2)`.
    /// Reproduce a failing current_dir() by cwd'ing into a tempdir and
    /// removing it from under ourselves. Linux/macOS only — Windows does
    /// not let you remove the active cwd, so the failure mode does not
    /// exist in this form.
    #[cfg(unix)]
    #[test]
    fn cwd_attaches_context_when_current_dir_fails() {
        // Acquire the process-wide CWD mutex via CwdGuard so this test does
        // not race with other CWD-dependent tests; the guard's drop restores
        // a valid cwd even though we delete the tempdir mid-test.
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = crate::CwdGuard::new(dir.path()).expect("CwdGuard");
        std::fs::remove_dir(dir.path()).expect("remove cwd");

        let err = cwd().expect_err("current_dir() must fail when cwd is gone");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("failed to read current working directory"),
            "error must carry the cwd context, got: {msg}"
        );
    }
}

#[cfg(test)]
mod exit_code_tests {
    use super::{extract_exit_code_override, ExitCodeOverride};

    #[test]
    fn override_surfaces_specific_code() {
        let err = anyhow::anyhow!("cancelled").context(ExitCodeOverride(130));
        assert_eq!(extract_exit_code_override(&err), Some(130));
    }

    #[test]
    fn no_override_returns_none() {
        let err = anyhow::anyhow!("boom");
        assert_eq!(extract_exit_code_override(&err), None);
    }

    #[test]
    fn override_works_through_nested_context() {
        let err = anyhow::anyhow!("io")
            .context(ExitCodeOverride(141))
            .context("while writing output");
        assert_eq!(extract_exit_code_override(&err), Some(141));
    }
}

#[cfg(test)]
mod log_level_tests {
    use super::parse_log_level;

    #[test]
    fn invalid_value_writes_visible_warning() {
        let mut buf: Vec<u8> = Vec::new();
        let bad = "!!not-a-level!!";
        let _ = parse_log_level(Some(bad), &mut buf);
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("invalid OPS_LOG_LEVEL"), "got: {out}");
        assert!(out.contains(bad), "should include offending value: {out}");
    }

    #[test]
    fn valid_value_writes_nothing() {
        let mut buf: Vec<u8> = Vec::new();
        let _ = parse_log_level(Some("debug"), &mut buf);
        assert!(buf.is_empty(), "no warning for valid level");
    }

    #[test]
    fn unset_writes_nothing() {
        let mut buf: Vec<u8> = Vec::new();
        let _ = parse_log_level(None, &mut buf);
        assert!(buf.is_empty(), "no warning when unset");
    }

    /// `OPS_LOG_LEVEL=` (set to empty) and
    /// `OPS_LOG_LEVEL="   "` (whitespace only) must be treated the same as
    /// unset. Without this, a CI matrix that uses `${LEVEL:-}` prints
    /// `ops: warning: invalid OPS_LOG_LEVEL=''` on every invocation, drowning
    /// real typos in the same channel.
    #[test]
    fn empty_string_writes_nothing() {
        let mut buf: Vec<u8> = Vec::new();
        let _ = parse_log_level(Some(""), &mut buf);
        assert!(
            buf.is_empty(),
            "no warning for empty OPS_LOG_LEVEL, got: {buf:?}"
        );
    }

    #[test]
    fn whitespace_only_writes_nothing() {
        let mut buf: Vec<u8> = Vec::new();
        let _ = parse_log_level(Some("   "), &mut buf);
        assert!(
            buf.is_empty(),
            "no warning for whitespace-only OPS_LOG_LEVEL, got: {buf:?}"
        );
    }

    #[test]
    fn empty_and_whitespace_return_info_default() {
        for v in [Some(""), Some("   "), Some("\t\n"), None] {
            let mut buf: Vec<u8> = Vec::new();
            let directive = parse_log_level(v, &mut buf);
            let s = format!("{directive}");
            assert!(
                s.contains("info"),
                "{v:?} must yield INFO default, got: {s}"
            );
        }
    }

    #[test]
    fn failed_write_still_returns_info_fallback() {
        struct FailingWriter;
        impl std::io::Write for FailingWriter {
            fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "broken",
                ))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "broken",
                ))
            }
        }
        let directive = parse_log_level(Some("!!bad!!"), &mut FailingWriter);
        // The returned directive should still be the INFO fallback despite
        // the writeln failure — the swallow is documented and intentional.
        let s = format!("{directive}");
        assert!(
            s.contains("info"),
            "expected INFO fallback directive, got: {s}"
        );
    }
}
