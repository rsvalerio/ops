//! Dry-run: resolve and print commands without executing.

use std::io::Write;
use std::process::ExitCode;

use ops_core::config::CommandSpec;
use ops_runner::command::is_sensitive_env_key;

/// SEC-001: Preview commands without executing.
///
/// Prints the resolved command(s) that would be run, including all
/// arguments and environment variables. Used for:
/// - Verifying config changes before running
/// - Auditing what commands are defined
/// - Debugging composite command expansion
pub(crate) fn run_command_dry_run(
    runner: &ops_runner::command::CommandRunner,
    name: &str,
) -> anyhow::Result<ExitCode> {
    run_command_dry_run_to(runner, name, &mut std::io::stdout())
}

pub(crate) fn run_command_dry_run_to(
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
            Some(CommandSpec::Exec(e)) => print_exec_spec(w, e, runner.variables())?,
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

pub(crate) fn print_exec_spec(
    w: &mut dyn Write,
    e: &ops_core::config::ExecCommandSpec,
    vars: &ops_core::expand::Variables,
) -> anyhow::Result<()> {
    writeln!(w, "      program: {}", vars.expand(&e.program))?;
    if let Some(args) = e.expanded_args_display(vars) {
        writeln!(w, "      args:    {}", args)?;
    }
    if !e.env.is_empty() {
        writeln!(w, "      env:")?;
        for (k, v) in &e.env {
            let display_val = if is_sensitive_env_key(k) {
                "***REDACTED***".to_string()
            } else {
                vars.expand(v).into_owned()
            };
            writeln!(w, "        {}={}", k, display_val)?;
        }
    }
    if let Some(cwd) = &e.cwd {
        writeln!(
            w,
            "      cwd:     {}",
            vars.expand(&cwd.display().to_string())
        )?;
    }
    if let Some(timeout) = e.timeout_secs {
        writeln!(w, "      timeout: {}s", timeout)?;
    }
    Ok(())
}
