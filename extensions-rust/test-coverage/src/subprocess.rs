//! Subprocess driver for `cargo llvm-cov`.
//!
//! ARCH-1 / TASK-1559: lifted out of `lib.rs` so the wiring layer stays focused
//! on extension registration. PATTERN-1 / TASK-1560: the cargo exit formatter
//! is centralised here so both the soft-fail path in `parse::collect_coverage`
//! and the hard-fail path in [`check_llvm_cov_output`] surface the same
//! marker for SIGKILL/OOM kills.

use ops_core::output::format_error_tail;
use ops_core::subprocess::{default_timeout, run_cargo_bounded, RunError};
use std::path::Path;
use std::process::{ExitStatus, Output};
use std::time::{Duration, Instant};

/// Ceiling for the `cargo llvm-cov` wait; overridable via
/// `OPS_SUBPROCESS_TIMEOUT_SECS`. Coverage runs the full test suite, so this
/// is the largest of the cargo-subprocess defaults.
///
/// CONC-9 / TASK-2068: this is no longer the whole story — see
/// [`llvm_cov_timeout`], which caps it at whatever is left of the provider
/// dispatch deadline.
pub const CARGO_LLVM_COV_TIMEOUT: Duration = Duration::from_mins(15);

/// TEST-23 / TASK-1554: the argv list `cargo llvm-cov` runs with. Exposed
/// so the regression guard for TASK-1057 (`--no-fail-fast` must remain
/// present; `--json` must remain the last flag) can assert against the
/// authoritative slice instead of grepping the source text via
/// `include_str!`, which rotted silently under rustfmt re-wraps and
/// helper-function moves.
pub const LLVM_COV_ARGS: &[&str] = &[
    "llvm-cov",
    "--workspace",
    "--no-cfg-coverage",
    "--tests",
    "--no-fail-fast",
    "--json",
];

/// Build the full argv for one `cargo llvm-cov` invocation: the static
/// [`LLVM_COV_ARGS`] plus `--output-path` pointing at `output_path`.
///
/// The JSON report goes to a file instead of stdout because the report
/// grows with the workspace (~8 MB here already) and blows past the
/// `OPS_OUTPUT_BYTE_CAP` stdout cap, which silently truncates it into
/// unparseable JSON and kills the entire coverage signal.
pub fn llvm_cov_argv(output_path: &str) -> Vec<&str> {
    let mut args = LLVM_COV_ARGS.to_vec();
    args.extend(["--output-path", output_path]);
    args
}

/// CONC-9 / TASK-2068: how long the `cargo llvm-cov` subprocess may wait,
/// given the provider dispatch deadline currently installed (if any).
///
/// [`ops_extension::DEFAULT_PROVIDER_BUDGET`] used to carry the invariant as
/// prose — the dispatch budget "must stay **above** every subprocess timeout a
/// provider can wait on", with [`CARGO_LLVM_COV_TIMEOUT`] named as the binding
/// one. TASK-2056 made the budget operator-configurable via
/// `[data] provider_budget_secs`, so a `.ops.toml` can now break that ordering:
/// an operator who tightens the budget to 60s got the coverage provider
/// blocking in `cargo llvm-cov` for the full fifteen minutes and only *then*
/// being told it was over budget — a bound that labelled the stall instead of
/// curing it (the shape SEC-33 / TASK-2052 removed from the tree walkers).
///
/// Sizing the wait as `min(ceiling, time remaining on the deadline)` makes the
/// two agree by construction, which is exactly what `Context::deadline`'s own
/// documentation asks callers with an inner timeout knob to do. An expired (or
/// instantly-expiring) deadline yields [`Duration::ZERO`], so the subprocess is
/// reaped immediately rather than started on a budget that is already spent.
#[must_use]
pub fn llvm_cov_timeout(deadline: Option<Instant>) -> Duration {
    let ceiling = default_timeout(CARGO_LLVM_COV_TIMEOUT);
    deadline.map_or(ceiling, |expires_at| {
        ceiling.min(expires_at.saturating_duration_since(Instant::now()))
    })
}

/// Run `cargo llvm-cov` against the workspace, writing the JSON report to
/// `output_path`, and return the captured `Output` (stdout stays small;
/// stderr carries the test-run log).
///
/// CONC-9 / TASK-2068: `deadline` is the provider dispatch deadline
/// (`Context::deadline`). The wait is sized from what is left of it via
/// [`llvm_cov_timeout`], so the subprocess cannot outlive the budget that is
/// supposed to bound it.
///
/// ERR-1 / TASK-1057: pass `--no-fail-fast` (forwarded to cargo test) so a
/// single failing test does not abort the whole suite — without it,
/// `cargo llvm-cov` returns non-zero with empty / partial JSON and the
/// entire coverage signal vanishes from `coverage_files` /
/// `coverage_summary`. With `--no-fail-fast`, every test runs and the
/// per-file coverage data for the passing slice is preserved; the
/// `check_llvm_cov_output` helper then tolerates a non-zero exit when
/// the report file still contains a parseable llvm-cov JSON document.
pub fn run_cargo_llvm_cov(
    working_dir: &Path,
    output_path: &str,
    deadline: Option<Instant>,
) -> Result<Output, RunError> {
    run_cargo_bounded(
        &llvm_cov_argv(output_path),
        working_dir,
        llvm_cov_timeout(deadline),
        "cargo llvm-cov",
    )
}

/// PATTERN-1 / TASK-1560: render an `ExitStatus` as the operator-visible
/// exit marker so the soft-fail (warn) path in [`crate::parse::collect_coverage`]
/// and the hard-fail (bail) path in [`check_llvm_cov_output`] surface the
/// same string for SIGKILL/OOM (`exit_code = None`) and the same string for
/// a regular non-zero exit (`status {code}`). Drifting between
/// `"signal"` and `"exit_code = None"` previously broke grep-on-logs
/// (TASK-1099).
pub fn format_cargo_exit(status: ExitStatus) -> String {
    status.code().map_or_else(
        || "exit_code = None (terminated by signal)".to_string(),
        |code| format!("status {code}"),
    )
}

/// PATTERN-1 / TASK-1099: include the exit code (or `signal` for `None`)
/// in the error so SIGKILL/OOM kills are distinguishable from a real
/// cargo failure.
///
/// ERR-1 / TASK-1057: when the exit is non-zero but the report file named
/// by `--output-path` holds a parseable llvm-cov JSON document,
/// `collect_coverage` treats it as a
/// soft failure (warn + continue) so the per-file coverage for the
/// passing slice of the workspace is preserved. This helper still
/// surfaces the non-zero exit; the caller decides whether to demote it.
pub fn check_llvm_cov_output(output: &Output) -> Result<(), anyhow::Error> {
    if !output.status.success() {
        let tail = format_error_tail(&output.stderr, 5);
        let marker = format_cargo_exit(output.status);
        // Preserve the historical "status 0" / "exit_code = None (terminated
        // by signal)" Display shape so log greps that pre-date TASK-1560
        // keep matching. PATTERN-1 / TASK-1099: keep "cargo llvm-cov" prefix
        // intact so sister assertions still bind.
        let hint = missing_tool_hint(&output.stderr);
        match output.status.code() {
            Some(_) => anyhow::bail!("cargo llvm-cov exited with {marker}: {tail}{hint}"),
            None => anyhow::bail!("cargo llvm-cov terminated by signal ({marker}): {tail}"),
        }
    }
    Ok(())
}

/// Actionable install hint appended to the hard-fail error when cargo
/// reports the `llvm-cov` subcommand is missing. Cargo's own stderr hint
/// (`cargo search …`) neither names the real crate nor the required
/// `llvm-tools-preview` toolchain component, so operators hitting
/// `ops about --refresh` on a machine without cargo-llvm-cov saw the
/// failure with no remediation path.
fn missing_tool_hint(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    if text.contains("no such command: `llvm-cov`") {
        "\nhint: coverage data comes from `cargo llvm-cov`, which is not installed. \
         Install with:\n  \
         cargo install cargo-llvm-cov\n  \
         rustup component add llvm-tools-preview"
            .to_string()
    } else {
        String::new()
    }
}
