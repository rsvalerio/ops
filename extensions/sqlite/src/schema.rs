//! Schema initialization and tracking for `Sqlite`.

use crate::connection::Sqlite;
use crate::error::{DbError, DbResult};
use std::path::Path;

/// Create the `data_sources` tracking table if it does not exist.
///
/// # Errors
///
/// [`DbError::MutexPoisoned`] if the connection lock is poisoned, or
/// [`DbError::Sqlite`] if the schema batch fails to execute.
pub fn init_schema(db: &Sqlite) -> DbResult<()> {
    let conn = db.lock()?;
    conn.execute_batch(
        r"
        CREATE TABLE IF NOT EXISTS data_sources (
            source_name    TEXT NOT NULL,
            workspace_root TEXT NOT NULL,
            loaded_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            source_path    TEXT NOT NULL,
            record_count   INTEGER NOT NULL DEFAULT 0,
            checksum       TEXT NOT NULL,
            metadata       TEXT,
            PRIMARY KEY (source_name, workspace_root)
        );
        ",
    )
    .map_err(|e| DbError::query_failed("init_schema", e))?;
    drop(conn);
    Ok(())
}

/// The `source_name` half of the `data_sources` primary key.
///
/// A distinct type from [`WorkspaceRoot`] so the two adjacent key arguments
/// of [`DataSourceMetadata::new`] cannot be swapped: a swap would write the
/// row under the wrong key, and it is a compile error instead.
#[derive(Debug, Clone, Copy)]
pub struct SourceName<'a>(&'a str);

impl<'a> SourceName<'a> {
    /// Wraps `name` as a data-source name.
    #[must_use]
    pub const fn new(name: &'a str) -> Self {
        Self(name)
    }

    /// The wrapped source name.
    #[must_use]
    pub const fn as_str(&self) -> &'a str {
        self.0
    }
}

/// The `workspace_root` half of the `data_sources` primary key.
///
/// Holds the root's raw OS string; see [`SourceName`] for why the two key
/// halves are distinct types.
#[derive(Debug, Clone, Copy)]
pub struct WorkspaceRoot<'a>(&'a std::ffi::OsStr);

impl<'a> WorkspaceRoot<'a> {
    /// Wraps `root` as a workspace root.
    #[must_use]
    pub const fn new(root: &'a std::ffi::OsStr) -> Self {
        Self(root)
    }

    /// The wrapped workspace root.
    #[must_use]
    pub const fn as_os_str(&self) -> &'a std::ffi::OsStr {
        self.0
    }
}

/// Metadata describing a loaded data source row.
#[non_exhaustive]
pub struct DataSourceMetadata<'a> {
    /// Name of the source that was loaded; first half of the primary key.
    pub source_name: &'a str,
    /// Workspace the data was collected from; second half of the primary key.
    pub workspace_root: &'a std::ffi::OsStr,
    /// Path of the staged file the rows were loaded from, as a label.
    pub source_path: &'a Path,
    /// Rows the load landed in the database.
    pub record_count: u64,
    /// SHA-256 of the staged file, as lowercase hex.
    pub checksum: &'a str,
}

impl<'a> DataSourceMetadata<'a> {
    /// Builds the row for `source_name` in `workspace_root`.
    #[must_use]
    pub const fn new(
        source_name: SourceName<'a>,
        workspace_root: WorkspaceRoot<'a>,
        source_path: &'a Path,
        record_count: u64,
        checksum: &'a str,
    ) -> Self {
        Self {
            source_name: source_name.0,
            workspace_root: workspace_root.0,
            source_path,
            record_count,
            checksum,
        }
    }
}

/// Upsert a `data_sources` row after a load.
///
/// A path persisted here must map back to the actual file, so a `source_path`
/// or workspace root that is not valid UTF-8 is rejected with
/// [`DbError::NonUtf8Path`] rather than stored through a lossy conversion.
/// `ops_about::identity::build_identity_value` applies the same policy to the
/// `project_root` it serializes.
///
/// # Errors
///
/// [`DbError::NonUtf8Path`] if `source_path` or the workspace root is not
/// valid UTF-8, or [`DbError::Sqlite`] if the upsert fails.
pub fn upsert_data_source(db: &Sqlite, meta: &DataSourceMetadata<'_>) -> DbResult<()> {
    let path_str = meta
        .source_path
        .to_str()
        .ok_or_else(|| DbError::NonUtf8Path(meta.source_path.as_os_str().to_os_string()))?;
    // `read_workspace_sidecar` preserves raw OS bytes verbatim, so the root
    // may not be UTF-8; a lossy conversion would ship a garbled key into the
    // `(source_name, workspace_root)` primary key.
    let workspace_root_str = meta
        .workspace_root
        .to_str()
        .ok_or_else(|| DbError::NonUtf8Path(meta.workspace_root.to_os_string()))?;
    let record_count_i64 = i64::try_from(meta.record_count)
        .map_err(|_| DbError::RecordCountOverflow(meta.record_count))?;
    let conn = db.lock()?;
    conn.execute(
        r"
        INSERT INTO data_sources (source_name, workspace_root, source_path, record_count, checksum)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT (source_name, workspace_root) DO UPDATE SET
            loaded_at = CURRENT_TIMESTAMP,
            source_path = excluded.source_path,
            record_count = excluded.record_count,
            checksum = excluded.checksum
        ",
        rusqlite::params![
            meta.source_name,
            workspace_root_str,
            path_str,
            record_count_i64,
            meta.checksum
        ],
    )
    .map_err(|e| DbError::query_failed("upsert_data_source", e))?;
    drop(conn);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Sqlite;
    use std::path::Path;

    #[test]
    fn init_schema_creates_data_sources() {
        let db = Sqlite::open_in_memory().unwrap();
        init_schema(&db).unwrap();
        let conn = db.lock().unwrap();
        conn.execute("SELECT 1 FROM data_sources LIMIT 0", [])
            .unwrap();
    }

    /// The checksum stored for `(source_name, workspace_root)`, if any.
    fn stored_checksum(db: &Sqlite, source_name: &str, workspace_root: &str) -> Option<String> {
        use rusqlite::OptionalExtension as _;
        let conn = db.lock().unwrap();
        let checksum = conn
            .query_row(
                "SELECT checksum FROM data_sources WHERE source_name = ? AND workspace_root = ?",
                rusqlite::params![source_name, workspace_root],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .unwrap();
        drop(conn);
        checksum
    }

    #[test]
    fn data_sources_is_empty_after_init() {
        let db = Sqlite::open_in_memory().unwrap();
        init_schema(&db).unwrap();
        assert!(stored_checksum(&db, "metadata", "/ws").is_none());
    }

    #[test]
    #[cfg(unix)]
    fn upsert_data_source_rejects_non_utf8_path() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let db = Sqlite::open_in_memory().unwrap();
        init_schema(&db).unwrap();
        let bytes = b"/ws/\xff\xfe.json";
        let bad_path = std::path::Path::new(OsStr::from_bytes(bytes));
        let result = upsert_data_source(
            &db,
            &DataSourceMetadata::new(
                SourceName::new("metadata"),
                WorkspaceRoot::new(std::ffi::OsStr::new("/ws")),
                bad_path,
                1,
                "abc",
            ),
        );
        assert!(matches!(result, Err(DbError::NonUtf8Path(_))));
    }

    /// ERR-1 (TASK-0885): the column must hold i64 so counts exceeding
    /// `i32::MAX` round-trip without truncation or driver-level bind error
    /// (SQLite INTEGER is i64 natively).
    #[test]
    fn record_count_over_i32_max_round_trips() {
        let db = Sqlite::open_in_memory().unwrap();
        init_schema(&db).unwrap();
        // `i32::MAX` is positive, so `unsigned_abs` is an exact widening to
        // `u32` and `u64::from` an exact widening from there.
        let big = u64::from(i32::MAX.unsigned_abs()) + 7;
        upsert_data_source(
            &db,
            &DataSourceMetadata::new(
                SourceName::new("big"),
                WorkspaceRoot::new(std::ffi::OsStr::new("/ws")),
                Path::new("/ws/target/ops/big.json"),
                big,
                "abc",
            ),
        )
        .unwrap();
        let conn = db.lock().unwrap();
        let stored: i64 = conn
            .query_row(
                "SELECT record_count FROM data_sources WHERE source_name = ? AND workspace_root = ?",
                rusqlite::params!["big", "/ws"],
                |r| r.get(0),
            )
            .unwrap();
        drop(conn);
        // The stored INTEGER must be exactly `big`; an out-of-`u64` (i.e.
        // negative) value fails the assertion instead of wrapping silently.
        assert_eq!(u64::try_from(stored).ok(), Some(big));
    }

    #[test]
    fn upsert_stores_and_replaces_the_checksum() {
        let db = Sqlite::open_in_memory().unwrap();
        init_schema(&db).unwrap();
        upsert_data_source(
            &db,
            &DataSourceMetadata::new(
                SourceName::new("metadata"),
                WorkspaceRoot::new(std::ffi::OsStr::new("/ws")),
                Path::new("/ws/target/ops/metadata.json"),
                1,
                "abc123",
            ),
        )
        .unwrap();
        assert_eq!(
            stored_checksum(&db, "metadata", "/ws").as_deref(),
            Some("abc123")
        );

        upsert_data_source(
            &db,
            &DataSourceMetadata::new(
                SourceName::new("metadata"),
                WorkspaceRoot::new(std::ffi::OsStr::new("/ws")),
                Path::new("/ws/target/ops/metadata.json"),
                2,
                "def456",
            ),
        )
        .unwrap();
        assert_eq!(
            stored_checksum(&db, "metadata", "/ws").as_deref(),
            Some("def456")
        );
    }
}
