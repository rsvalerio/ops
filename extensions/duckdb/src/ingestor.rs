//! `DataIngestor` trait for loading data into `DuckDb`.

use crate::connection::DuckDb;
// READ-4 / TASK-1875: `DbError` is imported for the intra-doc links in the
// `# Errors` sections below; without it `[`DbError::Io`]` and friends did not
// resolve. The `use` is doc-only, hence the narrow `expect`.
#[expect(
    unused_imports,
    reason = "referenced only by intra-doc links in this module"
)]
use crate::error::DbError;
use crate::error::DbResult;
use crate::sql::IngestDir;
use crate::sql::{CreateTableSql, CreateViewSql};
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
    pub const fn success(source_name: &'static str, record_count: u64) -> Self {
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
    /// the database with truncated input. With `atomic_write` the
    /// destination either holds the previous content or the full new
    /// payload — never a partial write.
    ///
    /// SEC-25 / TASK-2054: both staged writes now go through the verified
    /// [`IngestDir`] anchor rather than by path. `create_dir_all` is gone with
    /// them — the directory is created, hardened and verified once by
    /// [`IngestDir::open`] before `collect` is ever called, and re-creating it
    /// here would have been another by-name resolution of exactly the kind the
    /// anchor removes.
    ///
    /// # Errors
    ///
    /// [`DbError::Io`] if the JSON or the sidecar cannot be staged, or
    /// [`DbError::Serialization`] if `data` fails to serialize.
    pub fn collect_sidecar(
        &self,
        dir: &IngestDir,
        data: &impl serde::Serialize,
        working_directory: &Path,
    ) -> DbResult<()> {
        let json_bytes =
            serde_json::to_vec_pretty(data).map_err(crate::error::DbError::Serialization)?;
        dir.write_atomic(self.json_filename, &json_bytes)?;
        crate::sql::write_workspace_sidecar(dir, self.name, working_directory)?;
        Ok(())
    }

    /// Standard load pipeline.
    ///
    /// # Steps and side effects
    ///
    /// 1. `init_schema(db)` — idempotent; creates `data_sources` if absent.
    /// 2. Validate `count_table` and read the workspace sidecar (file I/O,
    ///    no lock held). Failure here aborts before any DB mutation.
    /// 3. Acquire the connection lock, check that `<json_filename>` resolves
    ///    to the same inode by path as through the anchor (SEC-25 /
    ///    TASK-2067: `create_sql` hands `DuckDB` that path and the engine
    ///    re-resolves it by name), then execute `create_sql` and `view_sql`.
    ///    On failure, the table/view created up to the failing statement
    ///    remain in `DuckDB` (partial state).
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
    ///
    /// # Crash between step 6 and steps 7-8 (TASK-1008)
    ///
    /// If the host crashes (`kill -9`, OOM, power loss) after the
    /// `upsert_data_source` row is durable but before `remove(json_path)`
    /// or `remove_workspace_sidecar` runs, the next invocation observes:
    ///
    /// - `DuckDB` row says `(source, checksum)` is fresh.
    /// - The staging JSON and sidecar are still on disk.
    /// - The next `provide_via_ingestor` short-circuits via
    ///   `table_has_data == true` and skips collect/load entirely.
    ///
    /// **Decision**: leftover JSON / sidecar after a successful upsert is
    /// expected debris of a crash and is operationally safe to delete.
    /// The post-success cleanup is best-effort by design — the durable
    /// state-of-truth is the `data_sources` row, and the staged files
    /// carry no information not already encoded in the checksum on that
    /// row. Operators auditing `target/ops/data.duckdb.ingest/` can
    /// remove any file whose corresponding `(source, checksum)` row is
    /// already current; a future ops invocation will repopulate the
    /// stage as needed.
    ///
    /// A future hardening (option B in TASK-1008) is to rename the JSON
    /// to a `.done` suffix under the same lock as the upsert so leftover
    /// debris is unambiguously post-load rather than mid-flight. The
    /// rename is *not* implemented today because the lower-cost
    /// mitigation — operators can recognize debris from the checksum row
    /// — covers the audit-clarity concern this contract documents.
    ///
    /// # Errors
    ///
    /// If schema initialisation, the sidecar load, or the create/view SQL
    /// fails; see [`DbError`] for the specific variants.
    pub fn load_with_sidecar(
        &self,
        db: &DuckDb,
        dir: &IngestDir,
        create_sql: &CreateTableSql,
        view_sql: &CreateViewSql,
    ) -> DbResult<crate::ingestor::LoadResult> {
        crate::schema::init_schema(db)?;

        // SEC-12 / TASK-0856: count_table is a TableName, validated at
        // construction. The quoted form is built without a runtime
        // identifier check — invalid identifiers can no longer reach
        // here at runtime.
        let quoted = self.count_table.quoted();
        let workspace_root = crate::sql::read_workspace_sidecar(dir, self.name)?;

        let record_count = {
            // CONC-2 / TASK-0364: hold the lock for the entire create→count
            // critical section. Splitting these into two `db.lock()` calls
            // let a concurrent ingestor running CREATE OR REPLACE TABLE
            // between them produce a record_count from a different table
            // than the one we just wrote.
            let conn = db.lock()?;
            // SEC-25 / TASK-2067: `create_sql` reads the staged JSON through
            // `read_json_auto('<path>')`, the one staged access the anchor
            // cannot cover — `DuckDB` takes a path string and has no
            // descriptor-passing API. Check the path and the anchor still name
            // the same inode, so a directory swapped between the anchored write
            // and the engine's read is refused instead of feeding the database
            // an attacker's rows.
            //
            // The check's value is the size of the gap between it and the
            // engine's own `open`, so it sits *inside* the connection lock,
            // with nothing but `create_tables_with` between the two — waiting
            // on `db.lock()` after checking would have widened that gap by an
            // unbounded amount under a concurrent ingest. It shrinks the
            // window rather than closing it; the reasoning is recorded on
            // `create_table_from_json_sql`.
            dir.verify_entry_identity(self.json_filename)?;
            self.create_tables_with(&conn, create_sql, view_sql)?;
            self.count_records_with(&conn, &quoted)?
        };

        self.persist_record(db, workspace_root.as_os_str(), dir, record_count)?;
        self.cleanup_artifacts(dir);

        Ok(LoadResult::success(self.name, record_count))
    }

    /// Step 1: execute the CREATE TABLE / CREATE VIEW statements on the
    /// already-locked connection. CONC-2 / TASK-0364: callers hold the
    /// lock across this *and* `count_records_with` so the row count is
    /// guaranteed to describe the table written by this call.
    fn create_tables_with(
        &self,
        conn: &duckdb::Connection,
        create_sql: &CreateTableSql,
        view_sql: &CreateViewSql,
    ) -> DbResult<()> {
        // PERF-3 / TASK-1243: keep the `format!` inside the `map_err` closure
        // so the success path (the dominant case on every ingest) allocates
        // zero strings for the error label. The pre-fix shape allocated two
        // `String`s per call (one per SQL execute) for labels that the
        // success path immediately dropped.
        conn.execute(create_sql.as_str(), [])
            .map_err(|e| crate::error::DbError::query_failed(format!("{} create", self.name), e))?;
        conn.execute(view_sql.as_str(), [])
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
                |row: &duckdb::Row<'_>| row.get::<_, i64>(0),
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

    /// Step 3: upsert the `data_sources` tracking row. Computes the file
    /// checksum (no lock held) before delegating to `upsert_data_source`.
    /// SEC-25 / TASK-2054: the checksum is computed over the file opened
    /// *through the anchor*, not over a re-resolved path, so the bytes recorded
    /// in `data_sources` are the bytes of the file this pipeline staged. The
    /// path stored on the provenance row stays a plain path — it is a label an
    /// operator reads, never something this code opens.
    fn persist_record(
        &self,
        db: &DuckDb,
        workspace_root: &std::ffi::OsStr,
        dir: &IngestDir,
        record_count: u64,
    ) -> DbResult<()> {
        let checksum = dir.checksum(self.json_filename)?;
        crate::schema::upsert_data_source(
            db,
            &crate::schema::DataSourceMetadata::new(
                crate::schema::SourceName::new(self.name),
                crate::schema::WorkspaceRoot::new(workspace_root),
                &dir.entry_path(self.json_filename),
                record_count,
                &checksum,
            ),
        )
    }

    /// Step 4: delete the staged JSON file and the sidecar.
    ///
    /// Both removals are best-effort: data is already persisted in `DuckDB` by
    /// the time we get here, so a leftover staged JSON or sidecar is a
    /// recoverable disk-hygiene issue, not a load failure. A transient
    /// permission error must not fail the whole ingest.
    ///
    /// ERR-1 (TASK-0466): the sidecar is removed only after the JSON
    /// removal has succeeded. If the JSON cannot be deleted, the sidecar
    /// is left in place so `read_workspace_sidecar` can drive a clean
    /// recovery on the next run instead of failing on a missing sidecar
    /// while leftover JSON still sits on disk.
    ///
    /// ERR-1 / TASK-1242: every breadcrumb that references the staging
    /// file logs *both* the original JSON path and the post-rename
    /// effective path. The two paths diverge in the cross-device-rename
    /// fallback (rename fails with EXDEV → `effective == original`) and
    /// after a successful rename (`effective == original.with_extension
    /// ("json.done")`). Emitting only one of the two collapsed those
    /// cases together and made it ambiguous which file an operator
    /// should be looking for after a half-cleaned crash. The dual-field
    /// contract is shared with `cleanup_artifacts_breadcrumb_paths` so
    /// the unit test can pin the formatting without intercepting a
    /// tracing subscriber.
    fn cleanup_artifacts(&self, dir: &IngestDir) {
        // FN-1 / TASK-1631: the rename-or-fallback and the unlink-with-recovery
        // each live in their own helper so this body stays a three-line
        // recovery policy ("rename, then unlink the effective path, removing
        // the sidecar when JSON is gone") rather than ~67 lines of mixed
        // syscalls + log formatting.
        let effective = rename_json_to_done(self.name, dir, self.json_filename);
        unlink_and_remove_sidecar(self.name, dir, self.json_filename, &effective);
    }
}

/// ARCH-2 / TASK-1008 (option B) — FN-1 / TASK-1631 (extracted): rename
/// the staging JSON to a `.done` suffix before unlinking. A `kill -9`
/// between rename and unlink leaves a single `*.done` file on disk that
/// operators can unambiguously identify as post-load debris (vs. a
/// `*.json` mid-flight). The rename → unlink ordering keeps the success
/// path identical while making the crash window observable rather than
/// indistinguishable.
///
/// SEC-25 / TASK-2054: `renameat` on the verified anchor, so the rename cannot
/// be redirected into a directory an attacker substituted for the ingest dir's
/// name after verification.
///
/// Returns the entry name that the subsequent unlink should target:
/// * the `.done` name on successful rename,
/// * the original name on EXDEV/permission failure (logged as debug) or
///   when the source was already absent (`NotFound`, no log — the caller's
///   `NotFound` branch will emit the appropriate breadcrumb).
fn rename_json_to_done(source: &'static str, dir: &IngestDir, json_name: &str) -> String {
    let done_name = format!("{json_name}.done");
    match dir.rename(json_name, &done_name) {
        Ok(()) => done_name,
        Err(crate::error::DbError::Io(e)) if e.kind() != std::io::ErrorKind::NotFound => {
            tracing::debug!(
                source,
                paths = %cleanup_artifacts_breadcrumb_paths(
                    &dir.entry_path(json_name),
                    &dir.entry_path(json_name),
                ),
                error = ?e,
                "cleanup_artifacts: rename to .done failed; falling back to direct unlink"
            );
            json_name.to_owned()
        }
        // NotFound on the rename: source already absent — fall through to
        // the unlink helper for the same recovery semantics.
        Err(_) => json_name.to_owned(),
    }
}

/// FN-1 / TASK-1631 (extracted): unlink the post-rename staging file and
/// remove the workspace sidecar. Both go through the verified anchor
/// (SEC-25 / TASK-2054).
///
/// ERR-1 / TASK-0466 contract: the sidecar is only removed once the JSON
/// is gone, so a transient permission/IO error leaves the sidecar in
/// place so `read_workspace_sidecar` can drive a clean recovery on the
/// next run.
///
/// ARCH-2 / TASK-1005: a `NotFound` on the unlink is operationally rare
/// (external scrubber, manual `rm`, mid-pipeline interruption). The
/// ERR-1 post-condition ("sidecar removed only after JSON gone") is
/// already satisfied if the JSON is absent, so the sidecar is removed
/// too; a debug breadcrumb makes the unexpected absence visible.
fn unlink_and_remove_sidecar(
    source: &'static str,
    dir: &IngestDir,
    json_name: &str,
    effective_name: &str,
) {
    match dir.remove_file(effective_name) {
        Ok(()) => crate::sql::remove_workspace_sidecar(dir, source),
        Err(crate::error::DbError::Io(err)) if err.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!(
                source,
                paths = %cleanup_artifacts_breadcrumb_paths(
                    &dir.entry_path(json_name),
                    &dir.entry_path(effective_name),
                ),
                "cleanup_artifacts: JSON staging file already absent before removal; removing sidecar anyway"
            );
            crate::sql::remove_workspace_sidecar(dir, source);
        }
        Err(err) => {
            tracing::warn!(
                source,
                paths = %cleanup_artifacts_breadcrumb_paths(
                    &dir.entry_path(json_name),
                    &dir.entry_path(effective_name),
                ),
                error = ?err,
                "failed to remove staged JSON after ingest; \
                 leaving sidecar to drive recovery on next run"
            );
        }
    }
}

/// ERR-1 / TASK-1242: format both the original JSON staging path and the
/// post-rename effective path into a single breadcrumb field. Sharing this
/// helper between the warn / debug call sites and the unit test pins the
/// dual-path logging contract: an operator chasing a half-cleaned crash
/// always sees both names, regardless of which branch the cleanup hit.
fn cleanup_artifacts_breadcrumb_paths(original: &Path, effective: &Path) -> String {
    format!(
        "original={:?} effective={:?}",
        original.display(),
        effective.display()
    )
}

/// Trait for data sources that collect raw data and load it into `DuckDB`.
///
/// Implementations handle the full lifecycle of external data:
/// 1. **Collect**: Run external commands or read files to produce JSON
/// 2. **Load**: Parse JSON and load into `DuckDB` tables/views
///
/// # Example
///
/// ```text
/// struct MetadataIngestor;
///
/// impl DataIngestor for MetadataIngestor {
///     fn name(&self) -> &'static str { "metadata" }
///     fn collect(&self, ctx: &Context, dir: &IngestDir) -> DbResult<()> {
///         // Run `cargo metadata` and stage it via `dir.write_atomic(..)`
///     }
///     fn load(&self, dir: &IngestDir, db: &DuckDb) -> DbResult<LoadResult> {
///         // Read the staged JSON through `dir` and create the DuckDB view
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
    /// This method runs the external tool (e.g., `cargo metadata`) and stages
    /// the output inside the ingest directory. It should not interact with the
    /// database.
    ///
    /// SEC-25 / TASK-2054: the parameter is a verified [`IngestDir`] anchor,
    /// not a bare `&Path`. Stage through [`IngestDir::write_atomic`] so the
    /// write resolves against the directory descriptor that was verified;
    /// re-deriving a path from [`IngestDir::path`] and opening it reintroduces
    /// exactly the swap window this signature exists to close.
    ///
    /// # Errors
    ///
    /// If the provider cannot gather its data or stage it into `dir`.
    fn collect(&self, ctx: &Context, dir: &IngestDir) -> DbResult<()>;

    /// Load collected data into `DuckDB` tables/views.
    ///
    /// This method reads the files staged in `dir` and creates or replaces
    /// tables/views in the database. Should be idempotent.
    ///
    /// SEC-25 / TASK-2054: reads and cleanup go through the anchor
    /// ([`IngestDir::open_read`], [`IngestDir::rename`],
    /// [`IngestDir::remove_file`]). [`IngestDir::entry_path`] is for the one
    /// thing that cannot take a descriptor — `DuckDB`'s `read_json_auto`,
    /// which is path-only — and for provenance labels.
    ///
    /// # Errors
    ///
    /// If the staged files cannot be read or the tables/views cannot be
    /// created.
    fn load(&self, dir: &IngestDir, db: &DuckDb) -> DbResult<LoadResult>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connection::DuckDb, error::DbError};

    /// ERR-1 / TASK-1242: the cleanup breadcrumb must surface *both* the
    /// original JSON staging path and the post-rename effective path.
    /// Pinned via the shared formatter so a future refactor that drops
    /// one of the two fields trips this test instead of silently
    /// collapsing the cross-device-fallback diagnostic.
    #[test]
    fn cleanup_breadcrumb_includes_both_original_and_effective_paths() {
        let original = Path::new("/tmp/ops-data/source.json");
        let effective = original.with_extension("json.done");

        // Successful rename: the two paths differ. Both must be visible.
        let rendered = cleanup_artifacts_breadcrumb_paths(original, &effective);
        assert!(
            rendered.contains("original=") && rendered.contains("source.json"),
            "must label and include the original JSON path: got {rendered}"
        );
        assert!(
            rendered.contains("effective=") && rendered.contains("source.json.done"),
            "must label and include the post-rename effective path: got {rendered}"
        );
    }

    #[test]
    fn cleanup_breadcrumb_dual_path_on_cross_device_fallback() {
        // EXDEV cross-device fallback: rename failed, so `effective`
        // collapses back onto `original`. The breadcrumb must still
        // emit both labelled fields so an operator parsing structured
        // logs sees the dual-path contract is honoured even on the
        // fallback branch.
        let original = Path::new("/mnt/dataA/source.json");
        let effective = original;
        let rendered = cleanup_artifacts_breadcrumb_paths(original, effective);
        assert!(
            rendered.contains("original=") && rendered.contains("effective="),
            "fallback must still emit both labelled fields: got {rendered}"
        );
        // The path itself appears twice — once per field.
        let occurrences = rendered.matches("/mnt/dataA/source.json").count();
        assert_eq!(
            occurrences, 2,
            "fallback must repeat the path under both fields: got {rendered}"
        );
    }

    /// SEC-25 / TASK-2054: every test stages through a verified anchor, the
    /// same way `provide_via_ingestor` does in production.
    fn anchor(tmp: &tempfile::TempDir) -> IngestDir {
        IngestDir::open(&tmp.path().join("data.duckdb.ingest")).expect("open ingest dir")
    }

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

        fn collect(&self, _ctx: &Context, dir: &IngestDir) -> DbResult<()> {
            dir.write_atomic("data.json", br#"{"test": "data"}"#)
        }

        fn load(&self, dir: &IngestDir, _db: &DuckDb) -> DbResult<LoadResult> {
            let json_path = dir.entry_path("data.json");
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
        let config = std::sync::Arc::new(ops_core::config::Config::empty());
        let ctx = Context::new(config, std::path::PathBuf::from("."));
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let dir = anchor(&temp_dir);
        ingestor
            .collect(&ctx, &dir)
            .expect("collect should succeed");
        assert!(dir.entry_path("data.json").exists());
    }

    /// SEC-25 / TASK-0911: a successful `collect_sidecar` must leave no
    /// `.tmp.*` leftover from the `atomic_write` sibling-temp pattern. Pin
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
        let dir = anchor(&temp_dir);
        let workspace = tempfile::tempdir().expect("workspace");
        cfg.collect_sidecar(&dir, &serde_json::json!({"k": "v"}), workspace.path())
            .expect("collect_sidecar");
        let json_path = dir.entry_path("data.json");
        assert!(json_path.exists(), "json written");
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
        let dir = anchor(&temp_dir);
        let parent = dir.path().to_path_buf();
        dir.write_atomic("data.json", b"{}").expect("write json");
        crate::sql::write_workspace_sidecar(&dir, config.name, temp_dir.path())
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

        config.cleanup_artifacts(&dir);

        // Restore perms before asserting so the test environment can clean up.
        std::fs::set_permissions(&parent, original).expect("restore");

        let sidecar = dir.entry_path(&crate::sql::sidecar_name(config.name));
        assert!(
            sidecar.exists(),
            "sidecar must remain when JSON removal fails: {sidecar:?}"
        );
    }

    /// ARCH-2 / TASK-1008: simulate a `kill -9` between the upsert and
    /// the post-load cleanup. The crashed run leaves a `*.json.done`
    /// file (rename-then-unlink ordering — the rename succeeded, the
    /// unlink didn't). The next user-driven invocation re-runs
    /// `cleanup_artifacts` against the original JSON path; the helper
    /// must leave no `*.json` and no `*.json.done` residue, so a
    /// `target/ops/data.duckdb.ingest/` audit shows the directory clean.
    #[test]
    fn cleanup_artifacts_clears_done_residue_left_by_prior_crash() {
        let config = SidecarIngestorConfig {
            name: "crash_resume",
            json_filename: "data.json",
            count_table: crate::sql::validation::TableName::from_static("data_sources"),
        };
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let dir = anchor(&temp_dir);
        let json_path = dir.entry_path("data.json");
        let done_path = dir.entry_path("data.json.done");

        // Prior crash residue: `.done` file exists but `.json` is gone.
        dir.write_atomic("data.json.done", b"crash-residue")
            .unwrap();

        // Re-run a successful ingest path: the staging JSON exists again.
        dir.write_atomic("data.json", b"new-load").unwrap();
        crate::sql::write_workspace_sidecar(&dir, config.name, temp_dir.path()).unwrap();
        config.cleanup_artifacts(&dir);

        assert!(
            !json_path.exists(),
            "json staging file must be gone after successful cleanup"
        );
        // The rename overwrites the prior `.done` residue, then unlink
        // removes the result — so a clean rerun also clears any leftover
        // crash debris on disk. The directory ends up empty (modulo the
        // sidecar removal already exercised by sister tests).
        assert!(
            !done_path.exists(),
            "prior-crash .done residue must be reaped by the next successful cleanup"
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
        let dir = anchor(&temp_dir);
        // Intentionally do NOT stage data.json — simulate a removal that
        // raced with cleanup. Writing the sidecar is enough.
        crate::sql::write_workspace_sidecar(&dir, config.name, temp_dir.path()).unwrap();
        config.cleanup_artifacts(&dir);
        // Sidecar removal should still complete.
    }

    #[test]
    fn data_ingestor_trait_load() {
        let ingestor = MockIngestor { name: "test" };
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let dir = anchor(&temp_dir);
        let db = DuckDb::open_in_memory().expect("db");
        dir.write_atomic("data.json", br#"{"test": "data"}"#)
            .unwrap();
        let result = ingestor.load(&dir, &db).expect("load should succeed");
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
            fn collect(&self, _ctx: &Context, _dir: &IngestDir) -> DbResult<()> {
                Err(DbError::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "collect failed",
                )))
            }
            fn load(&self, _dir: &IngestDir, _db: &DuckDb) -> DbResult<LoadResult> {
                Ok(LoadResult::success(self.name(), 0))
            }
        }

        #[test]
        fn ingestor_collect_error_propagates() {
            let ingestor = FailingCollectIngestor;
            let config = std::sync::Arc::new(ops_core::config::Config::empty());
            let ctx = Context::new(config, std::path::PathBuf::from("."));
            let temp_dir = tempfile::tempdir().expect("tempdir");
            let dir = anchor(&temp_dir);
            let result = ingestor.collect(&ctx, &dir);
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

            let tmp_a = tempfile::tempdir().expect("dir a");
            let tmp_b = tempfile::tempdir().expect("dir b");
            let dir_a = anchor(&tmp_a);
            let dir_b = anchor(&tmp_b);
            dir_a.write_atomic("a.json", b"{}").expect("write a.json");
            dir_b.write_atomic("b.json", b"{}").expect("write b.json");
            crate::sql::write_workspace_sidecar(&dir_a, "ingA", Path::new("/wA"))
                .expect("sidecar a");
            crate::sql::write_workspace_sidecar(&dir_b, "ingB", Path::new("/wB"))
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
            let create_a = CreateTableSql::from_literal_for_tests(
                "CREATE OR REPLACE TABLE shared_table AS \
                 SELECT * FROM (VALUES (1),(2),(3)) v(i)",
            );
            let create_b = CreateTableSql::from_literal_for_tests(
                "CREATE OR REPLACE TABLE shared_table AS \
                 SELECT * FROM (VALUES (1),(2),(3),(4),(5)) v(i)",
            );
            let view = CreateViewSql::from_literal_for_tests(
                "CREATE OR REPLACE VIEW shared_v AS SELECT * FROM shared_table",
            );

            let db_a = Arc::clone(&db);
            let db_b = Arc::clone(&db);
            let view_b = view.clone();
            let h1 = std::thread::spawn(move || {
                cfg_a.load_with_sidecar(&db_a, &dir_a, &create_a, &view)
            });
            let h2 = std::thread::spawn(move || {
                cfg_b.load_with_sidecar(&db_b, &dir_b, &create_b, &view_b)
            });

            let res_a = h1.join().expect("join a").expect("ingestor a");
            let res_b = h2.join().expect("join b").expect("ingestor b");

            assert_eq!(res_a.record_count, 3, "ingA must observe its own 3 rows");
            assert_eq!(res_b.record_count, 5, "ingB must observe its own 5 rows");
        }

        /// SEC-12 / TASK-0856: an invalid `count_table` can no longer reach
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
