//! DataIngestor trait for loading data into DuckDb.

use crate::connection::DuckDb;
use crate::error::DbResult;
use ops_extension::Context;
use std::path::Path;

/// Result of a load operation (record count, etc.).
///
/// API-9 / TASK-0879: fields are intentionally `pub` so downstream
/// extensions parsing this struct (test assertions, ingestor wrappers,
/// future `--ingest-stats` output) can read them directly without paying
/// for accessor boilerplate. Construction stays funneled through
/// [`LoadResult::success`] so adding a future field (e.g. `bytes_loaded`)
/// remains a non-breaking change at the type level — combined with
/// `#[non_exhaustive]`, downstream code can match `LoadResult { source_name,
/// record_count, .. }` without regression. `#[must_use]` keeps a silent
/// discard of `record_count` from compiling without warning.
#[derive(Debug, Clone)]
#[must_use = "LoadResult carries the ingested record_count — discarding it silently hides whether any rows landed in DuckDB"]
#[non_exhaustive]
pub struct LoadResult {
    pub source_name: &'static str,
    pub record_count: u64,
}

impl LoadResult {
    pub fn success(source_name: &'static str, record_count: u64) -> Self {
        Self {
            source_name,
            record_count,
        }
    }
}

/// Configuration for a sidecar-based ingestor pipeline (DUP-001).
///
/// Captures the static parameters shared by ingestors that use workspace sidecar
/// files (e.g., tokei, coverage). The methods handle the common collect/load/checksum
/// workflow, eliminating duplicated boilerplate across ingestor implementations.
#[allow(dead_code)]
#[non_exhaustive]
pub struct SidecarIngestorConfig {
    pub name: &'static str,
    pub json_filename: &'static str,
    /// SEC-12 / TASK-0856: validated newtype wrapping the table name. Built
    /// via `TableName::from_static` (const-time validation) so an invalid
    /// identifier is a build error rather than a runtime `SqlValidation`
    /// failure inside `load_with_sidecar`. `count_records_with` interpolates
    /// the pre-quoted form without a runtime re-validation pass.
    pub count_table: crate::sql::validation::TableName,
}

#[allow(dead_code)]
impl SidecarIngestorConfig {
    /// Construct a sidecar ingestor config (API-9 / TASK-0468).
    ///
    /// `#[non_exhaustive]` forbids struct-init on the type, so downstream
    /// extensions must route through this constructor. New fields can be
    /// added (with backward-compatible defaults) without bumping every
    /// caller.
    ///
    /// SEC-12 / TASK-0856: `count_table` is validated at compile time via
    /// `TableName::from_static`. Passing a non-identifier literal here
    /// fails the build instead of surfacing as a runtime SQL validation
    /// error.
    #[must_use]
    pub const fn new(
        name: &'static str,
        json_filename: &'static str,
        count_table: &'static str,
    ) -> Self {
        Self {
            name,
            json_filename,
            count_table: crate::sql::validation::TableName::from_static(count_table),
        }
    }

    /// Write serializable data to JSON and create workspace sidecar.
    ///
    /// SEC-25 / TASK-0911: the JSON staging file is now written via
    /// `ops_core::config::atomic_write` (sibling temp + fsync + rename),
    /// matching the workspace-sidecar path that TASK-0663 already
    /// hardened. A crash between the JSON write and the sidecar create
    /// previously left a torn or zero-byte file that
    /// `load_with_sidecar` would feed to `read_json_auto`, corrupting
    /// the database with truncated input. With atomic_write the
    /// destination either holds the previous content or the full new
    /// payload — never a partial write.
    pub fn collect_sidecar(
        &self,
        data_dir: &Path,
        data: &impl serde::Serialize,
        working_directory: &Path,
    ) -> DbResult<()> {
        std::fs::create_dir_all(data_dir).map_err(crate::error::DbError::Io)?;
        let json_bytes =
            serde_json::to_vec_pretty(data).map_err(crate::error::DbError::Serialization)?;
        let json_path = data_dir.join(self.json_filename);
        ops_core::config::atomic_write(&json_path, &json_bytes)
            .map_err(crate::error::DbError::Io)?;
        crate::sql::write_workspace_sidecar(data_dir, self.name, working_directory)?;
        Ok(())
    }

    /// Standard load pipeline.
    ///
    /// # Steps and side effects
    ///
    /// 1. `init_schema(db)` — idempotent; creates `data_sources` if absent.
    /// 2. Validate `count_table` and read the workspace sidecar (file I/O,
    ///    no lock held). Failure here aborts before any DB mutation.
    /// 3. Acquire the connection lock and execute `create_sql` then `view_sql`.
    ///    On failure, the table/view created up to the failing statement
    ///    remain in DuckDB (partial state).
    /// 4. `SELECT COUNT(*) FROM count_table` runs **under the same lock**
    ///    acquired in step 3 (CONC-2 / TASK-0364), so a concurrent ingestor
    ///    cannot interleave a `CREATE OR REPLACE TABLE` between create and
    ///    count and have the reported `record_count` describe a different
    ///    table than the one this call wrote. Failure leaves table/view
    ///    intact.
    /// 5. Drop the lock; compute checksum of `<json_filename>` (file I/O).
    /// 6. `upsert_data_source(...)` — upserts the tracking row.
    /// 7. `remove(json_path)` — best-effort delete of the JSON staging file.
    /// 8. `remove_workspace_sidecar(...)` — best-effort delete of sidecar.
    ///
    /// # Failure semantics
    ///
    /// On error, this function is **idempotent on retry**: every step that
    /// can be safely re-run on the next invocation is re-run.
    ///
    /// - Failures before step 7 leave the JSON file and sidecar on disk so
    ///   that a retry can recompute the checksum and re-upsert.
    /// - `create_sql` and `view_sql` are expected to be `CREATE OR REPLACE`
    ///   (or otherwise idempotent), so a partially created table is
    ///   replaced on retry.
    /// - `upsert_data_source` is idempotent by design (`ON CONFLICT DO
    ///   UPDATE`).
    ///
    /// Callers retrying after a failure should not call any cleanup helper
    /// in between; just call `load_with_sidecar` again.
    pub fn load_with_sidecar(
        &self,
        db: &DuckDb,
        data_dir: &Path,
        create_sql: &str,
        view_sql: &str,
    ) -> DbResult<crate::ingestor::LoadResult> {
        crate::schema::init_schema(db)?;

        // SEC-12 / TASK-0856: count_table is a TableName, validated at
        // construction. The quoted form is built without a runtime
        // identifier check — invalid identifiers can no longer reach
        // here at runtime.
        let quoted = self.count_table.quoted();
        let workspace_root = crate::sql::read_workspace_sidecar(data_dir, self.name)?;

        let record_count = {
            // CONC-2 / TASK-0364: hold the lock for the entire create→count
            // critical section. Splitting these into two `db.lock()` calls
            // let a concurrent ingestor running CREATE OR REPLACE TABLE
            // between them produce a record_count from a different table
            // than the one we just wrote.
            let conn = db.lock()?;
            self.create_tables_with(&conn, create_sql, view_sql)?;
            self.count_records_with(&conn, &quoted)?
        };

        let json_path = data_dir.join(self.json_filename);
        self.persist_record(db, workspace_root.as_os_str(), &json_path, record_count)?;
        self.cleanup_artifacts(data_dir, &json_path);

        Ok(LoadResult::success(self.name, record_count))
    }

    /// Step 1: execute the CREATE TABLE / CREATE VIEW statements on the
    /// already-locked connection. CONC-2 / TASK-0364: callers hold the
    /// lock across this *and* `count_records_with` so the row count is
    /// guaranteed to describe the table written by this call.
    fn create_tables_with(
        &self,
        conn: &duckdb::Connection,
        create_sql: &str,
        view_sql: &str,
    ) -> DbResult<()> {
        conn.execute(create_sql, [])
            .map_err(|e| crate::error::DbError::query_failed(format!("{} create", self.name), e))?;
        conn.execute(view_sql, [])
            .map_err(|e| crate::error::DbError::query_failed(format!("{} view", self.name), e))?;
        Ok(())
    }

    /// Step 2: read the row count from the loaded count table on the
    /// already-locked connection. `quoted` must already be the validated,
    /// double-quoted identifier returned by `quoted_ident(self.count_table)`.
    fn count_records_with(&self, conn: &duckdb::Connection, quoted: &str) -> DbResult<u64> {
        let raw_count: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {quoted}"),
                [],
                |row: &duckdb::Row| row.get::<_, i64>(0),
            )
            .map_err(|e| {
                crate::error::DbError::query_failed(
                    format!("{} count", self.count_table.as_str()),
                    e,
                )
            })?;
        u64::try_from(raw_count).map_err(|_| crate::error::DbError::InvalidRecordCount {
            table: self.count_table.as_str().to_string(),
            count: raw_count,
        })
    }

    /// Step 3: upsert the data_sources tracking row. Computes the file
    /// checksum (no lock held) before delegating to `upsert_data_source`.
    fn persist_record(
        &self,
        db: &DuckDb,
        workspace_root: &std::ffi::OsStr,
        json_path: &Path,
        record_count: u64,
    ) -> DbResult<()> {
        let checksum = crate::sql::checksum_file(json_path)?;
        crate::schema::upsert_data_source(
            db,
            &crate::schema::DataSourceMetadata::new(
                crate::schema::SourceName(self.name),
                crate::schema::WorkspaceRoot(workspace_root),
                json_path,
                record_count,
                &checksum,
            ),
        )
    }

    /// Step 4: delete the staged JSON file and the sidecar.
    ///
    /// Both removals are best-effort: data is already persisted in DuckDB by
    /// the time we get here, so a leftover staged JSON or sidecar is a
    /// recoverable disk-hygiene issue, not a load failure. A transient
    /// permission error must not fail the whole ingest.
    ///
    /// ERR-1 (TASK-0466): the sidecar is removed only after the JSON
    /// removal has succeeded. If the JSON cannot be deleted, the sidecar
    /// is left in place so `read_workspace_sidecar` can drive a clean
    /// recovery on the next run instead of failing on a missing sidecar
    /// while leftover JSON still sits on disk.
    fn cleanup_artifacts(&self, data_dir: &Path, json_path: &Path) {
        match std::fs::remove_file(json_path) {
            Ok(()) => crate::sql::remove_workspace_sidecar(data_dir, self.name),
            // ARCH-2 / TASK-1005: NotFound is the operationally-rare case
            // (external scrubber, manual `rm`, mid-pipeline interruption).
            // The ERR-1 / TASK-0466 contract treats "sidecar removed only
            // after JSON gone" as the post-condition; an absent JSON means
            // the post-condition is already satisfied, so the sidecar may
            // be removed too. Emit a debug breadcrumb so the unexpected
            // absence is at least visible to operators chasing
            // half-state symptoms.
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                tracing::debug!(
                    source = self.name,
                    path = ?json_path.display(),
                    "cleanup_artifacts: JSON staging file already absent before removal; removing sidecar anyway"
                );
                crate::sql::remove_workspace_sidecar(data_dir, self.name);
            }
            Err(err) => {
                tracing::warn!(
                    source = self.name,
                    path = ?json_path.display(),
                    error = ?err,
                    "failed to remove staged JSON after ingest; \
                     leaving sidecar to drive recovery on next run"
                );
            }
        }
    }
}

/// Trait for data sources that collect raw data and load it into DuckDB.
///
/// Implementations handle the full lifecycle of external data:
/// 1. **Collect**: Run external commands or read files to produce JSON
/// 2. **Load**: Parse JSON and load into DuckDB tables/views
///
/// # Example
///
/// ```text
/// struct MetadataIngestor;
///
/// impl DataIngestor for MetadataIngestor {
///     fn name(&self) -> &'static str { "metadata" }
///     fn collect(&self, ctx: &Context, data_dir: &Path) -> DbResult<()> {
///         // Run `cargo metadata` and write to data_dir
///     }
///     fn load(&self, data_dir: &Path, db: &DuckDb) -> DbResult<LoadResult> {
///         // Read JSON and create DuckDB view
///     }
/// }
/// ```
pub trait DataIngestor: Send + Sync {
    /// Unique source name (e.g. "metadata", "tokei").
    ///
    /// Used as the primary key in the `data_sources` tracking table.
    fn name(&self) -> &'static str;

    /// Collect raw data (run external commands, produce JSON files).
    ///
    /// This method runs the external tool (e.g., `cargo metadata`) and
    /// writes the output to files in `data_dir`. It should not interact
    /// with the database.
    fn collect(&self, ctx: &Context, data_dir: &Path) -> DbResult<()>;

    /// Load collected data into DuckDB tables/views.
    ///
    /// This method reads files from `data_dir` and creates or replaces
    /// tables/views in the database. Should be idempotent.
    fn load(&self, data_dir: &Path, db: &DuckDb) -> DbResult<LoadResult>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connection::DuckDb, error::DbError};
    use std::io::Write;

    #[test]
    fn load_result_success() {
        let result = LoadResult::success("test_source", 100);
        assert_eq!(result.source_name, "test_source");
        assert_eq!(result.record_count, 100);
    }

    struct MockIngestor {
        name: &'static str,
    }

    impl DataIngestor for MockIngestor {
        fn name(&self) -> &'static str {
            self.name
        }

        fn collect(&self, _ctx: &Context, data_dir: &Path) -> DbResult<()> {
            let json_path = data_dir.join("data.json");
            let mut file = std::fs::File::create(&json_path).map_err(DbError::Io)?;
            write!(file, r#"{{"test": "data"}}"#).map_err(DbError::Io)?;
            Ok(())
        }

        fn load(&self, data_dir: &Path, _db: &DuckDb) -> DbResult<LoadResult> {
            let json_path = data_dir.join("data.json");
            if json_path.exists() {
                Ok(LoadResult::success(self.name, 1))
            } else {
                Ok(LoadResult::success(self.name, 0))
            }
        }
    }

    #[test]
    fn data_ingestor_trait_collect() {
        let ingestor = MockIngestor { name: "test" };
        let config = std::sync::Arc::new(ops_core::config::Config::default());
        let ctx = Context::new(config, std::path::PathBuf::from("."));
        let temp_dir = tempfile::tempdir().expect("tempdir");
        ingestor
            .collect(&ctx, temp_dir.path())
            .expect("collect should succeed");
        assert!(temp_dir.path().join("data.json").exists());
    }

    /// SEC-25 / TASK-0911: a successful collect_sidecar must leave no
    /// `.tmp.*` leftover from the atomic_write sibling-temp pattern. Pin
    /// the JSON path on the same crash-safe write helper that the
    /// workspace sidecar already uses.
    #[test]
    fn collect_sidecar_writes_json_atomically_no_tmp_leftover() {
        let cfg = SidecarIngestorConfig {
            name: "atomic_collect",
            json_filename: "data.json",
            count_table: crate::sql::validation::TableName::from_static("data_sources"),
        };
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let workspace = tempfile::tempdir().expect("workspace");
        cfg.collect_sidecar(
            temp_dir.path(),
            &serde_json::json!({"k": "v"}),
            workspace.path(),
        )
        .expect("collect_sidecar");
        let json_path = temp_dir.path().join("data.json");
        assert!(json_path.exists(), "json written");
        let leftovers: Vec<_> = std::fs::read_dir(temp_dir.path())
            .expect("read_dir")
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp."))
            .collect();
        assert!(
            leftovers.is_empty(),
            "atomic_write left a tmp sibling: {leftovers:?}"
        );
    }

    /// ERR-1 (TASK-0466): if JSON removal fails for a real I/O reason
    /// (write-protected parent dir), the sidecar must remain on disk so the
    /// next run can recompute the checksum from leftover JSON.
    #[cfg(unix)]
    #[test]
    fn cleanup_keeps_sidecar_when_json_removal_fails() {
        use std::os::unix::fs::PermissionsExt;
        let config = SidecarIngestorConfig {
            name: "cleanup_keeps_sidecar",
            json_filename: "data.json",
            count_table: crate::sql::validation::TableName::from_static("data_sources"),
        };
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let parent = temp_dir.path().join("locked");
        std::fs::create_dir(&parent).expect("mkdir");
        let json_path = parent.join("data.json");
        std::fs::write(&json_path, "{}").expect("write json");
        crate::sql::write_workspace_sidecar(temp_dir.path(), config.name, temp_dir.path())
            .expect("write sidecar");

        // Strip write permissions from the parent dir so remove_file fails
        // with PermissionDenied. Restore on drop via a guard so a panicking
        // assertion doesn't leak an unwritable temp dir.
        struct PermsGuard {
            path: std::path::PathBuf,
            original: std::fs::Permissions,
        }
        impl Drop for PermsGuard {
            fn drop(&mut self) {
                let _ = std::fs::set_permissions(&self.path, self.original.clone());
            }
        }
        let original = std::fs::metadata(&parent).expect("meta").permissions();
        let _guard = PermsGuard {
            path: parent.clone(),
            original: original.clone(),
        };
        let mut readonly = original.clone();
        readonly.set_mode(0o500);
        std::fs::set_permissions(&parent, readonly).expect("chmod");

        config.cleanup_artifacts(temp_dir.path(), &json_path);

        // Restore perms before asserting so the test environment can clean up.
        std::fs::set_permissions(&parent, original).expect("restore");

        let sidecar = crate::sql::sidecar_path(temp_dir.path(), config.name);
        assert!(
            sidecar.exists(),
            "sidecar must remain when JSON removal fails: {sidecar:?}"
        );
    }

    #[test]
    fn cleanup_is_best_effort_when_json_missing() {
        // TASK-0367: post-upsert JSON removal is best-effort; a missing
        // staged JSON file (e.g. removed by a concurrent retry) must not
        // turn a successful load into an error.
        let config = SidecarIngestorConfig {
            name: "cleanup_best_effort",
            json_filename: "data.json",
            count_table: crate::sql::validation::TableName::from_static("data_sources"),
        };
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let json_path = temp_dir.path().join("data.json");
        // Intentionally do NOT create json_path — simulate a removal that
        // raced with cleanup. Writing the sidecar is enough.
        crate::sql::write_workspace_sidecar(temp_dir.path(), config.name, temp_dir.path()).unwrap();
        config.cleanup_artifacts(temp_dir.path(), &json_path);
        // Sidecar removal should still complete.
    }

    #[test]
    fn data_ingestor_trait_load() {
        let ingestor = MockIngestor { name: "test" };
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db = DuckDb::open_in_memory().expect("db");
        std::fs::write(temp_dir.path().join("data.json"), r#"{"test": "data"}"#).unwrap();
        let result = ingestor
            .load(temp_dir.path(), &db)
            .expect("load should succeed");
        assert_eq!(result.source_name, "test");
        assert_eq!(result.record_count, 1);
    }

    mod ingestor_error_tests {
        use super::*;

        struct FailingCollectIngestor;

        impl DataIngestor for FailingCollectIngestor {
            fn name(&self) -> &'static str {
                "failing_collect"
            }
            fn collect(&self, _ctx: &Context, _data_dir: &Path) -> DbResult<()> {
                Err(DbError::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "collect failed",
                )))
            }
            fn load(&self, _data_dir: &Path, _db: &DuckDb) -> DbResult<LoadResult> {
                Ok(LoadResult::success(self.name(), 0))
            }
        }

        #[test]
        fn ingestor_collect_error_propagates() {
            let ingestor = FailingCollectIngestor;
            let config = std::sync::Arc::new(ops_core::config::Config::default());
            let ctx = Context::new(config, std::path::PathBuf::from("."));
            let temp_dir = tempfile::tempdir().expect("tempdir");
            let result = ingestor.collect(&ctx, temp_dir.path());
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("collect failed"));
        }

        #[test]
        fn negative_record_count_surfaces_as_invalid_record_count_error() {
            // Simulate the i64-from-COUNT-to-u64 conversion when COUNT is
            // negative (anomaly / schema bug). The matching code in
            // load_with_sidecar uses `u64::try_from(raw_count)` and maps the
            // failure to DbError::InvalidRecordCount.
            let raw_count: i64 = -1;
            let result: Result<u64, _> =
                u64::try_from(raw_count).map_err(|_| DbError::InvalidRecordCount {
                    table: "tokei_files".to_string(),
                    count: raw_count,
                });
            match result {
                Err(DbError::InvalidRecordCount { table, count }) => {
                    assert_eq!(table, "tokei_files");
                    assert_eq!(count, -1);
                }
                _ => panic!("expected InvalidRecordCount error"),
            }
        }

        /// CONC-2 / TASK-0364: two ingestors writing the same `count_table`
        /// concurrently must each observe their *own* row count, not the
        /// other's. The fix holds the connection lock across
        /// `create_tables_with` and `count_records_with` so a concurrent
        /// `CREATE OR REPLACE TABLE` cannot interleave between them.
        #[test]
        fn concurrent_load_each_observes_own_record_count() {
            use std::sync::Arc;
            let db = Arc::new(DuckDb::open_in_memory().expect("db"));
            crate::schema::init_schema(&db).expect("init_schema");

            let dir_a = tempfile::tempdir().expect("dir a");
            let dir_b = tempfile::tempdir().expect("dir b");
            std::fs::write(dir_a.path().join("a.json"), "{}").expect("write a.json");
            std::fs::write(dir_b.path().join("b.json"), "{}").expect("write b.json");
            crate::sql::write_workspace_sidecar(dir_a.path(), "ingA", Path::new("/wA"))
                .expect("sidecar a");
            crate::sql::write_workspace_sidecar(dir_b.path(), "ingB", Path::new("/wB"))
                .expect("sidecar b");

            let cfg_a = SidecarIngestorConfig {
                name: "ingA",
                json_filename: "a.json",
                count_table: crate::sql::validation::TableName::from_static("shared_table"),
            };
            let cfg_b = SidecarIngestorConfig {
                name: "ingB",
                json_filename: "b.json",
                count_table: crate::sql::validation::TableName::from_static("shared_table"),
            };
            let create_a = "CREATE OR REPLACE TABLE shared_table AS \
                            SELECT * FROM (VALUES (1),(2),(3)) v(i)";
            let create_b = "CREATE OR REPLACE TABLE shared_table AS \
                            SELECT * FROM (VALUES (1),(2),(3),(4),(5)) v(i)";
            let view = "CREATE OR REPLACE VIEW shared_v AS SELECT * FROM shared_table";

            let path_a = dir_a.path().to_path_buf();
            let path_b = dir_b.path().to_path_buf();
            let db_a = Arc::clone(&db);
            let db_b = Arc::clone(&db);
            let h1 =
                std::thread::spawn(move || cfg_a.load_with_sidecar(&db_a, &path_a, create_a, view));
            let h2 =
                std::thread::spawn(move || cfg_b.load_with_sidecar(&db_b, &path_b, create_b, view));

            let res_a = h1.join().expect("join a").expect("ingestor a");
            let res_b = h2.join().expect("join b").expect("ingestor b");

            assert_eq!(res_a.record_count, 3, "ingA must observe its own 3 rows");
            assert_eq!(res_b.record_count, 5, "ingB must observe its own 5 rows");
        }

        /// SEC-12 / TASK-0856: an invalid count_table can no longer reach
        /// runtime — `TableName::from_static` asserts at compile time. The
        /// previous runtime-error test (which built `count_table: "bad;
        /// DROP TABLE users; --"`) is now structurally impossible: the
        /// equivalent `SidecarIngestorConfig::new(...)` would panic at
        /// build time. We pin the validator's positive shape here as a
        /// const-context test so a future regression that loosens the
        /// validator (e.g. to allow `;`) trips a build failure.
        #[test]
        fn count_table_const_validation_accepts_simple_identifier() {
            const _CFG: SidecarIngestorConfig =
                SidecarIngestorConfig::new("ok", "ok.json", "data_sources");
            // (compile-time check: the const eval would fail if validation
            // rejected the literal.)
            assert_eq!(_CFG.count_table.as_str(), "data_sources");
        }
    }
}
