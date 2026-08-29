//! Tests for the deps extension's crate-root surface: extension and data
//! provider registration, the `has_issues` gate, the reported schema, and
//! the cargo-tool probe.
//!
//! Parser, formatter, and type tests live beside the modules they cover:
//! `parse/deny/tests.rs`, `parse/upgrade/`, `format/`, `types/tests.rs`.

use super::*;

// -- TEST-5 / TASK-1845, ERR-4 / TASK-1827: the `ops deps` command path --

/// Drives `run_deps`, `ensure_tools` and `DepsProvider::provide` end to end
/// without a real `cargo-edit` / `cargo-deny` installation and without
/// touching the network.
///
/// The seam is the one `check_tool_in_times_out_on_hung_probe` already uses:
/// `ops_core::subprocess::run_cargo` resolves the binary through `$CARGO`, so
/// a shell script on disk stands in for cargo and decides what every probe
/// and every collection call returns. The other seam is `run_deps`' own
/// `DataRegistry` parameter, which exists so a stub provider can be
/// registered under `DATA_PROVIDER_NAME`.
mod command_path_tests {
    use super::*;
    use crate::test_support::{CwdGuard, EnvVarGuard};
    use ops_extension::{DataProviderSchema, DataRegistry};
    use serial_test::serial;

    /// A `$CARGO` stand-in. `body` is the shell script body; `exit 0` makes
    /// every `cargo <sub> --version` probe report the tool as installed.
    #[cfg(unix)]
    fn fake_cargo(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt as _;
        let path = dir.join("fake-cargo");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write fake cargo");
        let mut perms = std::fs::metadata(&path).expect("metadata").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).expect("chmod");
        path
    }

    /// A provider that yields a caller-supplied JSON payload under the
    /// `deps` key, and records whether the context it was handed was in
    /// refresh mode.
    struct StubProvider {
        payload: serde_json::Value,
        saw_refresh: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    impl DataProvider for StubProvider {
        fn name(&self) -> &'static str {
            DATA_PROVIDER_NAME
        }

        fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
            self.saw_refresh
                .store(ctx.is_refreshing(), std::sync::atomic::Ordering::SeqCst);
            Ok(self.payload.clone())
        }

        fn schema(&self) -> DataProviderSchema {
            DataProviderSchema::new("stub", vec![])
        }
    }

    /// Registry carrying a stub `deps` provider plus the flag it sets.
    fn stub_registry(
        payload: serde_json::Value,
    ) -> (DataRegistry, std::sync::Arc<std::sync::atomic::AtomicBool>) {
        let saw_refresh = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut registry = DataRegistry::new();
        let _ = registry.register(
            DATA_PROVIDER_NAME,
            Box::new(StubProvider {
                payload,
                saw_refresh: std::sync::Arc::clone(&saw_refresh),
            }),
        );
        (registry, saw_refresh)
    }

    fn clean_report_json() -> serde_json::Value {
        serde_json::to_value(DepsReport::default()).expect("serialize default report")
    }

    fn advisory_report_json() -> serde_json::Value {
        serde_json::to_value(DepsReport {
            deny: DenyResult {
                advisories: vec![AdvisoryEntry {
                    id: "RUSTSEC-2024-0001".into(),
                    package: "openssl".into(),
                    severity: "error".into(),
                    title: "buffer overflow".into(),
                }],
                ..Default::default()
            },
            ..Default::default()
        })
        .expect("serialize advisory report")
    }

    /// TEST-5 / TASK-1845 AC#1: the happy path returns `Ok` — the command
    /// wiring (config load → theme resolve → provider → decode → render)
    /// holds together.
    #[cfg(unix)]
    #[test]
    #[serial]
    fn run_deps_returns_ok_for_a_clean_report() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _cargo = EnvVarGuard::set("CARGO", fake_cargo(dir.path(), "exit 0"));
        let _cwd = CwdGuard::set(dir.path());

        let (registry, _) = stub_registry(clean_report_json());
        run_deps(&registry, &DepsOptions::new(false)).expect("clean report must return Ok");
    }

    /// TEST-5 / TASK-1845 AC#2: the product's actual contract — `ops deps`
    /// fails CI when there are dependency issues. `has_issues` → `bail!` is
    /// the only place "fail loudly" becomes a non-zero exit, and it was the
    /// one place with no test: a refactor that rendered the report and
    /// returned `Ok(())` regardless passed the entire suite.
    #[cfg(unix)]
    #[test]
    #[serial]
    fn run_deps_errs_when_the_report_carries_an_actionable_advisory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _cargo = EnvVarGuard::set("CARGO", fake_cargo(dir.path(), "exit 0"));
        let _cwd = CwdGuard::set(dir.path());

        let (registry, _) = stub_registry(advisory_report_json());
        let err = run_deps(&registry, &DepsOptions::new(false))
            .expect_err("an actionable advisory must fail the gate");
        assert!(
            err.to_string().contains("dependency issues"),
            "expected the gate's bail, got: {err}"
        );
    }

    /// TEST-5 / TASK-1845 AC#3: `opts.refresh` must reach `ctx.refresh` and
    /// therefore the provider — otherwise `ops deps --refresh` silently
    /// serves the cached payload it was asked to discard.
    #[cfg(unix)]
    #[test]
    #[serial]
    fn run_deps_propagates_refresh_to_the_provider_context() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _cargo = EnvVarGuard::set("CARGO", fake_cargo(dir.path(), "exit 0"));
        let _cwd = CwdGuard::set(dir.path());

        let (registry, saw_refresh) = stub_registry(clean_report_json());
        run_deps(&registry, &DepsOptions::new(false)).expect("clean report must return Ok");
        assert!(
            !saw_refresh.load(std::sync::atomic::Ordering::SeqCst),
            "refresh must default to false"
        );

        let (registry, saw_refresh) = stub_registry(clean_report_json());
        run_deps(&registry, &DepsOptions::new(true)).expect("clean report must return Ok");
        assert!(
            saw_refresh.load(std::sync::atomic::Ordering::SeqCst),
            "opts.refresh must reach ctx.is_refreshing() at the provider"
        );
    }

    /// ERR-4 / TASK-1827: `get_or_provide` serves a *previously persisted*
    /// payload when one exists, and `DepsReport` keeps gaining fields, so a
    /// cache written by an older `ops` is a live failure mode. Its bare `?`
    /// used to surface serde's own message — `missing field \`upgrades\`` —
    /// which reads as a bug in ops and hides the one-word remedy.
    #[cfg(unix)]
    #[test]
    #[serial]
    fn run_deps_stale_cached_payload_names_the_report_and_refresh() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _cargo = EnvVarGuard::set("CARGO", fake_cargo(dir.path(), "exit 0"));
        let _cwd = CwdGuard::set(dir.path());

        // A payload an older `ops` could plausibly have persisted: the
        // `upgrades` section had not been added yet.
        let stale = serde_json::json!({ "deny": DenyResult::default() });
        let (registry, _) = stub_registry(stale);

        let err = run_deps(&registry, &DepsOptions::new(false))
            .expect_err("an undecodable cached payload must surface");
        let chain = format!("{err:#}");
        assert!(
            chain.contains("dependency report payload"),
            "error must name the deps report payload; got: {chain}"
        );
        assert!(
            chain.contains("data cache"),
            "error must say the payload may come from the data cache; got: {chain}"
        );
        assert!(
            chain.contains("--refresh"),
            "error must point at --refresh; got: {chain}"
        );
        assert!(
            chain.contains("upgrades"),
            "serde's own diagnosis must be preserved in the chain; got: {chain}"
        );
    }

    /// ERR-4 / TASK-1827 AC#3: a provider-registry failure must name the
    /// provider being resolved rather than surfacing bare.
    #[cfg(unix)]
    #[test]
    #[serial]
    fn run_deps_provider_failure_names_the_deps_provider() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _cargo = EnvVarGuard::set("CARGO", fake_cargo(dir.path(), "exit 0"));
        let _cwd = CwdGuard::set(dir.path());

        // Empty registry: `get_or_provide` fails with `NotFound`.
        let registry = DataRegistry::new();
        let err = run_deps(&registry, &DepsOptions::new(false))
            .expect_err("a missing provider must surface");
        let chain = format!("{err:#}");
        assert!(
            chain.contains(DATA_PROVIDER_NAME) && chain.contains("data provider"),
            "error must name the deps data provider; got: {chain}"
        );
    }

    /// TEST-5 / TASK-1845 AC#5: `ensure_tools` reports a missing tool with
    /// the `cargo install <crate>` hint. A non-zero probe exit is what
    /// "not installed" looks like to `check_tool_in`.
    #[cfg(unix)]
    #[test]
    #[serial]
    fn ensure_tools_reports_the_missing_tool_with_an_install_hint() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _cargo = EnvVarGuard::set("CARGO", fake_cargo(dir.path(), "exit 101"));
        let _cwd = CwdGuard::set(dir.path());

        let err = ensure_tools().expect_err("a failing probe must report the tool as missing");
        let msg = err.to_string();
        assert!(
            msg.contains("cargo upgrade is not installed"),
            "must name the first missing tool: {msg}"
        );
        assert!(
            msg.contains("cargo install cargo-edit"),
            "must carry the install hint: {msg}"
        );
    }

    /// TEST-5 / TASK-1845 AC#4: close the provider-to-consumer round trip.
    /// `types/tests.rs` builds a `DepsReport` by hand, so it cannot catch a
    /// provider emitting a shape `run_deps` then fails to decode. Drive the
    /// real `DepsProvider::provide` against a fake cargo that emits a
    /// parseable upgrade table and a clean `cargo deny`, then decode its
    /// JSON exactly as `run_deps` does.
    #[cfg(unix)]
    #[test]
    #[serial]
    fn deps_provider_output_round_trips_into_a_deps_report() {
        let dir = tempfile::tempdir().expect("tempdir");
        // `cargo upgrade --dry-run` prints the table on stdout; `cargo deny
        // check` exits 0 with no diagnostics. Dispatch on the subcommand,
        // which `run_cargo` passes as the first argument.
        let script = r#"case "$1" in
  upgrade)
    printf 'name   old req compatible latest  new req note\n'
    printf '====   ======= ========== ======  ======= ====\n'
    printf 'clap   3.0.0   3.2.25     4.6.0   3.2.25  incompatible\n'
    printf 'serde  1.0.100 1.0.228    1.0.228 1.0.228\n'
    exit 0 ;;
  deny) exit 0 ;;
  *) exit 0 ;;
esac"#;
        let _cargo = EnvVarGuard::set("CARGO", fake_cargo(dir.path(), script));

        let mut ctx = Context::new(
            std::sync::Arc::new(ops_core::config::Config::empty()),
            dir.path().to_path_buf(),
        );
        let value = DepsProvider
            .provide(&mut ctx)
            .expect("provider must succeed against a well-formed cargo");

        let report: DepsReport =
            serde_json::from_value(value).expect("provider JSON must decode as a DepsReport");
        assert_eq!(report.upgrades.compatible.len(), 1);
        assert_eq!(report.upgrades.compatible[0].name, "serde");
        assert_eq!(report.upgrades.incompatible.len(), 1);
        assert_eq!(report.upgrades.incompatible[0].name, "clap");
        assert!(report.deny.advisories.is_empty());
        assert!(!has_issues(&report), "a clean deny run must pass the gate");
    }
}

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
        // TEST-23 / TASK-1842: the restore lives in a `Drop` guard so the
        // panic this test can produce cannot leave the whole binary running
        // in a tempdir that `TempDir::drop` then deletes.
        let _cwd = crate::test_support::CwdGuard::set(dir.path());
        // TEST-23: `merge_env_vars` overlays any `OPS__*` variable on top of
        // the on-disk config, so an ambient `OPS__STACK` would decide this
        // assertion instead of the `.ops.toml` the test just wrote. Clear it
        // for the call and restore whatever the developer had.
        let _stack = crate::test_support::EnvVarGuard::unset("OPS__STACK");

        let ctx = build_user_context().expect("build_user_context");

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

/// DUP-3 / TASK-1821: the gate (`severity_is_actionable`) and the renderer
/// (`SeverityClass`) must not be able to disagree. Both now classify through
/// the same `SeverityClass::classify`, so this walks every severity string
/// the crate can produce — the five cargo-deny values, the
/// `<missing-severity>` sentinel, and an unknown one — and asserts the gate
/// decision matches the rendered status. A one-sided edit to either module
/// fails here rather than shipping a red report on a zero exit (or the
/// reverse).
#[test]
fn gate_and_renderer_agree_on_every_severity() {
    use crate::format::SeverityClass;
    use crate::parse::MISSING_SEVERITY_SENTINEL;
    use ops_core::report::ReportStatus;

    // (severity, actionable on the strict gate, actionable on the relaxed
    // bans gate, rendered status)
    let cases: &[(&str, bool, bool, ReportStatus)] = &[
        ("error", true, true, ReportStatus::Error),
        ("warning", true, false, ReportStatus::Warning),
        ("note", false, false, ReportStatus::Info),
        ("help", false, false, ReportStatus::Info),
        ("info", false, false, ReportStatus::Info),
        (MISSING_SEVERITY_SENTINEL, true, true, ReportStatus::Error),
        ("critical", true, true, ReportStatus::Error),
    ];

    for &(severity, strict, relaxed, status) in cases {
        assert_eq!(
            severity_is_actionable(severity, false),
            strict,
            "strict gate disagreed for severity {severity:?}"
        );
        assert_eq!(
            severity_is_actionable(severity, true),
            relaxed,
            "relaxed (bans) gate disagreed for severity {severity:?}"
        );
        assert_eq!(
            SeverityClass::classify(severity).report_status(),
            status,
            "renderer disagreed for severity {severity:?}"
        );
        // The load-bearing invariant: anything the renderer paints as an
        // error must fail the strict gate, and anything it paints as info
        // must not.
        if status == ReportStatus::Error {
            assert!(
                severity_is_actionable(severity, false),
                "renderer says Error but the gate passes for severity {severity:?}"
            );
        }
        if status == ReportStatus::Info {
            assert!(
                !severity_is_actionable(severity, false),
                "renderer says Info but the gate fails for severity {severity:?}"
            );
        }
    }
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

    // TEST-23 / TASK-1842: the env mutations are reverted by `Drop`, which
    // runs on the unwind path too. Without that, an assertion failure here
    // leaked a `$CARGO` pointing at `exec sleep 30` plus a 1-second
    // subprocess timeout into every later test in the binary, turning one
    // clear failure into a cascade of unrelated timeout errors. The
    // `unsafe`/SAFETY argument lives on the guard itself and covers
    // restore-on-unwind, not just the `#[serial]` concurrency point.
    let _cargo = crate::test_support::EnvVarGuard::set("CARGO", &fake);
    let _timeout = crate::test_support::EnvVarGuard::set(ops_core::subprocess::TIMEOUT_ENV, "1");

    // Assertions can sit directly after the call now that cleanup no longer
    // depends on reaching the end of the body.
    let err =
        check_tool_in(&tool, dir.path()).expect_err("hung probe must error rather than block");
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
