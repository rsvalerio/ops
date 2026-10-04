//! Tests for the `cargo deny check` JSON diagnostic parser and for the
//! exit-code contract that decides whether its stderr is authoritative.

use super::*;

// -- Deny exit-code interpretation --

/// A broken `deny.toml` is not a check failure: cargo-deny 0.20 logs the
/// reason at `ERROR` and exits 1 (the same value as an advisories-only
/// failure) without emitting a classifiable diagnostic. The stream below is
/// what `cargo deny --format json check` prints for a key it does not know.
#[test]
fn interpret_deny_result_reports_config_error_from_error_log() {
    let stderr = r#"{"fields":{"code":"unexpected-keys","labels":[{"column":1,"line":3,"message":"","span":"bogus-key"}],"message":"found 1 unexpected keys","severity":"error"},"type":"diagnostic"}
{"fields":{"level":"ERROR","message":"failed to deserialize config from 'deny.toml'","timestamp":"2026-10-04T14:59:58Z"},"type":"log"}"#;
    let err = interpret_deny_result(Some(1), stderr).expect_err("config error must surface");
    let msg = err.to_string();
    assert!(
        msg.contains("status 1") && msg.contains("configuration error"),
        "expected a configuration error, got: {msg}"
    );
    assert!(
        msg.contains("failed to deserialize config"),
        "cargo-deny's reason preserved: {msg}"
    );
}

/// The reason is cargo-deny's own text, so it is Debug-escaped like every
/// other stderr excerpt.
#[test]
fn interpret_deny_result_config_error_debug_escapes_reason() {
    let stderr = r#"{"fields":{"level":"ERROR","message":"failed to parse \u001b[31mconfig\u001b[0m"},"type":"log"}"#;
    let err = interpret_deny_result(Some(1), stderr).expect_err("config error must surface");
    let msg = err.to_string();
    assert!(msg.contains("configuration error"), "got: {msg}");
    assert!(
        !msg.contains('\u{1b}'),
        "ANSI ESC must not survive in: {msg:?}"
    );
}

/// An `ERROR` log next to decodable findings does not hide them: the run
/// reported what it found, so the findings are rendered.
#[test]
fn interpret_deny_result_keeps_findings_despite_error_log() {
    let stderr = r#"{"type":"log","fields":{"level":"ERROR","message":"failed to fetch advisory database"}}
{"type":"diagnostic","fields":{"severity":"error","message":"crate is banned","code":"banned","graphs":[{"Krate":{"name":"bad","version":"0.1.0"}}]}}"#;
    let result = interpret_deny_result(Some(2), stderr).expect("findings decode");
    assert_eq!(result.bans.len(), 1);
}

/// A usage error exits 2 — the same value as a bans-only failure — with
/// plain-text stderr. With no diagnostic to decode it must surface as an
/// error rather than as a clean bans check.
#[test]
fn interpret_deny_result_errs_on_exit_2_usage_error() {
    let stderr = "error: unexpected argument '--nope' found\n";
    let err = interpret_deny_result(Some(2), stderr).expect_err("usage error must surface");
    let msg = err.to_string();
    assert!(
        msg.contains("status 2") && msg.contains("zero diagnostics"),
        "expected the exit-2 zero-diagnostics error, got: {msg}"
    );
    assert!(
        msg.contains("unexpected argument"),
        "stderr context preserved: {msg}"
    );
}

/// cargo-deny 0.20 exits with a bitset of the failed checks. A bans-only
/// failure exits 2; its findings are decoded, not reported as a
/// configuration error. Stream captured from cargo-deny 0.20.2.
#[test]
fn interpret_deny_result_decodes_bans_only_exit_code() {
    let stderr = r#"{"fields":{"code":"banned","graphs":[{"Krate":{"name":"denyexp","version":"0.1.0"}}],"labels":[{"column":19,"line":2,"message":"banned here","span":"denyexp"}],"message":"crate 'denyexp = 0.1.0' is explicitly banned","severity":"error"},"type":"diagnostic"}
{"fields":{"bans":{"errors":1,"helps":0,"notes":0,"warnings":0},"licenses":{"errors":0,"helps":1,"notes":0,"warnings":0},"sources":{"errors":0,"helps":0,"notes":0,"warnings":0}},"type":"summary"}"#;
    let result = interpret_deny_result(Some(2), stderr).expect("bans-only failure decodes");
    assert_eq!(result.bans.len(), 1);
    assert_eq!(result.bans[0].0.package, "denyexp");
    assert!(result.licenses.is_empty());
}

/// A licenses-only failure exits 4. Stream captured from cargo-deny 0.20.2.
#[test]
fn interpret_deny_result_decodes_licenses_only_exit_code() {
    let stderr = r#"{"fields":{"code":"rejected","graphs":[{"Krate":{"name":"denyexp","version":"0.1.0"}}],"labels":[{"column":12,"line":5,"message":"rejected: license is not explicitly allowed","span":"MIT"}],"message":"failed to satisfy license requirements","severity":"error"},"type":"diagnostic"}
{"fields":{"code":"license-not-encountered","graphs":[],"labels":[{"column":11,"line":2,"message":"unmatched license allowance","span":"Apache-2.0"}],"message":"license was not encountered","severity":"warning"},"type":"diagnostic"}
{"fields":{"bans":{"errors":0,"helps":0,"notes":0,"warnings":0},"licenses":{"errors":1,"helps":0,"notes":0,"warnings":1},"sources":{"errors":0,"helps":0,"notes":0,"warnings":0}},"type":"summary"}"#;
    let result = interpret_deny_result(Some(4), stderr).expect("licenses-only failure decodes");
    assert_eq!(result.licenses.len(), 1);
    assert_eq!(result.licenses[0].0.package, "denyexp");
    assert_eq!(result.unused_license_allowances.len(), 1);
    assert!(result.bans.is_empty());
}

/// Every non-empty combination of the four check bits is a check failure;
/// the first value past the mask is not.
#[test]
fn interpret_deny_result_accepts_every_check_bitset_and_nothing_wider() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"crate is banned","code":"banned","graphs":[{"Krate":{"name":"bad","version":"0.1.0"}}]}}"#;
    for code in 1..=15 {
        let result = interpret_deny_result(Some(code), stderr)
            .unwrap_or_else(|e| panic!("exit {code} is a check failure: {e}"));
        assert_eq!(result.bans.len(), 1, "exit {code}");
    }
    for code in [16, 17, 101, -1] {
        let err = interpret_deny_result(Some(code), stderr)
            .expect_err("a status outside the check bitset must surface");
        assert!(
            err.to_string()
                .contains(&format!("unexpected status code {code}")),
            "exit {code}: {err}"
        );
    }
}

/// cargo-deny exit 1 with empty stderr is "binary crashed before emitting
/// diagnostics", not "no issues found". Returning `Ok(default)` would let
/// `ops deps` exit 0 on a silently broken supply-chain pipeline.
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

/// Exit 1 with non-empty but non-JSON stderr (e.g. text-mode banners
/// "error[A001]: …" if the format flag drifts) fails closed. Without that,
/// every line decodes as malformed JSON, drops to debug, and the gate scores
/// green on a non-diagnostic stream.
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

/// `exit_code` = None means cargo-deny was killed by a signal. Treating
/// partial stderr as an authoritative diagnostic stream would turn a
/// SIGKILL/OOM into a "clean" run, so the gate errors instead.
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

/// Exit 0 with a non-empty stream that decoded **no** JSON envelopes at
/// all — a plain-text warning from a wrapper or a forgotten `--format
/// json` — must not score green. The stream is not the contract the parser
/// understands, and accepting it would be the exit-0 twin of the
/// zero-diagnostics-at-exit-1 silent muting.
#[test]
fn interpret_deny_result_errs_on_exit_code_0_with_non_json_stderr() {
    let result = interpret_deny_result(Some(0), "warning: cargo-deny cache not found\n");
    let err = result.expect_err("a non-JSON stream at exit 0 must not score as clean");
    let msg = err.to_string();
    assert!(
        msg.contains("no decodable JSON envelopes"),
        "error must name the zero-envelope refusal; got: {msg}"
    );
}

/// The zero-envelope guard must not fire on a stream that *is* the JSON
/// contract: `log` / `summary` envelopes are not findings, but they prove
/// the output mode, so an exit-0 run carrying only those stays `Ok`.
#[test]
fn interpret_deny_result_accepts_log_envelopes_on_exit_code_0() {
    let stderr = r#"{"type":"log","fields":{"level":"warn","message":"no advisory database found"}}
{"type":"summary","fields":{"errors":0,"warnings":1}}"#;
    let result = interpret_deny_result(Some(0), stderr).expect("envelope-carrying run is Ok");
    assert!(result.advisories.is_empty());
}

/// Exit 0 with *decodable* warning-level findings —
/// the dominant `ops deps` shape (`multiple-versions`, `unmaintained`,
/// `yanked` at `warn` all exit 0) — must parse and stay `Ok`.
#[test]
fn interpret_deny_result_parses_warnings_on_exit_code_0() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"warning","message":"duplicate","code":"duplicate","graphs":[{"Krate":{"name":"serde_yaml","version":"0.9.0"}}]}}
{"type":"diagnostic","fields":{"severity":"warning","message":"crate is unmaintained","code":"unmaintained","advisory":{"id":"RUSTSEC-2024-0375","package":"atty","title":"`atty` is unmaintained"},"graphs":[{"Krate":{"name":"atty","version":"0.2.14"}}]}}"#;

    let result = interpret_deny_result(Some(0), stderr).expect("warning-level run is Ok");
    assert_eq!(result.bans.len(), 1);
    assert_eq!(result.advisories.len(), 1);
}

/// The partial-decode-loss guard fires on exit 0 too. cargo-deny exits 0
/// whenever every finding is at warning level, so an exit-0 run routinely
/// carries a full diagnostic stream; skipping the guard there would leave
/// the dominant path with no schema-drift protection, letting a per-code
/// schema change drop rows while the report stayed green.
#[test]
fn interpret_deny_result_errs_on_partial_decode_loss_on_exit_code_0() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"warning","message":"duplicate","code":"duplicate","graphs":[{"Krate":{"name":"baz","version":"2.0.0"}}]}}
{"type":"diagnostic","fields":{"severity":"warning","message":"vuln","code":"security-vulnerability","advisory":{"id":"RUSTSEC-2024-0001","package":"a","title":"t"}}}
{"type":"diagnostic","fields":{"severity":"warning","message":"vuln","code":"security-vulnerability","advisory":{"id":"RUSTSEC-2024-0002","package":"b","title":"t"}}}
{"type":"diagnostic","fields":{"severity":"warning","message":"vuln","code":"security-vulnerability","advisory":{"id":"RUSTSEC-2024-0003","package":"c","title":"t"}}}"#;

    let err = interpret_deny_result(Some(0), stderr)
        .expect_err("a class-wide decode loss on an exit-0 run must not pass the subset through");
    let msg = err.to_string();
    assert!(
        msg.contains("4 diagnostic line(s)") && msg.contains("3 dropped"),
        "error must report the seen/decoded counts; got: {msg}"
    );
}

#[test]
fn interpret_deny_result_parses_diagnostics_on_exit_code_1() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"`atty` is unmaintained","code":"unmaintained","advisory":{"id":"RUSTSEC-2024-0375","package":"atty","title":"`atty` is unmaintained"},"graphs":[{"Krate":{"name":"atty","version":"0.2.14"},"parents":[]}]}}"#;
    let result = interpret_deny_result(Some(1), stderr).expect("issues run is Ok");
    assert_eq!(result.advisories.len(), 1);
    assert_eq!(result.advisories[0].id, "RUSTSEC-2024-0375");
}

// -- resolve_package is a read, not a consume --

/// Resolving the package twice — or resolving and then re-reading the
/// advisory / graph the answer came from — yields the same name, because
/// `resolve_package` borrows immutably. A version that hollowed out what it
/// read would answer `<no package>` on the second call for a diagnostic that
/// has one, and log a false "no package name" breadcrumb to match.
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
        labels: None,
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
        labels: None,
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

// -- partial decode loss --

/// The zero-diagnostics check catches only *total* decode failure, so a
/// per-code schema change that takes out one class needs the share-based
/// guard: every advisory falls into `classify_code`'s `None` arm, a single
/// unrelated ban still decodes, `is_empty()` stays false on that one vector,
/// and `ops deps` would otherwise render "Advisories: None" in green while
/// an unpatched RUSTSEC advisory sits in the tree.
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
    // Distinguishable from the zero-diagnostics and empty-stderr cases,
    // both of which describe a stream with nothing in it.
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

/// A diagnostic envelope whose `fields` no longer match `DiagnosticFields`
/// is still cargo-deny claiming a finding, so it counts as a candidate and
/// therefore as a *drop*. This is what the two-stage decode buys: decoding
/// the line in one step would send it to the malformed-JSON arm, which
/// counts nothing, so three broken advisories plus one surviving ban would
/// read as `candidates == 1, dropped == 0` and the gate would stay green
/// with an entire class missing.
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

/// The missing-`code` drop path leaves a tracing breadcrumb, like every
/// other drop path in this parser.
#[test]
#[serial_test::serial]
fn parse_deny_missing_code_logs_a_breadcrumb() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"something","graphs":[{"Krate":{"name":"pkg","version":"1.0.0"}}]}}"#;

    let (logged, result) =
        crate::test_support::capture_tracing(tracing::Level::DEBUG, || parse_deny_output(stderr));
    assert!(result.advisories.is_empty());
    assert!(
        logged.contains("no `code` field"),
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

/// A diagnostic whose code is not in any of the four known sets
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

/// A diagnostic that lacks a `severity` field keeps that fact visible: the
/// severity becomes the `<missing-severity>` sentinel, which classifies as
/// `SeverityClass::Unknown` and so still fails the `has_issues` gate, while
/// staying distinguishable from a real cargo-deny `error` in the parsed
/// entry and in the logs.
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

/// A diagnostic with no severity must NOT be rebadged as `"error"` — that
/// collides with cargo-deny's legitimate "error" value and stops callers
/// (and operators reading logs) from telling "real error" apart from "schema
/// drift, severity field gone". `<missing-severity>` is a distinct sentinel
/// that still fails the gate.
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
    // The missing-package sentinel must be visibly
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

/// cargo-deny 0.20.2 emits one `license-not-encountered` warning per
/// `[licenses] allow` entry no dependency uses. The diagnostic is about the
/// config, not a crate (`graphs: []`), so its subject is the license in
/// `labels[0].span`. It must decode as a license warning rather than drop.
const LICENSE_NOT_ENCOUNTERED: &str = r#"{"type":"diagnostic","fields":{"code":"license-not-encountered","graphs":[],"labels":[{"column":6,"line":24,"message":"unmatched license allowance","span":"0BSD"}],"message":"license was not encountered","notes":[],"severity":"warning"}}"#;

#[test]
fn parse_deny_license_not_encountered_is_an_unused_allowance_warning() {
    let result = parse_deny_output(LICENSE_NOT_ENCOUNTERED);
    assert!(result.licenses.is_empty(), "not a finding about a crate");
    let unused = &result.unused_license_allowances;
    assert_eq!(unused.len(), 1);
    assert_eq!(unused[0].package, "0BSD");
    assert_eq!(unused[0].severity, "warning");
    assert_eq!(unused[0].message, "license was not encountered");
}

/// A shared allow-list baseline makes these the whole stream; before they
/// were classified, every line dropped and the partial-decode-loss guard
/// failed `ops deps --check` on an otherwise clean run.
#[test]
fn interpret_deny_result_accepts_a_stream_of_unused_license_allowances() {
    let second = LICENSE_NOT_ENCOUNTERED.replace("0BSD", "Zlib");
    let stderr = format!(
        "{LICENSE_NOT_ENCOUNTERED}\n{second}\n{}",
        r#"{"type":"summary","fields":{"errors":0,"warnings":2}}"#
    );
    let result = interpret_deny_result(Some(0), &stderr).expect("unused allowances are warnings");
    let subjects: Vec<&str> = result
        .unused_license_allowances
        .iter()
        .map(|l| l.package.as_str())
        .collect();
    assert_eq!(subjects, ["0BSD", "Zlib"]);
}

/// The label fallback is limited to config-level codes: a crate diagnostic
/// with no crate in it keeps the `<no package>` sentinel rather than borrowing
/// a license string as its package name.
#[test]
fn parse_deny_label_span_is_not_a_package_for_crate_diagnostics() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"rejected","code":"rejected","labels":[{"span":"GPL-3.0"}],"graphs":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.licenses[0].package, "<no package>");
}

/// Undecodable diagnostics still fail closed: classifying the unused-allowance
/// code must not loosen the guard for codes nobody recognises.
#[test]
fn interpret_deny_result_still_fails_closed_on_unknown_codes_beside_unused_allowances() {
    let unknown = r#"{"type":"diagnostic","fields":{"severity":"error","message":"m","code":"brand-new-code","graphs":[]}}"#;
    let stderr = format!("{LICENSE_NOT_ENCOUNTERED}\n{unknown}");
    let err = interpret_deny_result(Some(1), &stderr).expect_err("1 of 2 dropped must fail closed");
    assert!(err.to_string().contains("1 dropped"), "got: {err}");
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
    assert!(
        logged.contains("malformed cargo-deny JSON line"),
        "missing malformed-line message: {logged}"
    );
}

// -- stderr tail Debug-escapes control bytes --

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

/// The same escape contract on an exit-2 usage error.
#[test]
fn interpret_deny_result_exit_two_debug_escapes_stderr_tail() {
    let stderr = "error: \x1b[31minvalid TOML\x1b[0m\nbye\n";
    let result = interpret_deny_result(Some(2), stderr);
    let err = result.expect_err("usage error must surface");
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
