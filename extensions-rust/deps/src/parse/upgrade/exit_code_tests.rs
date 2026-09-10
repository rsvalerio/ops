//! Tests for `interpret_upgrade_output`: which `(exit code, stdout, stderr)`
//! triples may be parsed as authoritative, and which must fail closed.

use super::*;

// -- Upgrade exit-code interpretation --

/// `cargo upgrade --dry-run` exit 1 (lockfile contention, network error,
/// etc.) surfaces as an error rather than parsing an empty stdout into an
/// empty `UpgradeResult` — the same posture as the cargo-deny exit-1 arm.
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

/// When cargo-edit emits a `====` separator row but the header line above it
/// has been renamed (e.g. `Package` / `Current Req` instead of `name` /
/// `old req`), the run must NOT be scored as authoritative — even though the
/// separator alone would still align columns and yield rows.
/// `interpret_upgrade_output` warns and bails, so the supply-chain gate
/// fails loudly on cargo-edit format drift.
#[test]
fn interpret_upgrade_output_bails_on_unrecognised_header_with_separator() {
    // Renamed header columns ("Package" / "Current Req" / "Available") with
    // a real `====` separator and one body row: alignable, but not a table
    // this parser recognises.
    let stdout = b"Package Current Req Available Latest  Pinned Note\n\
                   ======= =========== ========= ======  ====== ====\n\
                   serde   1.0.100     1.0.228   1.0.228 1.0.228\n";

    let (logged, result) = crate::test_support::capture_tracing(tracing::Level::WARN, || {
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

/// A second header-shaped line appearing between the separator and a body
/// row must NOT re-arm `columns` to `None`, which would drop every row after
/// it silently.
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

/// If cargo-edit emits a recognised header and a `====` separator with body
/// rows but every row fails the 5-column shape check (wholesale row-shape
/// drift), `interpret_upgrade_output` bails rather than silently scoring the
/// run as "no upgrades available".
#[test]
fn interpret_upgrade_output_bails_on_row_shape_drift() {
    // Recognised header, real separator, but every body row only fills 3
    // columns, so `parse_upgrade_row` drops all of them.
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

/// The *partial* permutation of row-shape drift: most rows failing while one
/// parses. An all-or-nothing check would see `entries_emitted > 0` and stay
/// silent, so `ops deps` would report one available upgrade where there are
/// four, on a green report.
#[test]
fn interpret_upgrade_output_bails_on_partial_row_loss() {
    let stdout = b"name   old req compatible latest  new req\n\
                   ====   ======= ========== ======  =======\n\
                   serde  1.0.100 1.0.228    1.0.228 1.0.228\n\
                   bad-a  1.0.0   1.0.1\n\
                   bad-b  2.0.0   2.0.1\n\
                   bad-c  3.0.0   3.0.1\n";

    let result = crate::parse::interpret_upgrade_output(Some(0), stdout, b"");
    let err = result.expect_err("partial row loss must bail, not return the survivor");
    let msg = err.to_string();
    assert!(
        msg.contains("4 body row(s)") && msg.contains("1 filled") && msg.contains("3 dropped"),
        "error must report the seen/parsed counts; got: {msg}"
    );
}

/// The tolerance is a *share*, not zero — one dropped
/// row among four or more is ordinary forward drift (a note or footer line
/// that never filled five columns) and must keep the run `Ok`.
#[test]
fn interpret_upgrade_output_tolerates_one_dropped_row_among_many() {
    let stdout = b"name   old req compatible latest  new req\n\
                   ====   ======= ========== ======  =======\n\
                   serde  1.0.100 1.0.228    1.0.228 1.0.228\n\
                   anyio  1.0.0   1.0.1\n\
                   tokio  1.35.0  1.38.0     1.38.0  1.38.0\n\
                   clap   3.0.0   3.2.25     4.6.0   3.2.25\n";

    let result = crate::parse::interpret_upgrade_output(Some(0), stdout, b"")
        .expect("1-in-4 dropped rows must stay tolerated");
    assert_eq!(result.len(), 3);
}

/// Preamble lines before the header must not feed the `body_lines`
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

/// `check_header_drift` and `check_row_shape_drift` are both gated on
/// `saw_separator`, so output carrying a *recognised* header plus body rows
/// but no `====` separator would escape both: no row sliced, `Ok(vec![])`
/// returned, and `ops deps` rendering "no upgrades" on a zero exit.
/// `check_missing_separator_drift` bails on it instead.
#[test]
fn interpret_upgrade_output_bails_on_missing_separator() {
    // Recognised header, real body rows, separator row dropped entirely
    // (e.g. cargo-edit switching to a box-drawing or ANSI-styled table).
    let stdout = b"name   old req compatible latest  new req\n\
                   serde  1.0.100 1.0.228    1.0.228 1.0.228\n\
                   tokio  1.35.0  1.38.0     1.38.0  1.38.0\n";

    let (logged, result) = crate::test_support::capture_tracing(tracing::Level::WARN, || {
        crate::parse::interpret_upgrade_output(Some(0), stdout, b"")
    });

    let err = result.expect_err("missing separator must bail, not score as `no upgrades`");
    let msg = err.to_string();
    assert!(
        msg.contains("separator"),
        "error must name the missing-separator case; got: {msg}"
    );
    // Distinguishable from the header-drift and row-shape-drift messages,
    // both of which describe a table that *could* be aligned.
    assert!(
        !msg.contains("header line was not recognised") && !msg.contains("5 fixed columns"),
        "missing-separator message must not read as header or row-shape drift; got: {msg}"
    );
    // The separator-drift breadcrumb stays observable in logs.
    assert!(
        logged.contains("TASK-1026") && logged.contains("separator"),
        "expected the TASK-1026 separator-drift warn to survive; got: {logged}"
    );
}

/// The missing-separator guard must not fire on genuinely empty output —
/// `cargo upgrade --dry-run` printing nothing is "no upgrades", not drift.
#[test]
fn interpret_upgrade_output_empty_stdout_is_not_separator_drift() {
    let result = crate::parse::interpret_upgrade_output(Some(0), b"", b"")
        .expect("empty stdout must stay Ok([])");
    assert!(result.is_empty());
}

// -- stderr tail Debug-escapes control bytes --

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
