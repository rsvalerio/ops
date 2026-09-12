//! `query_metadata_raw` payload-cap and singleton invariant tests.
//!
//! ARCH-1 / TASK-1545: split out from the legacy `tests.rs`.
//!
//! SQLite port note: `metadata_raw` is now a one-row `(json TEXT NOT NULL)`
//! blob table. The fixtures below seed it directly, bypassing the ingestor,
//! so the reader-side invariants (singleton, cap) are pinned independently of
//! the ingest path.

use crate::{
    query_metadata_raw, query_metadata_raw_with_cap, METADATA_MAX_BYTES_DEFAULT,
    METADATA_MAX_BYTES_ENV,
};

/// Seed `metadata_raw` with verbatim JSON blob rows.
fn seed_raw(db: &ops_sqlite::Sqlite, rows: &[&str]) {
    let conn = db.lock().expect("lock");
    conn.execute_batch("CREATE TABLE metadata_raw (json TEXT NOT NULL)")
        .expect("create");
    for row in rows {
        conn.execute("INSERT INTO metadata_raw VALUES (?1)", [row])
            .expect("seed row");
    }
    drop(conn);
}

/// ERR-1 / TASK-0599: `metadata_raw` is a singleton invariant. If a
/// future ingest path (re-collect without truncate, schema-version row)
/// inserts more than one row, `query_metadata_raw` must surface a
/// clear error rather than silently picking an arbitrary row via
/// `LIMIT 1`.
#[test]
fn query_metadata_raw_errors_on_multiple_rows() {
    let db = ops_sqlite::Sqlite::open_in_memory().expect("open in-memory");
    seed_raw(
        &db,
        &[r#"{"workspace_root":"/a"}"#, r#"{"workspace_root":"/b"}"#],
    );
    let err = query_metadata_raw(&db).expect_err("multi-row must fail");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("exactly one row") || msg.contains("found 2"),
        "got: {msg}"
    );
}

/// ERR-1 / TASK-0599: companion to the multi-row test — single-row
/// `metadata_raw` flows through unchanged.
#[test]
fn query_metadata_raw_succeeds_on_single_row() {
    let db = ops_sqlite::Sqlite::open_in_memory().expect("open in-memory");
    seed_raw(&db, &[r#"{"workspace_root":"/a"}"#]);
    let v = query_metadata_raw(&db).expect("single-row must succeed");
    assert_eq!(v["workspace_root"], "/a");
}

/// ERR-1 / TASK-1034: oversized payloads must fail fast with a
/// clear error rather than risking an OOM in `ops about`. The cap
/// is configurable via `OPS_METADATA_MAX_BYTES`; this test drives
/// the cap directly to avoid mutating process-global env.
#[test]
fn query_metadata_raw_errors_when_payload_exceeds_cap() {
    let db = ops_sqlite::Sqlite::open_in_memory().expect("open in-memory");
    seed_raw(
        &db,
        &[r#"{"workspace_root":"/workspace","pad":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#],
    );
    let err = query_metadata_raw_with_cap(&db, 32).expect_err("oversized must fail");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("exceeds") && msg.contains("byte cap"),
        "got: {msg}"
    );
    assert!(msg.contains(METADATA_MAX_BYTES_ENV), "got: {msg}");
}

/// SEC-33 / TASK-1194 (TEST-11 / TASK-2194): the cap must fire **inside the
/// SQL**, before the payload is materialised into a Rust `String`. The
/// mechanism — and the property that distinguishes this implementation from
/// a pre-check materialise-then-check shape — is `CAP_GUARD_SQL`'s `CASE`: an
/// over-cap row crosses the FFI boundary with `payload = NULL`, so the
/// oversized text never becomes a Rust allocation.
///
/// This test asserts the guard's observable behaviour with a small fixture:
/// over cap → `payload` is NULL and `bytes` is the exact payload length;
/// under cap → the payload arrives intact. The 100-MiB fixture the previous
/// version materialised on every run was removed (TASK-2194 AC #2) — the
/// NULL shape does not depend on the payload's size, only on the CASE branch.
#[test]
fn cap_guard_sql_nulls_the_payload_over_cap_before_it_crosses_ffi() {
    let db = ops_sqlite::Sqlite::open_in_memory().expect("open in-memory");
    let over = format!(r#"{{"pad":"{}"}}"#, "a".repeat(64));
    let under = r#"{"payload":"ok"}"#;
    seed_raw(&db, &[over.as_str(), under]);
    let conn = db.lock().expect("lock");
    // Cap 40 sits between the two rows' byte lengths.
    let sql = crate::CAP_GUARD_SQL.replace('?', "40");
    let rows: Vec<(i64, Option<String>)> = {
        let mut stmt = conn.prepare(&sql).expect("prepare cap-guard SQL");
        let mapped = stmt
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
            })
            .expect("query cap-guard SQL");
        mapped.collect::<Result<Vec<_>, _>>().expect("collect rows")
    };
    drop(conn);

    // Over-cap row: the payload must be NULL — a materialise-then-check
    // implementation cannot produce this shape, and the error it renders is
    // byte-identical (see doc comment).
    let over_row = rows
        .iter()
        .find(|(len, _)| *len > 40)
        .expect("over-cap row present");
    assert!(
        over_row.1.is_none(),
        "over-cap payload must be NULL at the FFI boundary, got: {:?}",
        over_row.1
    );
    // The byte count is exact (blob-length semantics), not a char count.
    assert_eq!(
        over_row.0,
        i64::try_from(over.len()).expect("len fits i64"),
        "bytes must be the payload's exact byte length"
    );

    // Under-cap row: the payload arrives intact (the CASE's ELSE branch).
    let under_row = rows
        .iter()
        .find(|(len, _)| *len <= 40)
        .expect("under-cap row present");
    let text = under_row
        .1
        .as_deref()
        .expect("under-cap payload must not be nulled");
    assert!(
        text.contains(r#""payload":"ok""#) || text.contains(r#""payload": "ok""#),
        "under-cap payload must carry the row's JSON text, got: {text}"
    );
}

/// ERR-1 / TASK-1034: payloads at or under the cap parse normally.
#[test]
fn query_metadata_raw_succeeds_when_payload_within_cap() {
    let db = ops_sqlite::Sqlite::open_in_memory().expect("open in-memory");
    seed_raw(&db, &[r#"{"workspace_root":"/workspace"}"#]);
    let v = query_metadata_raw_with_cap(&db, METADATA_MAX_BYTES_DEFAULT)
        .expect("under-cap payload should parse");
    assert_eq!(v["workspace_root"], "/workspace");
}

/// SEC-11 / TASK-1897: `OPS_METADATA_MAX_BYTES` validation and clamping.
/// Driven through the injectable [`crate::resolve_metadata_max_bytes`] seam
/// rather than by mutating process-global env, which the `OnceLock` snapshot
/// in `metadata_max_bytes` makes untestable anyway (one initialisation per
/// process).
mod max_bytes_env {
    use crate::{
        resolve_metadata_max_bytes, METADATA_MAX_BYTES_CEILING, METADATA_MAX_BYTES_DEFAULT,
        METADATA_MAX_BYTES_ENV,
    };
    use ops_about::test_support::capture_tracing;

    /// Resolve `raw` while capturing WARN-level output, so each rejected
    /// value can be checked for the diagnostic AC #1 requires.
    fn resolve_capturing_warns(raw: Option<&str>) -> (u64, String) {
        let (logs, resolved) =
            capture_tracing(tracing::Level::WARN, || resolve_metadata_max_bytes(raw));
        (resolved, logs)
    }

    #[test]
    fn unset_env_resolves_to_default_without_warning() {
        let (resolved, logs) = resolve_capturing_warns(None);
        assert_eq!(resolved, METADATA_MAX_BYTES_DEFAULT);
        assert!(
            logs.is_empty(),
            "an unset knob is not a misconfiguration: {logs}"
        );
    }

    #[test]
    fn valid_value_is_honoured_verbatim_without_warning() {
        let (resolved, logs) = resolve_capturing_warns(Some("1048576"));
        assert_eq!(resolved, 1_048_576);
        assert!(logs.is_empty(), "an accepted value must not warn: {logs}");
    }

    #[test]
    fn surrounding_whitespace_is_tolerated() {
        let (resolved, logs) = resolve_capturing_warns(Some(" 1048576\n"));
        assert_eq!(resolved, 1_048_576);
        assert!(
            logs.is_empty(),
            "a trailing newline must not silently drop the knob: {logs}"
        );
    }

    #[test]
    fn malformed_value_warns_and_falls_back() {
        let (resolved, logs) = resolve_capturing_warns(Some("64MB"));
        assert_eq!(resolved, METADATA_MAX_BYTES_DEFAULT);
        assert!(
            logs.contains(METADATA_MAX_BYTES_ENV),
            "warn must name the variable: {logs}"
        );
        assert!(
            logs.contains("64MB"),
            "warn must name the offending value: {logs}"
        );
    }

    #[test]
    fn negative_value_warns_and_falls_back() {
        let (resolved, logs) = resolve_capturing_warns(Some("-1"));
        assert_eq!(resolved, METADATA_MAX_BYTES_DEFAULT);
        assert!(
            logs.contains(METADATA_MAX_BYTES_ENV),
            "warn must name the variable: {logs}"
        );
        assert!(
            logs.contains("-1"),
            "warn must name the offending value: {logs}"
        );
    }

    #[test]
    fn zero_warns_and_falls_back() {
        let (resolved, logs) = resolve_capturing_warns(Some("0"));
        assert_eq!(resolved, METADATA_MAX_BYTES_DEFAULT);
        assert!(
            logs.contains(METADATA_MAX_BYTES_ENV),
            "warn must name the variable: {logs}"
        );
        assert!(
            logs.contains("zero byte cap"),
            "warn must explain the rejection: {logs}"
        );
    }

    #[test]
    fn above_ceiling_warns_and_clamps() {
        let (resolved, logs) = resolve_capturing_warns(Some("18446744073709551615"));
        assert_eq!(
            resolved, METADATA_MAX_BYTES_CEILING,
            "an unbounded knob would silently disable the SEC-33 guard"
        );
        assert!(
            logs.contains(METADATA_MAX_BYTES_ENV),
            "warn must name the variable: {logs}"
        );
        assert!(
            logs.contains("clamping"),
            "warn must say the value was clamped: {logs}"
        );
    }

    #[test]
    fn ceiling_is_exactly_u32_max() {
        assert_eq!(METADATA_MAX_BYTES_CEILING, u64::from(u32::MAX));
    }

    // SQLite port note: the former `resolved_ceiling_is_accepted_by_sqlite_
    // read_json` test pinned that every resolver output fit the engine-side
    // `maximum_object_size` option's UINTEGER domain. That option (and the
    // `read_json_auto` statement it threaded through) no longer exists: the
    // ingest-side cap is a Rust `str::len()` comparison and the read-side cap
    // binds an i64 parameter, neither of which has a narrower domain to
    // overflow. The test was not reinstated for the same reason the
    // `to_json`-serialisation plan test was removed: the cost it pinned is
    // gone with the code path.
}

// TEST-1 / TASK-1901: the former `metadata_max_bytes_is_memoised` test
// (`tests/accessors.rs`, cited as "PERF-3 / TASK-1248 AC #3") asserted that
// two consecutive `metadata_max_bytes()` calls return the same value. A
// deterministic parse of a process-global env var returns the same value
// with or without the `OnceLock`, so deleting the cache left the test green
// — a tautology, the same shape removed from `ingestor.rs` by TASK-1546. It
// went with `tests/accessors.rs` in TASK-1898 and is deliberately not
// reinstated: the snapshot property it should have pinned is unobservable
// without mutating process-global env, and everything worth asserting about
// the cap now lives in `max_bytes_env` above, which drives the same
// resolver through an injectable seam.
