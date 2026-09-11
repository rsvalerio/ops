//! ERR-1 / TASK-2188: the subprocess capture cap must not be mistaken for a
//! complete document.
//!
//! `run_with_timeout`'s drain threads bound each stream at
//! `OPS_OUTPUT_BYTE_CAP` (4 MiB default) and treat truncation as a warn-level
//! breadcrumb, not an error. Both consumers of `cargo metadata` stdout — the
//! ingest path (`MetadataIngestor::collect`, which stages `metadata.json`)
//! and the fallback path (`provide_via_cargo_metadata`, which serde-parses
//! the buffer) — used to treat the capped buffer as authoritative. These
//! tests pin the guard that refuses it.

use super::*;

/// AC#1+#4: a stdout at the capture cap is refused, with the cap value and
/// its env var named — not parsed into a truncated document whose failure
/// would otherwise surface as a misattributed serde error.
///
/// The allocation matches the resolved cap; with the default 4 MiB cap this
/// is one 4 MiB `Vec` plus a one-byte-shorter clone. A machine that raises
/// `OPS_OUTPUT_BYTE_CAP` far above the default makes this test allocate
/// accordingly — acceptable for a test process, and the only way to drive
/// the real resolved value rather than a reimplementation of it.
#[cfg(unix)]
#[test]
fn capped_cargo_metadata_stdout_is_refused_with_the_cap_named() {
    use std::os::unix::process::ExitStatusExt as _;

    let cap = metadata_output_cap();
    let output = Output {
        status: std::process::ExitStatus::from_raw(0),
        stdout: vec![b'x'; usize::try_from(cap).expect("cap fits usize")],
        stderr: Vec::new(),
    };

    let err = check_metadata_not_capped(&output)
        .expect_err("a capped stdout must be refused, not parsed");
    let msg = err.to_string();
    assert!(
        msg.contains(&cap.to_string()),
        "error must name the cap value; got: {msg}"
    );
    assert!(
        msg.contains(ops_core::subprocess::OUTPUT_CAP_ENV),
        "error must name the env var; got: {msg}"
    );
    assert!(
        msg.contains("truncated"),
        "error must say the document may be truncated; got: {msg}"
    );

    // One byte under the cap is a complete document and must pass.
    let mut under = output;
    under.stdout.pop();
    check_metadata_not_capped(&under).expect("just under the cap must pass");
}

/// AC#3: the metadata crate resolves the same cap the subprocess layer
/// enforces — same env var, same default — so the guard's comparison basis
/// cannot drift from the drain's.
///
/// The override half of the invariant cannot be asserted in this process
/// without comparing the resolver to itself: the cap is memoised in a
/// `OnceLock` initialised before any test can act, and setting the env var
/// from a sibling test would race every other reader. It is therefore
/// pinned in an isolated child process by
/// [`metadata_output_cap_honours_a_fixed_env_override`]; this test covers
/// the default-value half only.
#[test]
fn metadata_output_cap_defaults_to_the_subprocess_cap() {
    if std::env::var_os(ops_core::subprocess::OUTPUT_CAP_ENV).is_some() {
        // An override is present, so the default is not what the resolver
        // returns; the override path itself is covered by the re-exec test
        // below. Asserting here would compare the memoised value with
        // itself.
        return;
    }
    let cap = metadata_output_cap();
    let default = u64::try_from(ops_core::subprocess::DEFAULT_OUTPUT_BYTE_CAP).unwrap_or(u64::MAX);
    assert_eq!(cap, default);
}

/// The resolver must honour a fixed `OPS_OUTPUT_BYTE_CAP` set before its
/// first call. Driven in a re-exec'd child test process so the override is
/// present before the process-global `OnceLock` initialises and no sibling
/// test's env mutation races it.
#[test]
fn metadata_output_cap_honours_a_fixed_env_override() {
    let exe = std::env::current_exe().expect("test binary path");
    let out = std::process::Command::new(exe)
        .args([
            "--exact",
            "--ignored",
            "tests::output_cap::metadata_output_cap_resolves_the_env_override_child",
        ])
        .env(ops_core::subprocess::OUTPUT_CAP_ENV, "1234567")
        .output()
        .expect("re-exec the test binary with the override set");
    assert!(
        out.status.success(),
        "child override test failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Child half of [`metadata_output_cap_honours_a_fixed_env_override`]:
/// runs only when the parent re-execs this binary with
/// `OPS_OUTPUT_BYTE_CAP=1234567` in the environment, so the resolver's
/// first (and only) call sees the fixed override.
#[test]
#[ignore = "driven by metadata_output_cap_honours_a_fixed_env_override via re-exec"]
fn metadata_output_cap_resolves_the_env_override_child() {
    assert_eq!(metadata_output_cap(), 1_234_567);
}
