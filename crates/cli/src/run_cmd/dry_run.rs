//! Dry-run: resolve and print commands without executing.

use std::io::Write;
use std::process::ExitCode;

use ops_core::config::CommandSpec;
use ops_runner::command::{is_sensitive_env_key, looks_like_secret_value_public};

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
    let leaf_ids = runner.expand_to_leaves(name).map_err(anyhow::Error::from)?;

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
            // SEC-21: previously only redacted when the *key* matched the
            // narrow `SENSITIVE_REDACTION_PATTERNS` allowlist. Any env var
            // with a non-matching name (DATABASE_URL, GITHUB_PAT,
            // SLACK_WEBHOOK, …) printed in cleartext to stdout — which a
            // user then copy-pasted into a bug report. Mirror the
            // broader `warn_if_sensitive_env` policy: redact when the key
            // looks sensitive *or* the (expanded) value itself looks like
            // a secret per the JWT/UUID/high-entropy heuristics.
            //
            // SEC-21 known false-negatives — values that *will* leak through
            // when the key name does not match `is_sensitive_env_key`:
            //   - Short bearer tokens (<20 chars), e.g. shortened API keys.
            //   - Lowercase-hex-only strings (git SHAs, MD5/SHA1 hashes used
            //     as deploy tokens) — see SEC-11 in `secret_patterns`.
            //   - Connection strings whose host/path is the only secret
            //     (e.g. `https://hooks.slack.com/services/T.../B.../X...`).
            //   - Base32 / non-base64 encodings (TOTP seeds).
            // Defense in depth: name sensitive env vars with one of the
            // standard prefixes (TOKEN, SECRET, PASSWORD, KEY, AUTH, …) so
            // key-based redaction kicks in even if the value heuristic misses.
            let expanded = vars.expand(v);
            let display_val =
                if is_sensitive_env_key(k) || looks_like_secret_value_public(&expanded) {
                    "***REDACTED***".to_string()
                } else {
                    expanded.into_owned()
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
