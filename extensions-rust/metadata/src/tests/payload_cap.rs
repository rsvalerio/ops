//! `query_metadata_raw` payload-cap and singleton invariant tests.
//!
//! ARCH-1 / TASK-1545: split out from the legacy `tests.rs`.

use crate::{
    query_metadata_raw, query_metadata_raw_with_cap, METADATA_MAX_BYTES_DEFAULT,
    METADATA_MAX_BYTES_ENV,
};

/// ERR-1 / TASK-0599: `metadata_raw` is a singleton invariant. If a
/// future ingest path (re-collect without truncate, schema-version row)
/// inserts more than one row, `query_metadata_raw` must surface a
/// clear error rather than silently picking an arbitrary row via
/// `LIMIT 1`.
#[test]
fn query_metadata_raw_errors_on_multiple_rows() {
    let db = ops_duckdb::DuckDb::open_in_memory().expect("open in-memory");
    {
        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE metadata_raw (workspace_root VARCHAR, payload INTEGER);
             INSERT INTO metadata_raw VALUES ('/a', 1), ('/b', 2);",
        )
        .expect("seed");
    }
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
    let db = ops_duckdb::DuckDb::open_in_memory().expect("open in-memory");
    {
        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE metadata_raw (workspace_root VARCHAR, payload INTEGER);
             INSERT INTO metadata_raw VALUES ('/a', 1);",
        )
        .expect("seed");
    }
    let v = query_metadata_raw(&db).expect("single-row must succeed");
    assert_eq!(v["workspace_root"], "/a");
}

/// ERR-1 / TASK-1034: oversized payloads must fail fast with a
/// clear error rather than risking an OOM in `ops about`. The cap
/// is configurable via `OPS_METADATA_MAX_BYTES`; this test drives
/// the cap directly to avoid mutating process-global env.
#[test]
fn query_metadata_raw_errors_when_payload_exceeds_cap() {
    let db = ops_duckdb::DuckDb::open_in_memory().expect("open in-memory");
    {
        let conn = db.lock().expect("lock");
        // A row whose to_json serialisation comfortably exceeds 32 bytes.
        conn.execute_batch(
            "CREATE TABLE metadata_raw (workspace_root VARCHAR, payload VARCHAR);
             INSERT INTO metadata_raw VALUES \
             ('/workspace', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa');",
        )
        .expect("seed");
    }
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
/// the pre-TASK-1194 materialise-then-check shape — is `CAP_GUARD_SQL`'s
/// `CASE`: an over-cap row crosses the FFI boundary with `payload = NULL`,
/// so the oversized text never becomes a Rust allocation. The pre-fix shape
/// had no such guard: it pulled the full `String` across first and reported
/// the *same error text*, which is why the error-message assertions alone
/// (kept in `query_metadata_raw_errors_when_payload_exceeds_cap` above)
/// certified nothing.
///
/// This test asserts the guard's observable behaviour with a small fixture:
/// over cap → `payload` is NULL and `bytes` is the exact length; under cap →
/// the payload arrives intact. The 100-MiB fixture the previous version
/// materialised on every run was removed (TASK-2194 AC #2) — the NULL shape
/// does not depend on the payload's size, only on the CASE branch. The
/// single-serialisation cost claim for the same SQL is pinned separately by
/// `cap_guard_sql_serialises_to_json_once` below.
#[test]
fn cap_guard_sql_nulls_the_payload_over_cap_before_it_crosses_ffi() {
    let db = ops_duckdb::DuckDb::open_in_memory().expect("open in-memory");
    {
        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE metadata_raw (workspace_root VARCHAR, payload VARCHAR);
             INSERT INTO metadata_raw VALUES \
             ('/over', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'), \
             ('/under', 'ok');",
        )
        .expect("seed");
    }
    let conn = db.lock().expect("lock");
    // Cap 64 sits between the two rows' serialized sizes: the under-cap
    // row's `to_json(m)` text (~39 bytes) and the over-cap row's (95 bytes,
    // 58 payload bytes + the JSON envelope).
    let sql = crate::CAP_GUARD_SQL.replace('?', "64");
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

    // Over-cap row (58 payload bytes → 95 serialized): the payload must be
    // NULL — a materialise-then-check implementation cannot produce this
    // shape, and the error it renders is byte-identical (see doc comment).
    let over = rows
        .iter()
        .find(|(len, _)| *len > 64)
        .expect("over-cap row present");
    assert!(
        over.1.is_none(),
        "over-cap payload must be NULL at the FFI boundary, got: {:?}",
        over.1
    );

    // Under-cap row: the payload arrives intact (the CASE's ELSE branch) —
    // the full `to_json(m)` text, envelope included.
    let under = rows
        .iter()
        .find(|(len, _)| *len <= 64)
        .expect("under-cap row present");
    let text = under
        .1
        .as_deref()
        .expect("under-cap payload must not be nulled");
    assert!(
        text.contains("\"payload\": \"ok\"") || text.contains("\"payload\":\"ok\""),
        "under-cap payload must carry the row's JSON text, got: {text}"
    );
}

/// ERR-1 / TASK-1034: payloads at or under the cap parse normally.
#[test]
fn query_metadata_raw_succeeds_when_payload_within_cap() {
    let db = ops_duckdb::DuckDb::open_in_memory().expect("open in-memory");
    {
        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE metadata_raw (workspace_root VARCHAR, payload INTEGER);
             INSERT INTO metadata_raw VALUES ('/workspace', 1);",
        )
        .expect("seed");
    }
    let v = query_metadata_raw_with_cap(&db, METADATA_MAX_BYTES_DEFAULT)
        .expect("under-cap payload should parse");
    assert_eq!(v["workspace_root"], "/workspace");
}

/// READ-1 / TASK-1896 AC #1 + #2: the cap-guard SQL spells
/// `to_json(m)::VARCHAR` three times, and the comment on
/// [`crate::CAP_GUARD_SQL`] claims it is nonetheless serialised once per
/// row. That claim rests on `DuckDB`'s common-subexpression elimination, so
/// pin it against the physical plan: a single `to_json` projection node
/// means one serialisation. If a future `DuckDB` bump stops folding the
/// repetition, this test fails and the cost claim is revisited rather than
/// silently becoming false.
#[test]
fn cap_guard_sql_serialises_to_json_once() {
    let db = ops_duckdb::DuckDb::open_in_memory().expect("open in-memory");
    let conn = db.lock().expect("lock");
    conn.execute_batch(
        "CREATE TABLE metadata_raw (workspace_root VARCHAR, payload VARCHAR);
         INSERT INTO metadata_raw VALUES ('/workspace', 'x');",
    )
    .expect("seed");
    // The bind parameter is irrelevant to the plan shape; inline a literal
    // so `EXPLAIN` needs no parameters.
    let sql = crate::CAP_GUARD_SQL.replace('?', "1000000");
    let plan: String = conn
        .query_row(&format!("EXPLAIN {sql}"), [], |row| row.get(1))
        .expect("explain the cap-guard query");
    drop(conn);
    let serialisations = plan.matches("to_json").count();
    assert_eq!(
        serialisations, 1,
        "cap-guard SQL must serialise to_json once per row; physical plan:\n{plan}"
    );
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
    fn ceiling_is_exactly_duckdb_uinteger_max() {
        assert_eq!(METADATA_MAX_BYTES_CEILING, u64::from(u32::MAX));
    }

    /// SEC-11 / TASK-1897 AC #2: no value the resolver can produce may make
    /// the ingest `CREATE TABLE … read_json_auto(…)` fail on an
    /// option-conversion error. Execute the SQL at the ceiling — the one
    /// value most likely to overflow `DuckDB`'s `UINTEGER` domain — against a
    /// real connection.
    #[test]
    // macOS-impossible: DuckDB's `read_json_auto` fails with `EINVAL` reading
    // from the per-user `/var/folders` temp area (even canonicalized to
    // `/private/var/...`), which is where `tempfile` puts every fixture on a
    // default macOS host. Linux CI executes this against a real connection.
    #[cfg(not(target_os = "macos"))]
    fn resolved_ceiling_is_accepted_by_duckdb_read_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().canonicalize().unwrap().join("metadata.json");
        std::fs::write(&path, br#"{"workspace_root":"/workspace"}"#).expect("seed json");

        let resolved = resolve_metadata_max_bytes(Some("99999999999999"));
        assert_eq!(resolved, METADATA_MAX_BYTES_CEILING);

        let sql = crate::views::metadata_raw_create_sql_with_cap(&path, resolved)
            .expect("sql builds at the ceiling");
        let db = ops_duckdb::DuckDb::open_in_memory().expect("open in-memory");
        let conn = db.lock().expect("lock");
        conn.execute(sql.as_str(), [])
            .expect("DuckDB must accept maximum_object_size at the resolved ceiling");
        drop(conn);
    }
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
