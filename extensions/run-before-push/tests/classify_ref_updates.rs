//! Public-API tests for the pre-push ref-update classifier.
//!
//! These exercise only the crate's public surface
//! ([`classify_ref_updates`], [`push_refs`], [`skip_reason`],
//! [`should_skip`]) and therefore live outside `src/` per the
//! integration-test convention; the hook-script and config-pinning tests
//! that need private constants stay in `#[cfg(test)]` modules in the
//! crate.

use ops_hook_common::test_helpers::EnvGuard;
use ops_run_before_push::{
    classify_ref_updates, push_refs, should_skip, skip_reason, PushRefs, REFS_TRUNCATED_ENV_VAR,
    REF_UPDATES_ENV_VAR, SKIP_ENV_VAR,
};

const SHA1_A: &str = "1111111111111111111111111111111111111111";
const SHA1_B: &str = "2222222222222222222222222222222222222222";
const ZERO: &str = "0000000000000000000000000000000000000000";

// -- classify_ref_updates --

#[test]
fn classify_runs_when_stream_is_absent() {
    assert_eq!(classify_ref_updates(None), PushRefs::Run);
}

#[test]
fn classify_reports_nothing_to_push_for_empty_stream() {
    assert_eq!(classify_ref_updates(Some("")), PushRefs::NothingToPush);
    assert_eq!(
        classify_ref_updates(Some("\n  \n")),
        PushRefs::NothingToPush
    );
}

#[test]
fn classify_runs_for_a_normal_ref_update() {
    let line = format!("refs/heads/main {SHA1_A} refs/heads/main {SHA1_B}\n");
    assert_eq!(classify_ref_updates(Some(&line)), PushRefs::Run);
}

#[test]
fn classify_reports_delete_only_when_every_local_oid_is_zero() {
    let stream =
        format!("(delete) {ZERO} refs/heads/old {SHA1_A}\n(delete) {ZERO} refs/tags/v1 {SHA1_B}\n");
    assert_eq!(classify_ref_updates(Some(&stream)), PushRefs::DeleteOnly);
}

#[test]
fn classify_runs_when_a_delete_is_mixed_with_an_update() {
    let stream = format!(
        "(delete) {ZERO} refs/heads/old {SHA1_A}\nrefs/heads/main {SHA1_A} refs/heads/main {ZERO}\n"
    );
    assert_eq!(classify_ref_updates(Some(&stream)), PushRefs::Run);
}

#[test]
fn classify_runs_for_malformed_lines() {
    // too few fields
    assert_eq!(
        classify_ref_updates(Some(&format!("refs/heads/main {SHA1_A} refs/heads/main\n"))),
        PushRefs::Run
    );
    // too many fields
    assert_eq!(
        classify_ref_updates(Some(&format!(
            "refs/heads/main {SHA1_A} refs/heads/main {SHA1_B} extra\n"
        ))),
        PushRefs::Run
    );
    // oid that is not hex of the right length
    assert_eq!(
        classify_ref_updates(Some(&format!(
            "refs/heads/main zzzz refs/heads/main {SHA1_B}\n"
        ))),
        PushRefs::Run
    );
    // a malformed line must not be masked by a well-formed deletion
    assert_eq!(
        classify_ref_updates(Some(&format!(
            "(delete) {ZERO} refs/heads/old {SHA1_A}\ngarbage\n"
        ))),
        PushRefs::Run
    );
}

#[test]
fn skip_reason_is_none_only_for_run() {
    assert_eq!(PushRefs::Run.skip_reason(), None);
    assert_eq!(PushRefs::DeleteOnly.skip_reason(), Some("delete-only push"));
    assert_eq!(
        PushRefs::NothingToPush.skip_reason(),
        Some("nothing to push")
    );
}

#[test]
#[serial_test::serial]
fn skip_reason_reads_the_forwarded_env_var() {
    {
        let _guard = EnvGuard::remove(REF_UPDATES_ENV_VAR);
        assert_eq!(push_refs(), PushRefs::Run);
        assert_eq!(skip_reason(), None);
    }
    {
        let _guard = EnvGuard::set(
            REF_UPDATES_ENV_VAR,
            format!("(delete) {ZERO} refs/heads/old {SHA1_A}\n"),
        );
        assert_eq!(push_refs(), PushRefs::DeleteOnly);
        assert_eq!(skip_reason(), Some("delete-only push"));
    }
    {
        let _guard = EnvGuard::set(REF_UPDATES_ENV_VAR, "");
        assert_eq!(push_refs(), PushRefs::NothingToPush);
        assert_eq!(skip_reason(), Some("nothing to push"));
    }
}

/// The truncation marker short-circuits to `Run` before classification —
/// a truncated stream is a prefix, and a prefix of a delete-heavy
/// mirror push could otherwise read as delete-only and skip the checks.
#[test]
#[serial_test::serial]
fn push_refs_runs_when_the_truncation_marker_is_set() {
    let _refs = EnvGuard::set(
        REF_UPDATES_ENV_VAR,
        format!("(delete) {ZERO} refs/heads/old {SHA1_A}\n"),
    );
    assert_eq!(
        {
            let _marker = EnvGuard::remove(REFS_TRUNCATED_ENV_VAR);
            push_refs()
        },
        PushRefs::DeleteOnly,
        "without the marker the stream classifies normally"
    );
    {
        let _marker = EnvGuard::set(REFS_TRUNCATED_ENV_VAR, "1");
        assert_eq!(push_refs(), PushRefs::Run, "marker \"1\" must run");
        assert_eq!(skip_reason(), None, "the checks must not be skipped");
    }
    // Only the literal `1` counts; the hook exports an empty value when
    // it did not truncate, which must not read as truncated.
    let _marker = EnvGuard::set(REFS_TRUNCATED_ENV_VAR, "");
    assert_eq!(
        push_refs(),
        PushRefs::DeleteOnly,
        "an empty marker must not force Run"
    );
}

// -- should_skip --

#[test]
#[serial_test::serial]
fn should_skip_returns_false_by_default() {
    let _guard = EnvGuard::remove(SKIP_ENV_VAR);
    assert!(!should_skip());
}
