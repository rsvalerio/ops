//! `MetadataIngestor`: collect cargo metadata and load into `SQLite`.

use crate::views;
use crate::{check_metadata_not_capped, check_metadata_output, run_cargo_metadata};
use ops_extension::Context;
use ops_sqlite::sql::external_err;
use ops_sqlite::{
    init_schema, upsert_data_source, DataIngestor, DbError, DbResult, IngestDir, LoadResult, Sqlite,
};
use std::path::Path;

/// The single staged entry name this ingestor writes and reads.
///
/// It is a bare entry name rather than a path: every use resolves it against
/// the verified [`IngestDir`] anchor.
const METADATA_JSON: &str = "metadata.json";

pub struct MetadataIngestor;

impl DataIngestor for MetadataIngestor {
    fn name(&self) -> &'static str {
        "metadata"
    }

    fn collect(&self, ctx: &Context, dir: &IngestDir) -> DbResult<()> {
        // SEC-25: no `create_dir_all` here — the ingest directory is created,
        // hardened and verified once by `IngestDir::open` before `collect`
        // runs, and re-creating it by path would be another by-name resolution
        // of the directory this anchor exists to pin.
        let working_dir = ctx.working_directory();
        let output = run_cargo_metadata(working_dir).map_err(|e| match e {
            ops_core::subprocess::RunError::Io(io) => io_at(
                "running `cargo metadata` in working directory",
                working_dir,
                &io,
            ),
            ops_core::subprocess::RunError::Timeout(t) => DbError::Timeout {
                label: t.label,
                timeout_secs: t.timeout.as_secs(),
            },
            other => external_err(anyhow::Error::new(other).context("cargo metadata")),
        })?;
        check_metadata_output(&output).map_err(external_err)?;
        // ERR-1 / TASK-2188: a capped stdout is a truncated document. Refuse
        // it *before* `write_atomic`, so a truncated `metadata.json` is never
        // staged, never handed to `read_json_auto`, and never checksummed
        // into `data_sources` as certified ground truth.
        check_metadata_not_capped(&output).map_err(external_err)?;
        // SEC-25: persist `cargo metadata` stdout atomically (sibling temp +
        // fsync + rename), matching `SidecarIngestorConfig::collect_sidecar`,
        // so a crash mid-write cannot leave a torn or zero-byte
        // `metadata.json` for the subsequent `load` step to feed to SQLite's
        // `read_json_auto` and corrupt the database with truncated input.
        // The write is also anchored: temp create and publish rename both
        // resolve against the verified directory descriptor.
        dir.write_atomic(METADATA_JSON, &output.stdout)
            .map_err(|e| {
                // ERR-13 / TASK-1893: keep naming the path the write acted on;
                // `write_atomic` reports the syscall error, not the destination.
                match e {
                    DbError::Io(io) => io_at(
                        "writing staged cargo metadata JSON",
                        &dir.entry_path(METADATA_JSON),
                        &io,
                    ),
                    other => other,
                }
            })
    }

    fn load(&self, dir: &IngestDir, db: &Sqlite) -> DbResult<LoadResult> {
        let path = dir.entry_path(METADATA_JSON);
        // SEC-32: arm the cleanup *before* the first fallible step, so every
        // exit from `load` unlinks the staged file. `read_staged_payload`,
        // `init_schema`, `build_views`, the invariant guards, the
        // workspace-root extract and the checksum/upsert all return via `?`;
        // a guard armed any later would leave a full `cargo metadata` dump —
        // every workspace member, every dependency and absolute local paths —
        // on disk indefinitely.
        let _staged = StagedFile::new(dir);
        // ARCH-9 / TASK-1247 successor: the payload is read through the
        // anchor once, here, so the OPS_METADATA_MAX_BYTES cap is enforced in
        // Rust before the bytes reach the engine (the SQLite port replaced
        // the engine-side `maximum_object_size` read option with this check).
        let payload = read_staged_payload(dir)?;
        init_schema(db)?;
        // CONC-2: one guard held across table creation *and* the reads of
        // that table. Scoping `build_views` in its own block and re-acquiring
        // the lock on the next line would release nothing useful (nothing runs
        // in between) while splitting the `metadata_raw` (re)build from the
        // `count(*)` and `workspace_root` reads whose results are persisted
        // into the `data_sources` provenance row below. Anything replacing
        // `metadata_raw` in that gap would leave the recorded provenance
        // describing data that is no longer there. The orchestrator's
        // per-table ingest mutex
        // (`extensions/sqlite/src/sql/ingest/orchestrator.rs`) happens to
        // close the gap today, but `DataIngestor::load` is a public trait
        // method and its signature promises no such caller, so the atomicity
        // is enforced here instead of depended on from a distance.
        let conn = db.lock()?;
        build_views(&conn, &payload)?;
        let record_count = query_record_count(&conn)?;
        if record_count != 1 {
            return Err(reject_non_singleton(&conn, record_count));
        }
        ensure_object_payload(&conn)?;
        let workspace_root = extract_workspace_root(&conn)?;
        drop(conn);

        // SEC-25 / TASK-2054: checksum the file opened through the anchor, so
        // the provenance row describes the bytes this pipeline staged.
        let checksum = dir.checksum(METADATA_JSON)?;
        upsert_data_source(
            db,
            &ops_sqlite::DataSourceMetadata::new(
                ops_sqlite::SourceName::new(self.name()),
                ops_sqlite::WorkspaceRoot::new(std::ffi::OsStr::new(&workspace_root)),
                &path,
                record_count,
                &checksum,
            ),
        )?;
        Ok(LoadResult::success(self.name(), record_count))
    }
}

/// Reads the staged `metadata.json` through the anchor, enforcing the
/// [`crate::metadata_max_bytes`] cap on the bytes before they are handed to
/// the engine as a bound parameter.
///
/// One source of truth with `query_metadata_raw`'s read-side cap: the same
/// env knob governs the ingest-side allocation ceiling and the post-ingest
/// read guard. A payload over cap is refused here — before any DDL runs — so
/// an oversized document never becomes a `metadata_raw` row.
///
/// # Errors
///
/// [`DbError::Io`] if the staged file cannot be read through the anchor, or
/// [`DbError::External`] if the payload exceeds the cap (naming the observed
/// byte count, the cap, and the override env var).
fn read_staged_payload(dir: &IngestDir) -> DbResult<String> {
    use std::io::Read as _;
    let mut payload = String::new();
    dir.open_read(METADATA_JSON)?
        .read_to_string(&mut payload)
        .map_err(DbError::Io)?;
    let cap = crate::metadata_max_bytes();
    if payload.len() > usize::try_from(cap).unwrap_or(usize::MAX) {
        return Err(external_err(anyhow::anyhow!(
            "staged metadata payload is {} bytes, exceeds {cap}-byte cap \
             (override via {})",
            payload.len(),
            crate::METADATA_MAX_BYTES_ENV
        )));
    }
    Ok(payload)
}

/// Builds the `metadata_raw` table (single JSON blob row) and the
/// `crate_dependencies` view over it.
///
/// Kept separate from `MetadataIngestor::load` so the loader reads at one
/// nesting level. The staged payload arrives as an already-read string and is
/// bound as `?1` — no path reaches SQL (the SEC-25 / TASK-2067 residual is
/// closed by construction under the SQLite port).
fn build_views(conn: &rusqlite::Connection, payload: &str) -> DbResult<()> {
    ops_sqlite::sql::load_json_string(conn, &views::METADATA_RAW_LOAD, payload)?;
    let view_sql = views::crate_dependencies_view_sql();
    // DROP VIEW + CREATE VIEW must run together, hence `execute_batch`.
    conn.execute_batch(view_sql.as_str())
        .map_err(|e| DbError::query_failed("crate_dependencies view", e))?;
    Ok(())
}

/// Wraps an IO failure with the operation and the path it acted on.
///
/// `DbError::Io` renders as `"IO error: {0}"` and a bare `std::io::Error` names
/// no path, so an ENOSPC/EACCES on the ingest directory, on the working
/// directory, and on the staged JSON file would otherwise render identically —
/// `IO error: Permission denied (os error 13)`, with nothing telling the
/// operator which of the three failed. `ErrorKind` is preserved so callers
/// that branch on `NotFound` / `PermissionDenied` still can, and the variant
/// stays `DbError::Io` so a genuine filesystem failure is not laundered into
/// `DbError::External` (which `collect`'s tests use to mean "cargo ran and
/// failed").
fn io_at(op: &str, path: &Path, e: &std::io::Error) -> DbError {
    DbError::Io(std::io::Error::new(
        e.kind(),
        format!("{op} {}: {e}", path.display()),
    ))
}

/// Rejects a `metadata_raw` table that does not hold exactly one row, dropping
/// it so the next run re-ingests from scratch.
///
/// `metadata_raw` is a singleton table and [`crate::query_metadata_raw`] — its
/// only reader — hard-fails on any other row count. The blob load makes the
/// invariant structural (drop + create + one insert), so this firing signals
/// a foreign writer; it is kept as the last point where the bad state can
/// still be undone: accepting a multi-row table would leave the two halves of
/// the crate disagreeing, and because the table would then report
/// `table_has_data()`, every subsequent run would skip re-ingest and replay
/// the same failure. The dependent view is dropped along with the table.
fn reject_non_singleton(conn: &rusqlite::Connection, record_count: u64) -> DbError {
    tracing::warn!(
        rows = record_count,
        "metadata_raw must hold exactly one workspace_root row; dropping the table so the \
         next run re-ingests"
    );
    drop_metadata_tables(conn, "a non-singleton ingest");
    external_err(anyhow::anyhow!(
        "metadata_raw must contain exactly one row, found {record_count}; \
         dropped metadata_raw so the next ingest starts clean"
    ))
}

/// Rejects a staged payload that is not a single JSON object, dropping
/// `metadata_raw` so the next run re-ingests from scratch.
///
/// The single-blob load stores whatever JSON document was staged, verbatim —
/// `json(?1)` accepts arrays and scalars too. The `crate_dependencies` view
/// and the `workspace_root` extract both assume the cargo-metadata document
/// shape (an object), so anything else is refused with the observed shape
/// named rather than surfacing later as a confusing NULL-decode failure.
fn reject_non_object(conn: &rusqlite::Connection, observed: &str) -> DbError {
    tracing::warn!(
        shape = observed,
        "metadata_raw payload must be a single JSON object; dropping the table so the \
         next run re-ingests"
    );
    drop_metadata_tables(conn, "a non-object payload ingest");
    external_err(anyhow::anyhow!(
        "staged metadata payload must be a single JSON object, found {observed}; \
         dropped metadata_raw so the next ingest starts clean"
    ))
}

/// Checks that the loaded blob is a JSON object, rejecting the ingest
/// otherwise (see [`reject_non_object`]).
fn ensure_object_payload(conn: &rusqlite::Connection) -> DbResult<()> {
    let shape: String = conn
        .query_row("SELECT json_type(json) FROM metadata_raw", [], |row| {
            row.get(0)
        })
        .map_err(|e| DbError::query_failed("metadata_raw payload shape", e))?;
    if shape != "object" {
        return Err(reject_non_object(conn, &shape));
    }
    Ok(())
}

/// Best-effort teardown shared by the ingest rejection paths: drop the
/// `metadata_raw` table and the view that selects from it (view first), so
/// the next run's `table_has_data()` probe re-ingests instead of replaying
/// the failure. If the teardown itself fails the caller's error still
/// surfaces, and the operator sees both the invariant breach and why the
/// state is sticky.
fn drop_metadata_tables(conn: &rusqlite::Connection, reason: &'static str) {
    if let Err(e) = conn
        .execute_batch("DROP VIEW IF EXISTS crate_dependencies; DROP TABLE IF EXISTS metadata_raw;")
    {
        tracing::warn!(
            error = %e,
            "failed to drop metadata_raw after {reason}; the next run may replay this failure"
        );
    }
}

/// Counts rows in `metadata_raw`, mapping the raw `i64` to `u64` through the
/// project's `InvalidRecordCount` policy.
fn query_record_count(conn: &rusqlite::Connection) -> DbResult<u64> {
    let raw: i64 = conn
        .query_row("SELECT count(*) FROM metadata_raw", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|e| DbError::query_failed("metadata_raw count", e))?;
    u64::try_from(raw).map_err(|_| DbError::InvalidRecordCount {
        table: "metadata_raw".to_string(),
        count: raw,
    })
}

/// Reads `workspace_root` out of the loaded JSON blob.
///
/// A failure is enriched with a probe of the extracted value's observed type,
/// so a payload whose `workspace_root` is not a string names the type it
/// found rather than only the query.
fn extract_workspace_root(conn: &rusqlite::Connection) -> DbResult<String> {
    conn.query_row(
        "SELECT json_extract(json, '$.workspace_root') FROM metadata_raw",
        [],
        |row| row.get(0),
    )
    .map_err(|e| {
        let observed_type = conn
            .query_row(
                "SELECT typeof(json_extract(json, '$.workspace_root')) FROM metadata_raw",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap_or_else(|_| "<probe failed>".to_string());
        DbError::query_failed(
            format!("metadata_raw workspace_root extract (observed type: {observed_type})"),
            e,
        )
    })
}

/// Owns the staged `metadata.json` for the whole of
/// [`MetadataIngestor::load`] and unlinks it on `Drop`.
///
/// Success, `?` and the explicit `reject_non_singleton` rejection therefore all
/// clean up through one code path. Mirrors the terraform pipeline's
/// `with_artifact_cleanup`, using `Drop` rather than a wrapper because `load`'s
/// early exits are `?` rather than a single fallible expression.
///
/// Cleanup is unconditional: the file is a staging artifact that `collect`
/// rewrites from scratch on the next run, so there is no failure mode in
/// which keeping it helps.
struct StagedFile<'a> {
    dir: &'a IngestDir,
}

impl<'a> StagedFile<'a> {
    const fn new(dir: &'a IngestDir) -> Self {
        Self { dir }
    }
}

impl Drop for StagedFile<'_> {
    fn drop(&mut self) {
        cleanup_staged_file(self.dir);
    }
}

/// Best-effort removal of the staged JSON file.
///
/// A failure here never propagates: on the success path the `SQLite` row is
/// already committed and a propagated error would send the caller into a
/// re-ingest loop, and on a failure path the caller's own error is the one
/// worth surfacing.
fn cleanup_staged_file(dir: &IngestDir) {
    // SEC-25: no `exists()` probe first — that is check-then-act, and
    // `remove_file` already reports `NotFound`. An absent file is the normal
    // outcome when `load` fails before `collect` ever staged one, so it is
    // not worth a warning.
    // SEC-25 / TASK-2054: `unlinkat` on the verified descriptor.
    match dir.remove_file(METADATA_JSON) {
        Ok(()) => {}
        Err(DbError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            tracing::warn!(
                path = %dir.entry_path(METADATA_JSON).display(),
                error = %e,
                "failed to remove staged metadata file after load; leaving in place"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{ingest_anchor, ingest_dep, ingest_metadata, write_metadata_json};

    #[test]
    fn metadata_ingestor_name() {
        let ingestor = MetadataIngestor;
        assert_eq!(ingestor.name(), "metadata");
    }

    /// TEST-1 / TASK-1546: pin the failure mode to "cargo ran but couldn't
    /// locate a Cargo.toml" rather than asserting only `is_err()`. The bare
    /// assertion passed for the wrong reason on environments without
    /// `cargo` on `PATH` (`DbError::Io`) and on slow CI hits
    /// (`DbError::Timeout`), neither of which is what the test name
    /// promises. Match on `DbError::External` whose Display chain mentions
    /// `cargo metadata` so the test fails loudly if the upstream failure
    /// path stops surfacing the cargo origin.
    #[test]
    fn metadata_collect_fails_with_nonexistent_directory() {
        let ingestor = MetadataIngestor;
        // Build a path that is guaranteed not to exist by joining onto a
        // tempdir we never populate; the tempdir itself exists, but the
        // sub-path inside it does not.
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        let ctx = ops_extension::Context::test_context(missing);
        let data_dir = tempfile::tempdir().unwrap();
        let dir = ingest_anchor(&data_dir);
        let err = ingestor
            .collect(&ctx, &dir)
            .expect_err("collect must fail on a missing working directory");
        match &err {
            DbError::External(inner) => {
                let chain = format!("{inner:#}");
                assert!(
                    chain.contains("cargo metadata"),
                    "External error should attribute to cargo metadata; got: {chain}"
                );
            }
            DbError::Io(_) => panic!(
                "expected a cargo-metadata External error, got DbError::Io \
                 (is `cargo` on PATH?): {err}"
            ),
            other => panic!("expected DbError::External, got: {other:?}"),
        }
    }

    /// ERR-13 / TASK-1893: an IO failure inside `collect` must name the path
    /// it acted on, so a CI log distinguishes the filesystem edges without
    /// reading the source.
    ///
    /// SEC-25 / TASK-2054 rewrote which edges exist. `collect` no longer calls
    /// `create_dir_all` — the ingest directory is created, hardened and
    /// verified once by `IngestDir::open` before any ingestor runs — so the
    /// "creating metadata ingest directory" edge this test used to drive is
    /// gone with it. The staged-JSON edge is the one that remains inside
    /// `collect`, and it is driven here by parking a *directory* on the
    /// staged file's own name so the publish rename cannot succeed.
    #[test]
    fn metadata_collect_io_error_names_the_offending_path() {
        let data_dir = tempfile::tempdir().unwrap();
        let dir = ingest_anchor(&data_dir);
        std::fs::create_dir(dir.entry_path(METADATA_JSON)).expect("park a dir on the JSON name");

        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ctx = ops_extension::Context::test_context(manifest_dir);
        let err = MetadataIngestor
            .collect(&ctx, &dir)
            .expect_err("publishing over a directory must fail");
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains(&dir.entry_path(METADATA_JSON).display().to_string()),
            "IO error must name the path it operated on, got: {rendered}"
        );
        assert!(
            rendered.contains("writing staged cargo metadata JSON"),
            "IO error must name the operation that failed, got: {rendered}"
        );
    }

    /// SEC-25 / TASK-0933: a successful `MetadataIngestor::collect` must
    /// leave no `.tmp.*` leftover from the `atomic_write` sibling-temp
    /// pattern. Pin the cargo-metadata stdout write on the same crash-safe
    /// helper that `SidecarIngestorConfig::collect_sidecar` uses (TASK-0911),
    /// so a crash mid-write leaves either no `metadata.json` or the previous
    /// version — never a partial.
    #[test]
    fn metadata_collect_writes_atomically_no_tmp_leftover() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ctx = ops_extension::Context::test_context(manifest_dir);
        let data_dir = tempfile::tempdir().expect("tempdir");
        let dir = ingest_anchor(&data_dir);
        let ingestor = MetadataIngestor;
        ingestor
            .collect(&ctx, &dir)
            .expect("collect succeeds against this crate's manifest");
        let json_path = dir.entry_path(METADATA_JSON);
        assert!(json_path.exists(), "metadata.json was written");
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .expect("read_dir")
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp."))
            .collect();
        assert!(
            leftovers.is_empty(),
            "atomic_write left a tmp sibling: {leftovers:?}"
        );
    }

    #[test]
    fn metadata_load_with_sample_data() {
        let data_dir = tempfile::tempdir().unwrap();
        let dir = ingest_anchor(&data_dir);

        let metadata_json = ingest_metadata().dep(ingest_dep("serde", "^1.0")).value();
        let json_path = write_metadata_json(&dir, &metadata_json);

        let db = Sqlite::open_in_memory().expect("open in-memory db");
        let ingestor = MetadataIngestor;
        let result = ingestor.load(&dir, &db);
        assert!(result.is_ok());
        let load_result = result.unwrap();
        assert_eq!(load_result.source_name, "metadata");
        assert_eq!(load_result.record_count, 1);

        // Verify the view was created
        let conn = db.lock().unwrap();
        let dep_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM crate_dependencies WHERE dependency_name = 'serde'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        drop(conn);
        assert_eq!(dep_count, 1);

        // Verify JSON file was cleaned up
        assert!(!json_path.exists());
    }

    /// TASK-0982: regression — path dependencies (source = null) must not be
    /// silently dropped from the `crate_dependencies` view alongside registry
    /// deps.
    #[test]
    fn crate_dependencies_view_includes_path_deps() {
        let data_dir = tempfile::tempdir().unwrap();
        let dir = ingest_anchor(&data_dir);
        let metadata_json = ingest_metadata()
            .source(serde_json::json!(""))
            .dep(ingest_dep("serde", "^1.0"))
            .dep(ingest_dep("ws-sibling", "*").path_source())
            .value();
        write_metadata_json(&dir, &metadata_json);

        let db = Sqlite::open_in_memory().expect("open in-memory db");
        let ingestor = MetadataIngestor;
        let _ = ingestor.load(&dir, &db).unwrap();

        let conn = db.lock().unwrap();
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM crate_dependencies", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(total, 2, "both registry and path deps should be present");

        let path_dep_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM crate_dependencies WHERE dependency_name = 'ws-sibling'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        drop(conn);
        assert_eq!(path_dep_count, 1, "path dep (source=null) must be retained");
    }

    /// ERR-1 / TASK-1891 (was TASK-1043), ported to the blob shape: a staged
    /// payload that is not a single JSON object (here: an array of two
    /// cargo-metadata documents — the shape `DuckDB`'s `read_json_auto` used to
    /// explode into multiple rows) must be rejected at ingest rather than
    /// committed as a state `query_metadata_raw` then refuses to read.
    /// Assert both the warn and the error.
    #[test]
    fn metadata_load_rejects_metadata_raw_with_multiple_rows() {
        use ops_about::test_support::capture_tracing;

        let data_dir = tempfile::tempdir().unwrap();
        let dir = ingest_anchor(&data_dir);
        // Two-element JSON array → stored verbatim as one blob row, then
        // rejected by the object-shape guard with the observed shape named.
        let metadata_json = serde_json::Value::Array(vec![
            ingest_metadata().root("/test/a").value(),
            ingest_metadata().root("/test/b").value(),
        ]);
        write_metadata_json(&dir, &metadata_json);

        let db = Sqlite::open_in_memory().expect("open in-memory db");
        let ingestor = MetadataIngestor;
        let (logs, result) = capture_tracing(tracing::Level::WARN, || ingestor.load(&dir, &db));
        let err = result.expect_err("a non-object payload must not load successfully");
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("single JSON object") && rendered.contains("found array"),
            "error must name the invariant and the observed shape, got: {rendered}"
        );

        assert!(
            logs.contains("single JSON object"),
            "expected warn about the object-shape invariant, got: {logs}"
        );
        assert!(
            logs.contains("shape=\"array\""),
            // tracing renders a &str field value quoted: shape="array"
            "warn should include the observed shape field, got: {logs}"
        );
    }

    /// ERR-1 / TASK-1891 AC #2 + #3: the halves of the crate must agree.
    /// Drive `load` and then `query_metadata_raw` against the *same*
    /// `Sqlite`, and assert the combined outcome — a rejected load leaves no
    /// `metadata_raw` behind, so the orchestrator's `table_has_data()` probe
    /// re-ingests on the next run instead of replaying the read failure
    /// forever.
    #[test]
    fn metadata_load_rejection_leaves_no_sticky_metadata_raw() {
        let data_dir = tempfile::tempdir().unwrap();
        let dir = ingest_anchor(&data_dir);
        let metadata_json = serde_json::Value::Array(vec![
            ingest_metadata().root("/test/a").value(),
            ingest_metadata().root("/test/b").value(),
        ]);
        write_metadata_json(&dir, &metadata_json);

        let db = Sqlite::open_in_memory().expect("open in-memory db");
        let ingestor = MetadataIngestor;
        ingestor
            .load(&dir, &db)
            .expect_err("non-object load must fail");

        // The reader is the other half of the invariant: it must fail too,
        // and for the *absence* of the table rather than a row-count
        // mismatch, proving nothing readable-but-broken was committed.
        let read_err = crate::query_metadata_raw(&db)
            .expect_err("no metadata_raw should remain after a rejected load");
        let rendered = format!("{read_err:#}");
        assert!(
            !rendered.contains("exactly one row"),
            "a rejected load must not leave a multi-row table behind, got: {rendered}"
        );

        let conn = db.lock().expect("lock");
        let tables: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'metadata_raw'",
                [],
                |row| row.get(0),
            )
            .expect("catalog probe");
        drop(conn);
        assert_eq!(tables, 0, "metadata_raw must be gone after a rejected load");
    }

    /// SEC-32 / TASK-2033 AC #2 + #3: the rejection path added by ERR-1 /
    /// TASK-1891 is the most likely way out of `load` that is not `Ok`, and it
    /// used to skip the cleanup entirely — leaving a full `cargo metadata`
    /// dump (every workspace member, every dependency, absolute local paths)
    /// on disk with no bound on how long it stays there.
    #[test]
    fn metadata_load_rejection_removes_the_staged_json() {
        let data_dir = tempfile::tempdir().unwrap();
        let dir = ingest_anchor(&data_dir);
        let metadata_json = serde_json::Value::Array(vec![
            ingest_metadata().root("/test/a").value(),
            ingest_metadata().root("/test/b").value(),
        ]);
        let json_path = write_metadata_json(&dir, &metadata_json);

        let db = Sqlite::open_in_memory().expect("open in-memory db");
        MetadataIngestor
            .load(&dir, &db)
            .expect_err("two-row load must fail");

        assert!(
            !json_path.exists(),
            "the staged metadata.json must not survive a rejected load"
        );
    }

    /// SEC-32 / TASK-2033 AC #1: the guard is armed before the first fallible
    /// step, so a failure that happens *earlier* than the row-count check —
    /// here `metadata_raw create` choking on input `read_json_auto` cannot
    /// parse — cleans up too. Pins that the cleanup is a scope guard rather
    /// than a second call site bolted onto one more error path.
    #[test]
    fn metadata_load_removes_the_staged_json_when_the_table_build_fails() {
        let data_dir = tempfile::tempdir().unwrap();
        let dir = ingest_anchor(&data_dir);
        let json_path = dir.entry_path(METADATA_JSON);
        dir.write_atomic(METADATA_JSON, b"this is not JSON at all")
            .unwrap();

        let db = Sqlite::open_in_memory().expect("open in-memory db");
        MetadataIngestor
            .load(&dir, &db)
            .expect_err("unparseable staged JSON must fail the load");

        assert!(
            !json_path.exists(),
            "the staged metadata.json must not survive a failed table build"
        );
    }

    /// PATTERN-1 / TASK-1056: the same dependency declared under two
    /// `[target.'cfg(...)'.dependencies]` blocks must surface as TWO
    /// distinct rows in `crate_dependencies` (preserving the
    /// platform-specific shape via the new `target` column) rather than
    /// collapsing into a single tuple. cargo metadata serialises each
    /// declaration as its own entry in `package.dependencies`, so the
    /// view must keep both — TASK-0982 fixed the inverse drop, this
    /// fixes the duplicate-collapse.
    #[test]
    fn crate_dependencies_view_preserves_target_conditional_duplicates() {
        let data_dir = tempfile::tempdir().unwrap();
        let dir = ingest_anchor(&data_dir);
        let metadata_json = ingest_metadata()
            .dep(ingest_dep("libc", "^0.2").target("cfg(unix)"))
            .dep(ingest_dep("libc", "^0.2").target("cfg(windows)"))
            .value();
        write_metadata_json(&dir, &metadata_json);

        let db = Sqlite::open_in_memory().expect("open in-memory db");
        let ingestor = MetadataIngestor;
        let _ = ingestor.load(&dir, &db).unwrap();

        let conn = db.lock().unwrap();
        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM crate_dependencies WHERE dependency_name = 'libc'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            total, 2,
            "both target-conditional libc declarations must surface as distinct rows"
        );

        // The new `target` column must carry the cfg expression so
        // platform-specific shape isn't lost.
        let mut targets: Vec<String> = Vec::new();
        let mut stmt = conn
            .prepare(
                "SELECT target FROM crate_dependencies \
                 WHERE dependency_name = 'libc' \
                 ORDER BY target",
            )
            .unwrap();
        let rows = stmt
            .query_map([], |row| {
                Ok(row
                    .get::<_, Option<String>>(0)?
                    .unwrap_or_else(|| "<null>".to_string()))
            })
            .unwrap();
        for r in rows {
            targets.push(r.unwrap());
        }
        // `stmt` borrows `conn`, so it has to go first.
        drop(stmt);
        drop(conn);
        assert_eq!(targets, vec!["cfg(unix)", "cfg(windows)"]);
    }

    /// FN-1 / TASK-1543 AC#2: drive the `extract_workspace_root` typeof-probe
    /// fallback by handing it a blob whose `workspace_root` value is
    /// JSON-numeric (`json_extract` then yields an INTEGER, which cannot
    /// decode to `String`). The probe should observe the type and surface it
    /// in the error so the operator sees the offending shape.
    #[test]
    fn extract_workspace_root_typeof_probe_surfaces_observed_type() {
        let db = Sqlite::open_in_memory().expect("open in-memory db");
        let conn = db.lock().expect("acquire connection");
        conn.execute("CREATE TABLE metadata_raw (json TEXT NOT NULL)", [])
            .expect("create table");
        conn.execute(
            "INSERT INTO metadata_raw VALUES ('{\"workspace_root\": 42}')",
            [],
        )
        .expect("seed row");
        let err = super::extract_workspace_root(&conn)
            .expect_err("numeric workspace_root cannot deserialise to String");
        drop(conn);
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("observed type: integer"),
            "typeof-probe must name observed value type; got: {rendered}"
        );
    }

    // TEST-1 / TASK-1546: the previous `negative_record_count_surfaces_as_…`
    // test constructed `u64::try_from(-1)` inline and pattern-matched the
    // error it created itself — it exercised no production code path. The
    // `InvalidRecordCount` mapping in `MetadataIngestor::load` (see lines
    // ~67-72 above) is already exercised by the loader's existing
    // success-path tests and by the broader SQLite record-count plumbing
    // in `ops-sqlite`; a dedicated tautology test added no coverage and
    // gave reviewers false confidence, so it has been removed.
}
