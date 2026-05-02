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

fn parse_log_level<W: io::Write>(
    raw: Option<&str>,
    warn: &mut W,
) -> tracing_subscriber::filter::Directive {
    let Some(v) = raw else {
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
    // ERR-4: a malformed `.ops.toml` previously degraded silently to
    // `Config::default()`, hiding the real cause behind downstream
    // "unknown command" errors. The shared helper logs to `tracing::warn!`
    // *and* surfaces a user-visible note so `ops --help` / `ops init` still
    // work without hiding the parse diagnostic. (DUP-3 / TASK-0345)
    // OWN-2 / TASK-0841: wrap the loaded config in an Arc once at the
    // top-level entry so `dispatch` and the run-path (build_runner →
    // CommandRunner::from_arc_config) can share one allocation. Eliminates
    // the per-invocation deep clone in build_runner.
    let early_config: std::sync::Arc<ops_core::config::Config> =
        std::sync::Arc::new(ops_core::config::load_config_or_default("early"));
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
    early_config: &std::sync::Arc<ops_core::config::Config>,
    detected_stack: Option<ops_core::stack::Stack>,
) -> anyhow::Result<ExitCode> {
    // ERR-1 (TASK-0427): the same Config loaded once in `run()` is threaded
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
        Some(CoreSubcommand::NewCommand) => new_command_cmd::run_new_command()?,
        Some(CoreSubcommand::RunBeforeCommit {
            changed_only,
            action,
        }) => return run_before_commit(std::sync::Arc::clone(early_config), action, changed_only),
        Some(CoreSubcommand::RunBeforePush {
            changed_only,
            action,
        }) => return run_before_push(std::sync::Arc::clone(early_config), action, changed_only),
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
        Some(CoreSubcommand::Plans {
            json_file,
            out,
            json_out,
            keep_plan,
            no_color,
            detailed_exitcode,
            show_outputs,
            passthrough,
        }) => {
            return ops_tfplan::run_plan_pipeline(ops_tfplan::PlanOptions {
                json_file,
                out,
                json_out,
                keep_plan,
                no_color,
                detailed_exitcode,
                show_outputs,
                passthrough,
            });
        }
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
            let cmd = hide_irrelevant_commands(Cli::command(), detected_stack);
            print_categorized_help(cmd, early_config, detected_stack, false);
        }
    }

    Ok(ExitCode::SUCCESS)
}

/// CLI-level cwd lookup. The pre-resolved `Config` is threaded by the caller
/// (TASK-0427) — we only need the current directory at the handler boundary.
pub(crate) fn cwd() -> anyhow::Result<PathBuf> {
    Ok(std::env::current_dir()?)
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
