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
/// metadata_raw flows through unchanged.
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

/// SEC-33 / TASK-1194: an oversized payload must fail the byte cap
/// **before** the full text is materialised into a Rust `String`.
/// Pre-TASK-1194 the check ran on `json_text.len()` after the
/// `query_row` had already pulled the entire payload across the FFI
/// boundary — by the time the cap fired, the very allocation it was
/// meant to prevent had already happened. We verify the new ordering
/// by wiring up a ~100-MiB synthetic payload with a 1-MiB cap: if
/// the implementation regressed to materialising-then-checking, peak
/// RSS would balloon by ~100 MiB during this test (and on a
/// memory-tight CI runner the OS would kill the process the AC
/// describes).
#[test]
fn query_metadata_raw_rejects_oversized_payload_before_materialising() {
    let db = ops_duckdb::DuckDb::open_in_memory().expect("open in-memory");
    {
        let conn = db.lock().expect("lock");
        // 100 MiB of 'a' bytes wrapped in a singleton row. DuckDB's
        // repeat() builds the value server-side so the seed itself
        // does not pull 100 MiB across the FFI boundary.
        conn.execute_batch(
            "CREATE TABLE metadata_raw (workspace_root VARCHAR, blob VARCHAR);
             INSERT INTO metadata_raw \
             SELECT '/workspace', repeat('a', 100 * 1024 * 1024);",
        )
        .expect("seed");
    }
    let cap: u64 = 1024 * 1024;
    let err = query_metadata_raw_with_cap(&db, cap)
        .expect_err("100-MiB payload must fail under a 1-MiB cap");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("exceeds") && msg.contains("byte cap"),
        "error message must cite the cap, got: {msg}"
    );
    assert!(
        msg.contains(METADATA_MAX_BYTES_ENV),
        "error message must cite the override env var, got: {msg}"
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
