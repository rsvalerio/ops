//! Extension + provider wiring tests.

use crate::{check_metadata_output, MetadataProvider};
use ops_extension::{Context, DataProvider, DataProviderError};

#[test]
fn metadata_provider_name() {
    assert_eq!(MetadataProvider.name(), "metadata");
}

/// Integration test that exercises the live `MetadataProvider::provide` path.
///
/// This test runs in every default `cargo test`
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

/// `provide` can fail for at least four unrelated reasons — cargo missing
/// from `PATH`, the subprocess exceeding `CARGO_METADATA_TIMEOUT`, a
/// database failure before cargo ran at all, and the one this test is named
/// for: cargo ran and found no `Cargo.toml`. A bare `is_err()` is green in
/// every one of those, so the test matches the variant and asserts the
/// chain names the cargo-metadata origin *and* the missing manifest.
#[test]
fn metadata_provider_fails_in_non_cargo_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut ctx = Context::test_context(dir.path().to_path_buf());
    let err = MetadataProvider
        .provide(&mut ctx)
        .expect_err("a directory with no Cargo.toml must fail");
    let chain = format!("{err:#}");
    assert!(
        matches!(&err, DataProviderError::ComputationFailed(_)),
        "expected ComputationFailed carrying the cargo-metadata chain, got: {err:?}"
    );
    assert!(
        chain.contains("cargo metadata"),
        "failure must attribute itself to cargo metadata; a missing `cargo` on PATH or a \
         database failure before cargo ran would not: {chain}"
    );
    assert!(
        chain.contains("Cargo.toml"),
        "failure must be the missing-manifest one, not a timeout or a spawn error: {chain}"
    );
}

/// Whether `path` (the schema's notation: `.` descends into an object, `[]`
/// into every element of an array) names a key present in `value`. An array
/// segment is satisfied by any one element.
fn path_resolves(value: &serde_json::Value, path: &str) -> bool {
    let (head, rest) = path.split_once('.').unwrap_or((path, ""));
    let (key, is_array) = head
        .strip_suffix("[]")
        .map_or((head, false), |key| (key, true));
    let Some(child) = value.get(key) else {
        return false;
    };
    match (is_array, rest.is_empty()) {
        (false, true) => true,
        (false, false) => path_resolves(child, rest),
        (true, true) => child.is_array(),
        (true, false) => child
            .as_array()
            .is_some_and(|items| items.iter().any(|item| path_resolves(item, rest))),
    }
}

#[test]
fn path_resolves_rejects_keys_the_document_lacks() {
    let doc = serde_json::json!({"packages": [{"name": "a", "dependencies": []}]});
    assert!(path_resolves(&doc, "packages"));
    assert!(path_resolves(&doc, "packages[].name"));
    assert!(!path_resolves(&doc, "members"));
    assert!(!path_resolves(&doc, "packages[].dependencies[].req"));
    assert!(!path_resolves(&doc, "packages.name"));
}

/// Every schema field must be a key of the document `provide` returns, so the
/// schema cannot advertise a field consumers will never find.
#[test]
fn every_schema_field_is_a_key_of_the_provided_document() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut ctx = Context::test_context(manifest_dir);
    let document = MetadataProvider
        .provide(&mut ctx)
        .expect("cargo metadata should succeed");
    let schema = MetadataProvider.schema();
    let absent: Vec<&str> = schema
        .fields
        .iter()
        .map(|f| f.name)
        .filter(|name| !path_resolves(&document, name))
        .collect();
    assert!(
        absent.is_empty(),
        "schema fields absent from the cargo metadata document: {absent:?}"
    );
    let names: Vec<&str> = schema.fields.iter().map(|f| f.name).collect();
    for expected in [
        "workspace_root",
        "workspace_members",
        "workspace_default_members",
        "packages[].dependencies[].req",
    ] {
        assert!(names.contains(&expected), "schema lacks {expected}");
    }
}

/// `#[cfg(unix)]` sits on the `fn`, not on the assertion: on the assertion
/// the test would still exist off-unix but compile to setup with no
/// assertion — a test that cannot fail.
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

/// Non-zero exit codes must appear in the
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

/// A None exit (signal kill, e.g. OOM)
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

/// `cargo metadata` must run with `--locked` so the read-only ingestor
/// cannot mutate Cargo.lock.
///
/// Asserts on [`crate::CARGO_METADATA_ARGS`] — the array
/// `run_cargo_metadata` actually hands to `run_cargo` — rather than on
/// `include_str!` of the source file, so reformatting `lib.rs` cannot fail
/// this test. Bypassing the constant cannot pass silently either:
/// `run_cargo_metadata` is its only user, so a call site that stops reading
/// it makes the `pub(crate) const` dead and the workspace's
/// `-D warnings` gate rejects the build.
#[test]
fn run_cargo_metadata_arg_list_includes_locked() {
    assert_eq!(
        crate::CARGO_METADATA_ARGS,
        ["metadata", "--format-version", "1", "--locked"],
        "cargo metadata must run with --locked"
    );
}
