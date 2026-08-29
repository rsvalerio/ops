//! Tests for the `cargo deny check` JSON diagnostic parser and for the
//! exit-code contract that decides whether its stderr is authoritative.

use super::*;

// -- Deny exit-code interpretation --

#[test]
fn interpret_deny_result_treats_exit_code_2_as_config_error() {
    // TASK-0386: cargo-deny exit 2 = configuration error (e.g. broken
    // deny.toml). interpret_deny_result must surface that, not silently
    // return an empty DenyResult.
    let stderr = "error: failed to read deny.toml: invalid TOML at line 4\n";
    let result = interpret_deny_result(Some(2), stderr);
    let err = result.expect_err("config error must surface");
    let msg = err.to_string();
    assert!(
        msg.contains("status 2") && msg.contains("configuration error"),
        "expected exit-2 error, got: {msg}"
    );
    assert!(
        msg.contains("invalid TOML"),
        "stderr context preserved: {msg}"
    );
}

/// ERR-1 / TASK-0612: cargo-deny exit 1 with empty stderr is "binary crashed
/// before emitting diagnostics", not "no issues found". Returning Ok(default)
/// would let `ops deps` exit 0 on a silently broken supply-chain pipeline.
#[test]
fn interpret_deny_result_errs_on_exit_1_with_empty_stderr() {
    let result = interpret_deny_result(Some(1), "");
    let err = result.expect_err("empty stderr at exit 1 must surface");
    let msg = err.to_string();
    assert!(
        msg.contains("status 1") && msg.contains("no diagnostics"),
        "got: {msg}"
    );
}

#[test]
fn interpret_deny_result_errs_on_exit_1_with_whitespace_stderr() {
    let result = interpret_deny_result(Some(1), "   \n\t \n");
    assert!(
        result.is_err(),
        "whitespace-only stderr at exit 1 must surface"
    );
}

/// ERR-1 / TASK-0958: exit 1 with non-empty but non-JSON stderr (e.g. text-mode
/// banners "error[A001]: …" if the user-format flag drifts) must fail closed.
/// Previously every line decoded as malformed JSON, dropped to debug, and the
/// gate scored green on a non-diagnostic stream.
#[test]
fn interpret_deny_result_errs_on_exit_1_with_non_json_stderr() {
    let stderr = "error[A001]: failed to parse manifest\nerror: aborting due to previous error\n";
    let result = interpret_deny_result(Some(1), stderr);
    let err = result.expect_err("non-JSON stderr at exit 1 must surface");
    let msg = err.to_string();
    assert!(
        msg.contains("status 1") && msg.contains("zero diagnostics"),
        "error must cite the zero-diagnostic case, got: {msg}"
    );
}

/// ERR-7 (TASK-0598): `exit_code` = None means cargo-deny was killed by
/// signal. Treating partial stderr as an authoritative diagnostic stream
/// silently turns a SIGKILL/OOM into a "clean" run; the gate must error
/// instead so CI does not score a killed run as green.
#[test]
fn interpret_deny_result_errs_on_signal_kill() {
    let result = interpret_deny_result(None, "");
    let err = result.expect_err("None exit code must surface");
    assert!(
        err.to_string().contains("signal"),
        "error must name the signal-kill case, got: {err}"
    );
}

#[test]
fn interpret_deny_result_errs_on_signal_kill_even_with_partial_stderr() {
    // Even if the binary flushed some JSON before being killed, partial
    // diagnostics are not a clean run.
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"x","code":"vulnerability","advisory":{"id":"RUSTSEC-2024-0001","package":"x","title":"t"},"graphs":[]}}"#;
    let result = interpret_deny_result(None, stderr);
    assert!(result.is_err());
}

#[test]
fn interpret_deny_result_passes_exit_code_0_through() {
    // Clean run: empty stderr, no diagnostics.
    let result = interpret_deny_result(Some(0), "").expect("clean run is Ok");
    assert!(result.advisories.is_empty());
    assert!(result.licenses.is_empty());
    assert!(result.bans.is_empty());
    assert!(result.sources.is_empty());
}

#[test]
fn interpret_deny_result_parses_diagnostics_on_exit_code_1() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"`atty` is unmaintained","code":"unmaintained","advisory":{"id":"RUSTSEC-2024-0375","package":"atty","title":"`atty` is unmaintained"},"graphs":[{"Krate":{"name":"atty","version":"0.2.14"},"parents":[]}]}}"#;
    let result = interpret_deny_result(Some(1), stderr).expect("issues run is Ok");
    assert_eq!(result.advisories.len(), 1);
    assert_eq!(result.advisories[0].id, "RUSTSEC-2024-0375");
}

// -- OWN-1 / TASK-1848: resolve_package is a read, not a consume --

/// Resolving the package twice — or resolving and then re-reading the
/// advisory / graph the answer came from — must yield the same name. The
/// `&mut` version hollowed out whatever it read, so the second call returned
/// `<no package>` for a diagnostic that had one, plus a false TASK-0597
/// "no package name" warning.
#[test]
fn resolve_package_is_idempotent_and_leaves_the_diagnostic_intact() {
    let advisory_backed = DecodedDiagnostic {
        code: "vulnerability".to_string(),
        severity: "error".to_string(),
        message: "vulnerable".to_string(),
        advisory: Some(DenyAdvisory {
            id: "RUSTSEC-2024-0099".to_string(),
            package: Some("vuln-pkg".to_string()),
            title: Some("vuln title".to_string()),
        }),
        graphs: None,
    };
    assert_eq!(resolve_package(&advisory_backed), "vuln-pkg");
    assert_eq!(
        resolve_package(&advisory_backed),
        "vuln-pkg",
        "a second resolve must not fall through to the <no package> sentinel"
    );
    assert_eq!(
        advisory_backed
            .advisory
            .as_ref()
            .and_then(|a| a.package.as_deref()),
        Some("vuln-pkg"),
        "resolve_package must leave the advisory readable for push_diagnostic"
    );

    let graph_backed = DecodedDiagnostic {
        code: "banned".to_string(),
        severity: "error".to_string(),
        message: "crate is banned".to_string(),
        advisory: None,
        graphs: Some(vec![DenyGraph {
            krate: Some(DenyKrate {
                name: "bad-crate".to_string(),
            }),
        }]),
    };
    assert_eq!(resolve_package(&graph_backed), "bad-crate");
    assert_eq!(
        resolve_package(&graph_backed),
        "bad-crate",
        "the graphs[0].krate fallback must survive a first resolve too"
    );
    assert_eq!(
        graph_backed
            .graphs
            .as_ref()
            .and_then(|g| g.first())
            .and_then(|g| g.krate.as_ref())
            .map(|k| k.name.as_str()),
        Some("bad-crate"),
        "resolve_package must not empty krate.name"
    );
}

// -- ERR-1 / TASK-1840: partial decode loss --

/// The exit-1 guard only caught *total* decode failure, so a per-code schema
/// change that took out one class passed straight through: every advisory
/// falls into `classify_code`'s `None` arm, a single unrelated ban still
/// decodes, `is_empty()` stays false on that one vector, and `ops deps`
/// renders "Advisories: None" in green while an unpatched RUSTSEC advisory
/// sits in the tree.
#[test]
fn interpret_deny_result_errs_on_partial_decode_loss() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"warning","message":"duplicate","code":"duplicate","graphs":[{"Krate":{"name":"baz","version":"2.0.0"}}]}}
{"type":"diagnostic","fields":{"severity":"error","message":"vuln","code":"security-vulnerability","advisory":{"id":"RUSTSEC-2024-0001","package":"a","title":"t"}}}
{"type":"diagnostic","fields":{"severity":"error","message":"vuln","code":"security-vulnerability","advisory":{"id":"RUSTSEC-2024-0002","package":"b","title":"t"}}}
{"type":"diagnostic","fields":{"severity":"error","message":"vuln","code":"security-vulnerability","advisory":{"id":"RUSTSEC-2024-0003","package":"c","title":"t"}}}"#;

    let err = interpret_deny_result(Some(1), stderr)
        .expect_err("a class-wide decode loss must not report the surviving subset as complete");
    let msg = err.to_string();
    assert!(
        msg.contains("4 diagnostic line(s)") && msg.contains("3 dropped"),
        "error must report the seen/decoded counts; got: {msg}"
    );
    // Distinguishable from the TASK-0958 zero-diagnostics and TASK-0612
    // empty-stderr cases, both of which describe a stream with nothing in it.
    assert!(
        !msg.contains("zero diagnostics") && !msg.contains("no diagnostics"),
        "partial loss must not read as the total-loss cases; got: {msg}"
    );
}

/// The tolerance is a *share*, not zero: one unrecognised code among many
/// findings is ordinary forward drift (cargo-deny adding a category) and
/// must not fail the gate, or every upstream release breaks `ops deps`.
#[test]
fn interpret_deny_result_tolerates_a_single_unknown_code_among_many() {
    let known = (0..9)
        .map(|i| {
            format!(
                r#"{{"type":"diagnostic","fields":{{"severity":"error","message":"m","code":"banned","graphs":[{{"Krate":{{"name":"pkg-{i}","version":"1.0.0"}}}}]}}}}"#
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let stderr = format!(
        "{known}\n{}",
        r#"{"type":"diagnostic","fields":{"severity":"warning","message":"new","code":"hypothetical-new-category","graphs":[]}}"#
    );

    let result =
        interpret_deny_result(Some(1), &stderr).expect("1-in-10 unknown codes must stay tolerated");
    assert_eq!(result.bans.len(), 9);
}

/// `log` and `summary` envelopes are not findings, so they must not count
/// toward the candidate denominator — otherwise a normal run with a couple
/// of findings and a chatty log stream would look like a mass drop.
#[test]
fn interpret_deny_result_log_envelopes_do_not_inflate_the_candidate_count() {
    let stderr = r#"{"type":"log","fields":{"timestamp":"2024-01-01","level":"INFO","message":"checking"}}
{"type":"log","fields":{"timestamp":"2024-01-01","level":"INFO","message":"fetching"}}
{"type":"diagnostic","fields":{"severity":"error","message":"crate is banned","code":"banned","graphs":[{"Krate":{"name":"bad","version":"0.1.0"}}]}}
{"type":"summary","fields":{"bans":{"errors":1}}}"#;

    let result = interpret_deny_result(Some(1), stderr)
        .expect("log/summary envelopes must not be counted as dropped diagnostics");
    assert_eq!(result.bans.len(), 1);
}

/// ERR-1 / TASK-1840: a diagnostic envelope whose `fields` no longer match
/// `DiagnosticFields` is still cargo-deny claiming a finding, so it must be
/// counted as a candidate and therefore as a *drop*.
///
/// Before the two-stage decode, the whole line failed `serde_json::from_str`
/// and fell into the malformed-JSON arm, which counts nothing: three
/// broken advisories plus one surviving ban produced
/// `candidates == 1, dropped == 0` and the gate stayed green while an entire
/// class had disappeared.
#[test]
fn interpret_deny_result_counts_diagnostics_whose_fields_fail_to_decode() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"crate is banned","code":"banned","graphs":[{"Krate":{"name":"bad","version":"0.1.0"}}]}}
{"type":"diagnostic","fields":{"severity":["error"],"message":"vuln","code":"security-vulnerability"}}
{"type":"diagnostic","fields":"not-an-object"}
{"type":"diagnostic"}"#;

    let err = interpret_deny_result(Some(1), stderr)
        .expect_err("recognised diagnostic envelopes with undecodable fields must count as drops");
    let msg = err.to_string();
    assert!(
        msg.contains("4 diagnostic line(s)") && msg.contains("3 dropped"),
        "every `type=diagnostic` envelope must reach the denominator; got: {msg}"
    );
}

/// A line that is not JSON at all, and a non-diagnostic envelope, still must
/// not reach the denominator — otherwise a chatty run looks like a mass drop.
#[test]
fn interpret_deny_result_malformed_non_diagnostic_lines_are_not_candidates() {
    let stderr = r#"this is not json at all
{"type":"log","fields":"not-an-object"}
{"type":"diagnostic","fields":{"severity":"error","message":"crate is banned","code":"banned","graphs":[{"Krate":{"name":"bad","version":"0.1.0"}}]}}"#;

    let result = interpret_deny_result(Some(1), stderr)
        .expect("non-diagnostic noise must not be counted as dropped diagnostics");
    assert_eq!(result.bans.len(), 1);
}

/// ERR-1 / TASK-1840 AC#4: the missing-`code` drop path was the only one in
/// the crate with no tracing breadcrumb.
#[test]
#[serial_test::serial]
fn parse_deny_missing_code_logs_a_breadcrumb() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"something","graphs":[{"Krate":{"name":"pkg","version":"1.0.0"}}]}}"#;

    let (logged, result) =
        crate::test_support::capture_tracing(tracing::Level::DEBUG, || parse_deny_output(stderr));
    assert!(result.advisories.is_empty());
    assert!(
        logged.contains("TASK-1840") && logged.contains("code"),
        "expected a missing-code breadcrumb; got: {logged}"
    );
}

// -- Deny output parser tests --

#[test]
fn parse_deny_advisory() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"`atty` is unmaintained","code":"unmaintained","advisory":{"id":"RUSTSEC-2024-0375","package":"atty","title":"`atty` is unmaintained","description":"...","date":"2024-09-25","informational":"unmaintained","url":"https://example.com","aliases":[],"categories":[],"cvss":null,"keywords":[],"references":[],"related":[],"withdrawn":null},"labels":[],"graphs":[{"Krate":{"name":"atty","version":"0.2.14"},"parents":[]}],"notes":["ID: RUSTSEC-2024-0375"]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.advisories.len(), 1);
    assert_eq!(result.advisories[0].id, "RUSTSEC-2024-0375");
    assert_eq!(result.advisories[0].package, "atty");
    assert_eq!(result.advisories[0].severity, "error");
    assert_eq!(result.advisories[0].title, "`atty` is unmaintained");
    assert!(result.licenses.is_empty());
    assert!(result.bans.is_empty());
    assert!(result.sources.is_empty());
}

#[test]
fn parse_deny_license() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"failed to satisfy license requirements","code":"rejected","labels":[{"message":"rejected","span":"MIT"}],"graphs":[{"Krate":{"name":"some-crate","version":"1.0.0"},"parents":[]}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.licenses.len(), 1);
    assert_eq!(result.licenses[0].package, "some-crate");
    assert_eq!(result.licenses[0].severity, "error");
}

#[test]
fn parse_deny_ban() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"crate is banned","code":"banned","labels":[],"graphs":[{"Krate":{"name":"bad-crate","version":"0.1.0"},"parents":[]}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.bans.len(), 1);
    assert_eq!(result.bans[0].package, "bad-crate");
}

#[test]
fn parse_deny_source() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"source not allowed","code":"source-not-allowed","labels":[],"graphs":[{"Krate":{"name":"sketchy-crate","version":"0.1.0"},"parents":[]}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.sources.len(), 1);
    assert_eq!(result.sources[0].package, "sketchy-crate");
}

#[test]
fn parse_deny_skips_log_and_summary() {
    let stderr = r#"{"type":"log","fields":{"timestamp":"2024-01-01","level":"INFO","message":"checking"}}
{"type":"summary","fields":{"advisories":{"errors":0},"bans":{"errors":0},"licenses":{"errors":0},"sources":{"errors":0}}}"#;
    let result = parse_deny_output(stderr);
    assert!(result.advisories.is_empty());
    assert!(result.licenses.is_empty());
    assert!(result.bans.is_empty());
    assert!(result.sources.is_empty());
}

#[test]
fn parse_deny_empty() {
    let result = parse_deny_output("");
    assert!(result.advisories.is_empty());
    assert!(result.licenses.is_empty());
    assert!(result.bans.is_empty());
    assert!(result.sources.is_empty());
}

#[test]
fn parse_deny_skips_invalid_json() {
    let stderr = "not json\n{broken\n";
    let result = parse_deny_output(stderr);
    assert!(result.advisories.is_empty());
}

#[test]
fn parse_deny_mixed_diagnostics() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"unmaintained","code":"unmaintained","advisory":{"id":"RUSTSEC-2024-0001","package":"foo","title":"foo is old"},"labels":[],"graphs":[],"notes":[]}}
{"type":"diagnostic","fields":{"severity":"error","message":"license rejected","code":"rejected","labels":[],"graphs":[{"Krate":{"name":"bar","version":"1.0.0"}}],"notes":[]}}
{"type":"diagnostic","fields":{"severity":"warning","message":"duplicate","code":"duplicate","labels":[],"graphs":[{"Krate":{"name":"baz","version":"2.0.0"}}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.advisories.len(), 1);
    assert_eq!(result.licenses.len(), 1);
    assert_eq!(result.bans.len(), 1);
}

/// TASK-0436: a diagnostic whose code is not in any of the four known sets
/// (e.g. cargo-deny adds a new category) is dropped from the result, but
/// still observable via `tracing::debug` — the entry must not silently change
/// the `DenyResult` shape.
#[test]
fn parse_deny_unknown_code_does_not_appear_in_result() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"warning","message":"future schema","code":"hypothetical-new-category","labels":[],"graphs":[{"Krate":{"name":"some","version":"1.0.0"}}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert!(result.advisories.is_empty());
    assert!(result.licenses.is_empty());
    assert!(result.bans.is_empty());
    assert!(result.sources.is_empty());
}

/// ERR-2 / TASK-0845: a diagnostic that lacks a `severity` field used to
/// be silently substituted with the literal "error" sentinel — making it
/// indistinguishable from a real cargo-deny error and silently inverting
/// the fail-closed schema-drift contract for informational diagnostics.
/// The new behaviour preserves the missing severity as the
/// `<missing-severity>` sentinel, which routes through `has_issues`'s
/// fail-closed `_other` branch (still fails the gate) but is observable
/// in the parsed entry and via tracing.
#[test]
fn parse_deny_missing_severity_uses_distinct_sentinel() {
    use crate::parse::MISSING_SEVERITY_SENTINEL;
    let stderr = r#"{"type":"diagnostic","fields":{"message":"unmaintained","code":"unmaintained","advisory":{"id":"RUSTSEC-2024-0001","package":"foo","title":"foo is old"},"labels":[],"graphs":[],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.advisories.len(), 1);
    assert_eq!(
        result.advisories[0].severity, MISSING_SEVERITY_SENTINEL,
        "missing severity must surface as a distinct sentinel, not 'error'"
    );
    assert_ne!(
        result.advisories[0].severity, "error",
        "must not collide with the legitimate 'error' severity value"
    );
}

// -- Deny parser edge cases --

#[test]
fn parse_deny_no_code_field_skipped() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"something","labels":[],"graphs":[{"Krate":{"name":"pkg","version":"1.0.0"}}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert!(result.advisories.is_empty());
    assert!(result.licenses.is_empty());
    assert!(result.bans.is_empty());
    assert!(result.sources.is_empty());
}

/// ERR-2 / TASK-0845: a diagnostic with no severity must NOT be silently
/// rebadged as `"error"` — that collides with cargo-deny's legitimate
/// "error" value and prevents callers (and operators reading logs) from
/// distinguishing "real error" from "schema drift, severity field gone".
/// The new contract uses `<missing-severity>` as a distinct sentinel that
/// `has_issues` routes through the fail-closed `_other` branch.
#[test]
fn parse_deny_no_severity_uses_missing_sentinel_not_error() {
    use crate::parse::MISSING_SEVERITY_SENTINEL;
    let stderr = r#"{"type":"diagnostic","fields":{"message":"license rejected","code":"rejected","labels":[],"graphs":[{"Krate":{"name":"some-crate","version":"1.0.0"}}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.licenses.len(), 1);
    assert_eq!(result.licenses[0].severity, MISSING_SEVERITY_SENTINEL);
    assert_ne!(result.licenses[0].severity, "error");
}

#[test]
fn parse_deny_advisory_without_advisory_field_uses_code_as_id() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"warning","message":"crate is yanked","code":"yanked","labels":[],"graphs":[{"Krate":{"name":"old-crate","version":"0.1.0"}}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.advisories.len(), 1);
    assert_eq!(result.advisories[0].id, "yanked");
    assert_eq!(result.advisories[0].title, "crate is yanked");
    assert_eq!(result.advisories[0].package, "old-crate");
}

#[test]
fn parse_deny_package_from_graphs_when_no_advisory_package() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"vulnerable","code":"vulnerability","advisory":{"id":"RUSTSEC-2024-0099","title":"vuln title"},"labels":[],"graphs":[{"Krate":{"name":"vuln-pkg","version":"1.0.0"}}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.advisories.len(), 1);
    assert_eq!(result.advisories[0].package, "vuln-pkg");
    assert_eq!(result.advisories[0].id, "RUSTSEC-2024-0099");
}

#[test]
fn parse_deny_package_unknown_when_no_graphs_or_advisory_package() {
    // ERR-7 (TASK-0597): the missing-package sentinel must be visibly
    // distinct from any plausible crate name so operators can tell schema
    // drift apart from a real dependency on a crate named "unknown".
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"bad license","code":"unlicensed","labels":[],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.licenses.len(), 1);
    assert_eq!(result.licenses[0].package, "<no package>");
}

#[test]
fn parse_deny_additional_advisory_codes() {
    // Test "vulnerability", "notice", "unsound" codes
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"vuln found","code":"vulnerability","advisory":{"id":"RUSTSEC-2024-0010","package":"pkg-a","title":"vuln"},"labels":[],"graphs":[],"notes":[]}}
{"type":"diagnostic","fields":{"severity":"warning","message":"notice issued","code":"notice","advisory":{"id":"RUSTSEC-2024-0011","package":"pkg-b","title":"notice"},"labels":[],"graphs":[],"notes":[]}}
{"type":"diagnostic","fields":{"severity":"error","message":"unsound code","code":"unsound","advisory":{"id":"RUSTSEC-2024-0012","package":"pkg-c","title":"unsound"},"labels":[],"graphs":[],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.advisories.len(), 3);
    assert_eq!(result.advisories[0].id, "RUSTSEC-2024-0010");
    assert_eq!(result.advisories[1].id, "RUSTSEC-2024-0011");
    assert_eq!(result.advisories[2].id, "RUSTSEC-2024-0012");
}

#[test]
fn parse_deny_additional_license_codes() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"no license","code":"unlicensed","labels":[],"graphs":[{"Krate":{"name":"pkg-a","version":"1.0.0"}}],"notes":[]}}
{"type":"diagnostic","fields":{"severity":"warning","message":"missing field","code":"no-license-field","labels":[],"graphs":[{"Krate":{"name":"pkg-b","version":"1.0.0"}}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.licenses.len(), 2);
    assert_eq!(result.licenses[0].package, "pkg-a");
    assert_eq!(result.licenses[1].package, "pkg-b");
}

#[test]
fn parse_deny_additional_ban_codes() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"crate not allowed","code":"not-allowed","labels":[],"graphs":[{"Krate":{"name":"pkg-a","version":"1.0.0"}}],"notes":[]}}
{"type":"diagnostic","fields":{"severity":"warning","message":"workspace dup","code":"workspace-duplicate","labels":[],"graphs":[{"Krate":{"name":"pkg-b","version":"1.0.0"}}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.bans.len(), 2);
    assert_eq!(result.bans[0].package, "pkg-a");
    assert_eq!(result.bans[1].package, "pkg-b");
}

#[test]
fn parse_deny_git_source_underspecified() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"warning","message":"git source underspecified","code":"git-source-underspecified","labels":[],"graphs":[{"Krate":{"name":"git-dep","version":"0.1.0"}}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.sources.len(), 1);
    assert_eq!(result.sources[0].package, "git-dep");
}

#[test]
fn parse_deny_unknown_code_ignored() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"something new","code":"future-check-type","labels":[],"graphs":[{"Krate":{"name":"pkg","version":"1.0.0"}}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert!(result.advisories.is_empty());
    assert!(result.licenses.is_empty());
    assert!(result.bans.is_empty());
    assert!(result.sources.is_empty());
}

#[test]
fn parse_deny_fields_deserialization_failure_skipped() {
    // Valid JSON line but fields can't deserialize to DiagnosticFields
    let stderr = r#"{"type":"diagnostic","fields":"not an object"}"#;
    let result = parse_deny_output(stderr);
    assert!(result.advisories.is_empty());
}

#[test]
#[serial_test::serial]
fn parse_deny_output_skips_malformed_json_with_tracing() {
    // First line is malformed JSON; second has valid envelope but bad fields
    // shape. Both should be skipped; both should log.
    let stderr = "{not json\n{\"type\":\"diagnostic\",\"fields\":42}\n";

    let (logged, result) =
        crate::test_support::capture_tracing(tracing::Level::DEBUG, || parse_deny_output(stderr));
    assert!(result.advisories.is_empty());
    assert!(logged.contains("ERR-1"), "missing ERR-1 marker: {logged}");
    assert!(
        logged.contains("malformed cargo-deny JSON line"),
        "missing malformed-line message: {logged}"
    );
}

// -- ERR-7 / SEC-21 / TASK-1160: stderr tail Debug-escapes control bytes --

/// `interpret_deny_result` must format the stderr tail through the `?`
/// formatter so embedded ANSI / newlines / NULs from cargo-deny cannot forge
/// log records or repaint the operator terminal. The sibling contract on
/// `interpret_upgrade_output` is pinned in `upgrade/exit_code_tests.rs`.
/// Pin the escape on the zero-diagnostics exit-1 arm.
#[test]
fn interpret_deny_result_zero_diagnostics_debug_escapes_stderr_tail() {
    let stderr = "error[A001]\n\x1b[31mfatal\x1b[0m\n";
    let result = interpret_deny_result(Some(1), stderr);
    let err = result.expect_err("non-JSON stderr at exit 1 must surface");
    let msg = err.to_string();
    assert!(
        !msg.contains('\u{1b}'),
        "ANSI ESC must not survive in: {msg:?}"
    );
}

/// Pin the escape contract on the exit-2 (configuration error) arm — TASK-1250.
#[test]
fn interpret_deny_result_exit_two_debug_escapes_stderr_tail() {
    let stderr = "error: \x1b[31minvalid TOML\x1b[0m\nbye\n";
    let result = interpret_deny_result(Some(2), stderr);
    let err = result.expect_err("config error must surface");
    let msg = err.to_string();
    assert!(
        !msg.contains('\u{1b}'),
        "ANSI ESC must not survive in: {msg:?}"
    );
    // Operator-readable content survives.
    assert!(
        msg.contains("invalid TOML"),
        "expected stderr context preserved: {msg}"
    );
}
