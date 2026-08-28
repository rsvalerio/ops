//! Tests for the deps extension's crate-root surface: extension and data
//! provider registration, the `has_issues` gate, the reported schema, and
//! the cargo-tool probe.
//!
//! Parser, formatter, and type tests live beside the modules they cover:
//! `parse/deny/tests.rs`, `parse/upgrade/`, `format/`, `types/tests.rs`.

use super::*;

// -- Extension trait tests --

mod extension_tests {
    use super::*;

    ops_extension::test_datasource_extension!(
        DepsExtension,
        name: "deps",
        data_provider: "deps"
    );
}

// -- ERR-4 / TASK-0405: user config reaches DepsProvider context --

mod user_config_tests {
    use super::*;
    use serial_test::serial;

    /// `build_user_context` must read the user's `.ops.toml` rather than
    /// falling back to `Config::empty()`. We chdir to a tempdir that
    /// contains a config file with a recognizable `stack` and confirm the
    /// resulting Context carries that value through to the data provider
    /// boundary (i.e. `ctx.config().stack`).
    #[test]
    #[serial]
    fn build_user_context_loads_stack_from_local_ops_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".ops.toml"), "stack = \"rust\"\n")
            .expect("write .ops.toml");
        let prev = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir.path()).expect("chdir");

        let ctx = build_user_context().expect("build_user_context");

        std::env::set_current_dir(&prev).expect("restore cwd");

        assert_eq!(
            ctx.config().stack.as_deref(),
            Some("rust"),
            "Context.config must carry stack from the loaded user config"
        );
    }
}

// -- has_issues tests --

/// DUP-3 / TASK-0989: a `warning` severity must be actionable on the
/// strict gate (advisories / licenses / sources) and non-actionable on
/// the relaxed gate (bans). Both branches now route through the same
/// `is_actionable(severity, relax_warning)` helper instead of two
/// near-identical match arms, so this test pins the contract by
/// exercising both modes via the same `DepsReport` shape.
#[test]
fn has_issues_warning_is_actionable_only_on_strict_gate() {
    // Strict gate (advisories): warning => actionable.
    let strict = DepsReport {
        deny: DenyResult {
            advisories: vec![AdvisoryEntry {
                id: "X".into(),
                package: "a".into(),
                severity: "warning".into(),
                title: "t".into(),
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(
        has_issues(&strict),
        "strict gate: warning must be actionable"
    );

    // Relaxed gate (bans): warning => not actionable.
    let relaxed = DepsReport {
        deny: DenyResult {
            bans: vec![BanEntry(DenyEntry {
                package: "a".into(),
                message: "duplicate".into(),
                severity: "warning".into(),
            })],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(
        !has_issues(&relaxed),
        "relaxed gate (bans): warning must be informational"
    );
}

#[test]
fn has_issues_clean_report() {
    let report = DepsReport::default();
    assert!(!has_issues(&report));
}

#[test]
fn has_issues_advisory_error() {
    let report = DepsReport {
        deny: DenyResult {
            advisories: vec![AdvisoryEntry {
                id: "RUSTSEC-2024-0001".into(),
                package: "foo".into(),
                severity: "error".into(),
                title: "bad".into(),
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(has_issues(&report));
}

#[test]
fn has_issues_advisory_warning() {
    let report = DepsReport {
        deny: DenyResult {
            advisories: vec![AdvisoryEntry {
                id: "RUSTSEC-2024-0002".into(),
                package: "bar".into(),
                severity: "warning".into(),
                title: "meh".into(),
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(has_issues(&report));
}

#[test]
fn has_issues_advisory_info_not_actionable() {
    let report = DepsReport {
        deny: DenyResult {
            advisories: vec![AdvisoryEntry {
                id: "RUSTSEC-2024-0003".into(),
                package: "baz".into(),
                severity: "info".into(),
                title: "fyi".into(),
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(!has_issues(&report));
}

/// ERR-2 (TASK-0601): an unknown severity (e.g. cargo-deny adding a new
/// `critical` severity in a future release) must fail the gate, not slip
/// through silently. Combined with the missing-severity `error` default in
/// `parse_deny_output`, this guarantees schema drift either surfaces or
/// errs on the side of failing CI.
#[test]
fn has_issues_unknown_severity_fails_closed() {
    let report = DepsReport {
        deny: DenyResult {
            advisories: vec![AdvisoryEntry {
                id: "RUSTSEC-2024-0099".into(),
                package: "x".into(),
                severity: "critical".into(),
                title: "future severity".into(),
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(
        has_issues(&report),
        "unknown severities must be treated as actionable"
    );
}

#[test]
fn has_issues_license_error() {
    let report = DepsReport {
        deny: DenyResult {
            licenses: vec![LicenseEntry(DenyEntry {
                package: "evil".into(),
                message: "rejected".into(),
                severity: "error".into(),
            })],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(has_issues(&report));
}

#[test]
fn has_issues_license_warning() {
    let report = DepsReport {
        deny: DenyResult {
            licenses: vec![LicenseEntry(DenyEntry {
                package: "sketchy".into(),
                message: "unclear".into(),
                severity: "warning".into(),
            })],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(has_issues(&report));
}

#[test]
fn has_issues_ban_error() {
    let report = DepsReport {
        deny: DenyResult {
            bans: vec![BanEntry(DenyEntry {
                package: "banned".into(),
                message: "not allowed".into(),
                severity: "error".into(),
            })],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(has_issues(&report));
}

#[test]
fn has_issues_ban_warning_not_actionable() {
    let report = DepsReport {
        deny: DenyResult {
            bans: vec![BanEntry(DenyEntry {
                package: "dup".into(),
                message: "duplicate".into(),
                severity: "warning".into(),
            })],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(!has_issues(&report));
}

/// TASK-0701: a ban with an unknown severity (e.g. 'critical') must be
/// treated as actionable by `is_actionable`, not silently ignored as the
/// old hardcoded `== "error"` check did.
#[test]
fn has_issues_ban_critical_severity_fails_closed() {
    let report = DepsReport {
        deny: DenyResult {
            bans: vec![BanEntry(DenyEntry {
                package: "dangerous".into(),
                message: "critical ban".into(),
                severity: "critical".into(),
            })],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(
        has_issues(&report),
        "unknown ban severities must be treated as actionable"
    );
}

#[test]
fn has_issues_source_error() {
    let report = DepsReport {
        deny: DenyResult {
            sources: vec![SourceEntry(DenyEntry {
                package: "untrusted".into(),
                message: "bad source".into(),
                severity: "error".into(),
            })],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(has_issues(&report));
}

#[test]
fn has_issues_source_warning() {
    let report = DepsReport {
        deny: DenyResult {
            sources: vec![SourceEntry(DenyEntry {
                package: "sketchy".into(),
                message: "underspecified".into(),
                severity: "warning".into(),
            })],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(has_issues(&report));
}

// -- Schema tests --

#[test]
fn schema_has_expected_fields() {
    use ops_extension::DataProvider;
    let schema = DepsProvider.schema();
    assert_eq!(schema.fields.len(), 6);
    let field_names: Vec<&str> = schema.fields.iter().map(|f| f.name).collect();
    assert!(field_names.contains(&"upgrades.compatible"));
    assert!(field_names.contains(&"upgrades.incompatible"));
    assert!(field_names.contains(&"deny.advisories"));
    assert!(field_names.contains(&"deny.licenses"));
    assert!(field_names.contains(&"deny.bans"));
    assert!(field_names.contains(&"deny.sources"));
}

/// ASYNC-6 (TASK-0791): `check_tool_in` must surface a clear timeout error
/// when the cargo probe hangs, rather than blocking the process indefinitely.
/// Drive the timeout path via `OPS_SUBPROCESS_TIMEOUT_SECS=1` plus a fake
/// `$CARGO` that sleeps far past the deadline.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn check_tool_in_times_out_on_hung_probe() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("tempdir");
    let fake = dir.path().join("cargo");
    // `exec` so SIGKILL on the script reaches the underlying sleep process;
    // otherwise sleep keeps the inherited stdout/stderr pipes open and the
    // drain threads block until sleep finishes naturally, masking the
    // timeout firing.
    std::fs::write(&fake, "#!/bin/sh\nexec sleep 30\n").unwrap();
    let mut perms = std::fs::metadata(&fake).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&fake, perms).unwrap();

    let tool = CargoTool {
        subcommand: "probe-test",
        install_crate: "cargo-probe-test",
        probe_args: &["probe-test", "--version"],
    };

    // SAFETY: serial guards against concurrent env mutation; the helper reads
    // these vars synchronously on this thread.
    unsafe { std::env::set_var("CARGO", &fake) };
    unsafe { std::env::set_var(ops_core::subprocess::TIMEOUT_ENV, "1") };
    let result = check_tool_in(&tool, dir.path());
    unsafe { std::env::remove_var(ops_core::subprocess::TIMEOUT_ENV) };
    unsafe { std::env::remove_var("CARGO") };

    let err = result.expect_err("hung probe must error rather than block");
    let msg = format!("{err}");
    assert!(
        msg.contains("timed out") || msg.contains("wedged"),
        "expected timeout-shaped error, got: {msg}"
    );
}

// -- ERR-4 / TASK-1523: error source chain preserved through .context() --

#[test]
fn provide_error_preserves_source_chain() {
    use ops_extension::DataProviderError;

    let root = std::io::Error::new(std::io::ErrorKind::NotFound, "cargo not found");
    let wrapped: anyhow::Error = anyhow::Error::from(root);
    let with_context: anyhow::Error = wrapped.context("cargo upgrade failed");
    let provider_err = DataProviderError::from(with_context);

    let err: &dyn std::error::Error = &provider_err;
    assert!(
        err.source().is_some(),
        "DataProviderError must preserve the error source chain"
    );
}
