//! OpsDb connection wrapper and path resolution.

use crate::config::DataConfig;
use crate::extensions::ops_db::error::{DbError, DbResult};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Thread-safe DuckDB connection wrapper.
///
/// # Concurrency Design (EFF-001)
///
/// Uses `Mutex<Connection>` which serializes all database operations. DuckDB itself
/// supports concurrent reads, but the Rust `duckdb` crate's `Connection` type is not
/// thread-safe for concurrent use. This design choice:
///
/// - **Pros**: Simple, safe, no risk of data races
/// - **Cons**: All DB operations are serialized, potential bottleneck under load
///
/// If read-heavy concurrent access becomes a performance issue, consider:
/// 1. Opening multiple read-only connections
/// 2. Using connection pooling
/// 3. Moving to `RwLock` if/when the duckdb crate supports concurrent reads
///
/// For typical cargo-ops usage (single command execution at a time), this is acceptable.
pub struct OpsDb {
    conn: Mutex<duckdb::Connection>,
    #[allow(dead_code)]
    db_path: PathBuf,
}

#[allow(dead_code)]
impl OpsDb {
    /// Open (or create) a database at the given path, read-write.
    pub fn open(path: &Path) -> DbResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(DbError::Io)?;
        }
        let conn = duckdb::Connection::open(path).map_err(DbError::DuckDb)?;
        let db_path = path.to_path_buf();
        Ok(Self {
            conn: Mutex::new(conn),
            db_path,
        })
    }

    /// Open a database at the given path in read-only mode.
    pub fn open_readonly(path: &Path) -> DbResult<Self> {
        let path = path.to_path_buf();
        let conn = duckdb::Connection::open_with_flags(
            &path,
            duckdb::Config::default()
                .access_mode(duckdb::AccessMode::ReadOnly)
                .map_err(DbError::DuckDb)?,
        )
        .map_err(DbError::DuckDb)?;
        Ok(Self {
            conn: Mutex::new(conn),
            db_path: path,
        })
    }

    /// Open an in-memory database (for tests).
    pub fn open_in_memory() -> DbResult<Self> {
        let conn = duckdb::Connection::open_in_memory().map_err(DbError::DuckDb)?;
        Ok(Self {
            conn: Mutex::new(conn),
            db_path: PathBuf::from(":memory:"),
        })
    }

    /// Resolved absolute path to the database file.
    pub fn path(&self) -> &Path {
        &self.db_path
    }

    /// Lock the connection for exclusive use.
    pub(crate) fn lock(&self) -> DbResult<std::sync::MutexGuard<'_, duckdb::Connection>> {
        self.conn.lock().map_err(|e| {
            tracing::warn!("db mutex poisoned");
            DbError::MutexPoisoned(e.to_string())
        })
    }

    /// Resolve the DB path from config and workspace root.
    /// If config.data.path is set, resolve it (absolute or relative to workspace_root).
    /// Otherwise default to workspace_root/target/cargo-ops/data.duckdb.
    pub fn resolve_path(config: &DataConfig, workspace_root: &Path) -> PathBuf {
        match &config.path {
            None => workspace_root
                .join("target")
                .join("cargo-ops")
                .join("data.duckdb"),
            Some(p) => {
                if p.is_absolute() {
                    p.clone()
                } else {
                    workspace_root.join(p)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_path_default() {
        let config = DataConfig::default();
        let root = Path::new("/home/proj");
        let path = OpsDb::resolve_path(&config, root);
        assert_eq!(
            path,
            PathBuf::from("/home/proj/target/cargo-ops/data.duckdb")
        );
    }

    #[test]
    fn resolve_path_relative() {
        let config = DataConfig {
            path: Some(PathBuf::from(".ops-data/project.duckdb")),
        };
        let root = Path::new("/home/proj");
        let path = OpsDb::resolve_path(&config, root);
        assert_eq!(path, PathBuf::from("/home/proj/.ops-data/project.duckdb"));
    }

    #[test]
    fn resolve_path_absolute() {
        let config = DataConfig {
            path: Some(PathBuf::from("/absolute/shared.duckdb")),
        };
        let root = Path::new("/home/proj");
        let path = OpsDb::resolve_path(&config, root);
        assert_eq!(path, PathBuf::from("/absolute/shared.duckdb"));
    }

    /// TQ-004: Test OpsDb error path handling.
    mod error_path_tests {
        use super::*;

        #[test]
        fn ops_db_open_in_memory_succeeds() {
            let result = OpsDb::open_in_memory();
            assert!(result.is_ok(), "in-memory DB should always succeed");
        }

        #[test]
        fn ops_db_open_creates_parent_directory() {
            let dir = tempfile::tempdir().expect("tempdir");
            let db_path = dir.path().join("subdir/nested/db.duckdb");
            let result = OpsDb::open(&db_path);

            assert!(result.is_ok(), "should create parent directories");
            assert!(db_path.exists(), "db file should exist");
        }

        #[test]
        fn ops_db_lock_returns_guard() {
            let db = OpsDb::open_in_memory().expect("open");
            let guard = db.lock();
            assert!(guard.is_ok(), "lock should succeed");
        }

        #[test]
        fn ops_db_path_returns_stored_path() {
            let db = OpsDb::open_in_memory().expect("open");
            assert_eq!(db.path(), Path::new(":memory:"));
        }

        #[test]
        fn ops_db_open_readonly_nonexistent_fails() {
            let result = OpsDb::open_readonly(Path::new("/nonexistent/path/db.duckdb"));
            assert!(result.is_err(), "readonly open of nonexistent should fail");
        }
    }
}
