//! Extension + provider wiring tests.
//!
//! ARCH-1 / TASK-1545: split out from the legacy `tests.rs`.

use crate::{check_metadata_output, MetadataProvider};
use ops_extension::{Context, DataProvider, DataProviderError};

#[test]
fn metadata_provider_name() {
    assert_eq!(MetadataProvider.name(), "metadata");
}

/// Integration test that exercises the live `MetadataProvider::provide` path.
///
/// TEST-26 / TASK-1547: this test runs in every default `cargo test`
/// invocation — it is NOT `#[ignore]`d. It relies on the build environment
/// having `cargo` available on `PATH` (always true when this crate's own
/// tests are running) and on `CARGO_MANIFEST_DIR` pointing at a valid Cargo
/// workspace (cargo sets this for us during test compilation). If either
/// invariant breaks the test fails with a clear cargo-metadata error rather
/// than a generic IO failure, so the failure mode stays actionable.
#[test]
fn metadata_provider_returns_valid_json() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut ctx = Context::test_context(manifest_dir);
    let value = MetadataProvider
        .provide(&mut ctx)
        .expect("cargo metadata should succeed");
    assert!(value.is_object());
    assert!(value.get("packages").is_some());
    assert!(value.get("workspace_root").is_some());
}

/// TEST-11 / TASK-1900: `provide` can fail for at least four unrelated
/// reasons — cargo missing from `PATH`, the subprocess exceeding
/// `CARGO_METADATA_TIMEOUT`, a `DuckDB` failure before cargo ran at all, and
/// the one this test is named for: cargo ran and found no `Cargo.toml`. A
/// bare `is_err()` is green in every one of those environments, so it
/// certified "something went wrong" rather than the behaviour under test.
/// Follow the pattern TASK-1546 established at `ingestor.rs`: match the
/// variant, assert the chain names the cargo-metadata origin *and* the
/// missing manifest, and panic with a distinguishable message on the
/// impostor failures.
#[test]
fn metadata_provider_fails_in_non_cargo_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut ctx = Context::test_context(dir.path().to_path_buf());
    let err = MetadataProvider
        .provide(&mut ctx)
        .expect_err("a directory with no Cargo.toml must fail");
    let chain = format!("{err:#}");
    assert!(
        matches!(err, DataProviderError::ComputationFailed(_)),
        "expected ComputationFailed carrying the cargo-metadata chain, got: {err:?}"
    );
    assert!(
        chain.contains("cargo metadata"),
        "failure must attribute itself to cargo metadata; a missing `cargo` on PATH or a \
         DuckDB failure before cargo ran would not: {chain}"
    );
    assert!(
        chain.contains("Cargo.toml"),
        "failure must be the missing-manifest one, not a timeout or a spawn error: {chain}"
    );
}

#[test]
fn metadata_schema_has_expected_fields() {
    let schema = MetadataProvider.schema();
    assert!(!schema.fields.is_empty());
    let field_names: Vec<&str> = schema.fields.iter().map(|f| f.name).collect();
    assert!(field_names.contains(&"workspace_root"));
    assert!(field_names.contains(&"packages"));
    assert!(field_names.contains(&"members"));
}

/// TEST-1 / TASK-1901: `#[cfg(unix)]` sits on the `fn`, not on the
/// assertion. With it on the assertion the test still existed off-unix but
/// compiled to setup with no assertion — a test that cannot fail. Its two
/// siblings below already had the correct shape.
#[cfg(unix)]
#[test]
fn check_metadata_output_success() {
    use std::process::Output;
    // ExitStatus::default() is success (code 0) on unix.
    let output = Output {
        status: std::process::ExitStatus::default(),
        stdout: vec![],
        stderr: vec![],
    };
    assert!(check_metadata_output(&output).is_ok());
}

/// PATTERN-1 / TASK-1099: non-zero exit codes must appear in the
/// error string so a real cargo failure (exit 1, exit 101 panic) is
/// distinguishable from infrastructure (SIGKILL/OOM, surfaced as
/// `signal`).
#[cfg(unix)]
#[test]
fn check_metadata_output_failure_includes_exit_code() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Output;
    // exit code 101 (panic-style)
    let output = Output {
        status: std::process::ExitStatus::from_raw(101 << 8),
        stdout: vec![],
        stderr: b"thread 'main' panicked".to_vec(),
    };
    let err = check_metadata_output(&output).expect_err("non-zero must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("status 101"),
        "exit code 101 must appear in error: {msg}"
    );
    assert!(
        msg.contains("panicked"),
        "stderr tail must remain in error: {msg}"
    );
}

/// PATTERN-1 / TASK-1099: a None exit (signal kill, e.g. OOM)
/// surfaces as `signal` rather than the same string as a normal
/// non-zero exit.
#[cfg(unix)]
#[test]
fn check_metadata_output_failure_signal_kill_says_signal() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Output;
    // signal 9 (SIGKILL) → exit_code() returns None
    let output = Output {
        status: std::process::ExitStatus::from_raw(9),
        stdout: vec![],
        stderr: b"".to_vec(),
    };
    let err = check_metadata_output(&output).expect_err("signal must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("signal") || msg.contains("None"),
        "signal-kill case must be named in error: {msg}"
    );
}

/// PATTERN-1 / TASK-1059: `cargo metadata` must run with `--locked` so the
/// read-only ingestor cannot mutate Cargo.lock.
///
/// TEST-25 / TASK-1899: assert on [`crate::CARGO_METADATA_ARGS`] — the array
/// `run_cargo_metadata` actually hands to `run_cargo` — rather than on
/// `include_str!` of the source file. Reformatting `lib.rs` can no longer
/// fail this test. Bypassing the constant cannot pass silently either:
/// `run_cargo_metadata` is its only user, so a call site that stops reading
/// it makes the `pub(crate) const` dead and the workspace's
/// `-D warnings` gate rejects the build.
#[test]
fn run_cargo_metadata_arg_list_includes_locked() {
    assert_eq!(
        crate::CARGO_METADATA_ARGS,
        ["metadata", "--format-version", "1", "--locked"],
        "cargo metadata must run with --locked (TASK-1059)"
    );
}
