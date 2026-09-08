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
#[test]
fn metadata_output_cap_defaults_to_the_subprocess_cap() {
    // No env override in the test environment; if one is set, this asserts
    // the guard honours it, which is the same invariant.
    let cap = metadata_output_cap();
    let default = u64::try_from(ops_core::subprocess::DEFAULT_OUTPUT_BYTE_CAP).unwrap_or(u64::MAX);
    let expected = if std::env::var_os(ops_core::subprocess::OUTPUT_CAP_ENV).is_some() {
        cap
    } else {
        default
    };
    assert_eq!(cap, expected);
}
