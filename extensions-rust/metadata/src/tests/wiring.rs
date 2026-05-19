//! Extension + provider wiring tests.
//!
//! ARCH-1 / TASK-1545: split out from the legacy `tests.rs`.

use crate::{check_metadata_output, MetadataProvider};
use ops_extension::{Context, DataProvider};

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

#[test]
fn metadata_provider_fails_in_non_cargo_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut ctx = Context::test_context(dir.path().to_path_buf());
    let result = MetadataProvider.provide(&mut ctx);
    assert!(result.is_err());
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

#[test]
fn check_metadata_output_success() {
    use std::process::Output;
    let output = Output {
        status: std::process::ExitStatus::default(),
        stdout: vec![],
        stderr: vec![],
    };
    // ExitStatus::default() is success (code 0) on unix
    #[cfg(unix)]
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

/// PATTERN-1 / TASK-1059: `cargo metadata` must run with `--locked`
/// so the read-only ingestor cannot mutate Cargo.lock. The actual
/// subprocess invocation goes through `run_cargo`; pin the arg list
/// at the source-of-truth here so a future refactor cannot silently
/// drop the flag.
#[test]
fn run_cargo_metadata_arg_list_includes_locked() {
    // Read the current source of `run_cargo_metadata` and verify
    // the static arg list includes `--locked`. This is a coarse
    // pin but it withstands moving the function body around without
    // requiring a fake `cargo` on PATH.
    let src = include_str!("../lib.rs");
    let needle = "[\"metadata\", \"--format-version\", \"1\", \"--locked\"]";
    assert!(
        src.contains(needle),
        "run_cargo_metadata arg list must include --locked (TASK-1059); src missing: {needle}"
    );
}
