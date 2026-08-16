//! Tests for `interpret_upgrade_output`: which `(exit code, stdout, stderr)`
//! triples may be parsed as authoritative, and which must fail closed.

use super::*;

// -- Upgrade exit-code interpretation (ERR-1 / TASK-0913) --

/// ERR-1 / TASK-0913: `cargo upgrade --dry-run` exit 1 (lockfile contention,
/// network error, etc.) must surface as an error rather than parsing an
/// empty stdout into an empty `UpgradeResult`. Mirrors the cargo-deny exit-1
/// fix in TASK-0612.
#[test]
fn interpret_upgrade_output_errs_on_exit_one() {
    let stderr = b"error: failed to update registry: connection timed out\n";
    let result = crate::parse::interpret_upgrade_output(Some(1), b"", stderr);
    let err = result.expect_err("non-zero exit must surface");
    let msg = err.to_string();
    assert!(msg.contains("status 1"), "expected exit-1 in error: {msg}");
    assert!(
        msg.contains("connection timed out"),
        "stderr tail must be preserved: {msg}"
    );
}

/// `cargo upgrade --dry-run` exit 101 (cargo or subtool panic) must
/// surface, not silently render as a clean upgrade report.
#[test]
fn interpret_upgrade_output_errs_on_exit_one_oh_one() {
    let stderr = b"thread 'main' panicked at 'assertion failed: ...'\n";
    let result = crate::parse::interpret_upgrade_output(Some(101), b"", stderr);
    let err = result.expect_err("panic exit must surface");
    let msg = err.to_string();
    assert!(
        msg.contains("status 101"),
        "expected exit-101 in error: {msg}"
    );
    assert!(
        msg.contains("panicked"),
        "stderr tail must be preserved: {msg}"
    );
}

#[test]
fn interpret_upgrade_output_errs_on_signal_kill() {
    let result = crate::parse::interpret_upgrade_output(None, b"", b"");
    let err = result.expect_err("None exit must surface");
    assert!(
        err.to_string().contains("signal"),
        "error must name signal-kill case, got: {err}"
    );
}

#[test]
fn interpret_upgrade_output_parses_on_clean_exit() {
    let stdout = b"name   old req compatible latest  new req note\n\
                   ====   ======= ========== ======  ======= ====\n\
                   serde  1.0.100 1.0.228    1.0.228 1.0.228\n";
    let result = crate::parse::interpret_upgrade_output(Some(0), stdout, b"")
        .expect("clean exit must parse");
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "serde");
}

/// PATTERN-1 / TASK-1074: when cargo-edit emits a `====` separator row but
/// the header line above it has been renamed (e.g. `Package` / `Current Req`
/// instead of `name` / `old req`), the parser must NOT silently score the
/// run as authoritative — even though the separator alone would still align
/// columns and yield rows. `interpret_upgrade_output` must emit a warn and
/// bail so the supply-chain gate fails loudly on cargo-edit format drift.
#[test]
fn interpret_upgrade_output_bails_on_unrecognised_header_with_separator() {
    // Renamed header columns ("Package" / "Current Req" / "Available") with
    // a real `====` separator and one body row. Pre-fix this would silently
    // parse and return 1 entry; post-fix it bails with a TASK-1074 warn.
    let stdout = b"Package Current Req Available Latest  Pinned Note\n\
                   ======= =========== ========= ======  ====== ====\n\
                   serde   1.0.100     1.0.228   1.0.228 1.0.228\n";

    let (result, logged) =
        crate::test_support::with_captured_logs(tracing::Level::WARN, false, || {
            crate::parse::interpret_upgrade_output(Some(0), stdout, b"")
        });

    let err = result
        .expect_err("renamed header with separator must bail, not silently parse as authoritative");
    let msg = err.to_string();
    assert!(
        msg.contains("header") && (msg.contains("not recognised") || msg.contains("drift")),
        "error must call out header-drift; got: {msg}"
    );
    assert!(
        logged.contains("TASK-1074") && logged.contains("header"),
        "expected a TASK-1074 header-drift warn; got: {logged}"
    );
}

/// ERR-1 / TASK-1203: a second header-shaped line appearing between the
/// separator and a body row must NOT re-arm `columns` to None. Pre-fix the
/// post-second-header rows were dropped silently.
#[test]
fn parse_upgrade_table_repeat_header_keeps_columns() {
    let stdout = "\
name   old req compatible latest  new req
====   ======= ========== ======  =======
serde  1.0.100 1.0.228    1.0.228 1.0.228
name   old req compatible latest  new req
tokio  1.35.0  1.38.0     1.38.0  1.38.0
";
    let entries = parse_upgrade_table(stdout);
    assert_eq!(
        entries.len(),
        2,
        "second header must not drop subsequent rows; got: {entries:?}"
    );
    assert_eq!(entries[0].name, "serde");
    assert_eq!(entries[1].name, "tokio");
}

/// ERR-1 / TASK-1202: if cargo-edit emits a recognised header and a `====`
/// separator with body rows but every row fails the 5-column shape check
/// (wholesale row-shape drift), `interpret_upgrade_output` must bail rather
/// than silently scoring the run as "no upgrades available".
#[test]
fn interpret_upgrade_output_bails_on_row_shape_drift() {
    // Recognised header, real separator, but every body row only fills 3
    // columns — pre-fix: parse_upgrade_row drops them at debug, returns Ok([]).
    let stdout = b"name   old req compatible latest  new req\n\
                   ====   ======= ========== ======  =======\n\
                   serde  1.0.100 1.0.228\n\
                   tokio  1.35.0  1.38.0\n";

    let result = crate::parse::interpret_upgrade_output(Some(0), stdout, b"");
    let err = result.expect_err("row-shape drift must bail, not silently parse as authoritative");
    let msg = err.to_string();
    assert!(
        msg.contains("row-shape") || msg.contains("body row"),
        "error must call out row-shape drift; got: {msg}"
    );
}

/// TASK-1492: preamble lines before the header must not feed the `body_lines`
/// counter. A recognised header + separator with zero real body rows must
/// return Ok([]), not bail with row-shape-drift.
#[test]
fn interpret_upgrade_output_preamble_does_not_inflate_body_lines() {
    let stdout = b"Updating crates.io index\n\
                   Some other preamble line\n\
                   name   old req compatible latest  new req\n\
                   ====   ======= ========== ======  =======\n";
    let result = crate::parse::interpret_upgrade_output(Some(0), stdout, b"")
        .expect("preamble + header + separator + zero body rows must be Ok");
    assert!(result.is_empty());
}

// -- ERR-7 / SEC-21 / TASK-1160: stderr tail Debug-escapes control bytes --

/// `interpret_upgrade_output` and `interpret_deny_result` must format the
/// stderr tail through the `?` formatter so embedded ANSI / newlines /
/// NULs from cargo-edit / cargo-deny cannot forge log records or repaint
/// the operator terminal. Pin the value-level escape on the unrecognised
/// upgrade-exit arm; the `interpret_deny_result` arms are pinned in
/// `deny/tests.rs`.
#[test]
fn interpret_upgrade_output_unrecognised_exit_debug_escapes_stderr_tail() {
    let stderr = b"warn\nerror: \x1b[31mhi\x1b[0m\nbye\n";
    let result = crate::parse::interpret_upgrade_output(Some(7), b"", stderr);
    let err = result.expect_err("unrecognised exit must surface");
    let msg = err.to_string();
    assert!(
        !msg.contains('\u{1b}'),
        "ANSI ESC must not survive in: {msg:?}"
    );
    assert!(
        msg.contains("\\n") || !msg.contains('\n'),
        "stderr newlines must be escaped or stripped: {msg:?}"
    );
}
