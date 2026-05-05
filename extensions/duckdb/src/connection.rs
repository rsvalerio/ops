//! DuckDb connection wrapper and path resolution.

use crate::error::{DbError, DbResult};
use ops_core::config::DataConfig;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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
/// For typical ops usage (single command execution at a time), this is acceptable.
pub struct DuckDb {
    conn: Mutex<duckdb::Connection>,
    #[allow(dead_code)]
    db_path: PathBuf,
    /// Per-table ingest locks scoped to this `DuckDb` instance.
    ///
    /// CONC-7 (TASK-0779): keying by table name and storing the map in
    /// the connection bounds growth to the database schema and releases
    /// every entry when the instance is dropped, instead of leaking
    /// `(db_path, table)` tuples in a process-global `OnceLock` for the
    /// lifetime of the binary.
    /// PERF-3 / TASK-1007: keyed by `&'static str` so `ingest_mutex_for`
    /// looks up an existing entry without paying the per-call
    /// `String::to_owned` allocation that `HashMap<String, _>::entry`
    /// charges on every probe. All call sites already pass static
    /// literals; the signature change makes a future dynamic key a build
    /// error rather than a silent regression.
    ingest_locks: Mutex<HashMap<&'static str, Arc<Mutex<()>>>>,
}

#[allow(dead_code)]
impl DuckDb {
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
            ingest_locks: Mutex::new(HashMap::new()),
        })
    }

    /// Open a database at the given path in read-only mode.
    ///
    /// READ-5 (TASK-0525): unlike [`Self::open`], this does **not**
    /// `create_dir_all` the parent directory. A read-only opener that
    /// creates writable directories on disk would contradict the access
    /// mode the caller requested — an unresolvable path is the caller's
    /// signal that the DB has not been provisioned yet, not an invitation
    /// for the read path to mutate the filesystem. The asymmetry with
    /// `open` is intentional and the resulting `DuckDb`-level error
    /// (rather than a more friendly mkdir error) is the price of that
    /// honesty.
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
            ingest_locks: Mutex::new(HashMap::new()),
        })
    }

    /// Open an in-memory database (for tests).
    pub fn open_in_memory() -> DbResult<Self> {
        let conn = duckdb::Connection::open_in_memory().map_err(DbError::DuckDb)?;
        Ok(Self {
            conn: Mutex::new(conn),
            db_path: PathBuf::from(":memory:"),
            ingest_locks: Mutex::new(HashMap::new()),
        })
    }

    /// Resolved absolute path to the database file.
    pub fn path(&self) -> &Path {
        &self.db_path
    }

    /// Lock the connection for exclusive use.
    pub fn lock(&self) -> DbResult<std::sync::MutexGuard<'_, duckdb::Connection>> {
        // Intentionally no logging here: callers know the query/operation
        // context and are responsible for either propagating or logging
        // (READ-8). A library primitive should not double-log.
        self.conn
            .lock()
            .map_err(|e| DbError::MutexPoisoned(e.to_string()))
    }

    /// Return the per-table ingest mutex, creating it on first use.
    ///
    /// ERR-5 (TASK-0780): the registry mutex recovers from poisoning via
    /// `into_inner` so a panic inside one ingestor's `collect`/`load` does
    /// not permanently brick every other ingest. The connection lock at
    /// `Self::lock` continues to surface poisoning as
    /// [`DbError::MutexPoisoned`] because a poisoned DuckDB connection
    /// reflects partially applied state we cannot trust to keep using; a
    /// poisoned per-table coordination mutex only guards a `()`, so
    /// recovering is safe and avoids the documented denial-of-service.
    ///
    /// Operator signal: when a prior panic poisons the per-table mutex,
    /// `provide_via_ingestor` emits `tracing::warn!` on recovery, so a
    /// transient ingest panic leaves an audit breadcrumb in production
    /// logs rather than recovering silently.
    pub(crate) fn ingest_mutex_for(&self, table_name: &'static str) -> Arc<Mutex<()>> {
        let mut map = self.ingest_locks.lock().unwrap_or_else(|poisoned| {
            tracing::warn!("ingest_locks registry mutex was poisoned by a prior panic; recovered");
            poisoned.into_inner()
        });
        // PERF-3 / TASK-1007: with the `&'static str` key, `entry` consumes
        // a `Copy` &'static reference instead of allocating a fresh
        // `String` on every probe. The hot path (entry already present)
        // is now alloc-free.
        Arc::clone(map.entry(table_name).or_default())
    }

    #[cfg(test)]
    pub(crate) fn ingest_lock_count(&self) -> usize {
        self.ingest_locks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
    }

    /// Resolve the DB path from config and workspace root.
    /// If config.data.path is set, resolve it (absolute or relative to workspace_root).
    /// Otherwise default to workspace_root/target/ops/data.duckdb.
    pub fn resolve_path(config: &DataConfig, workspace_root: &Path) -> PathBuf {
        match &config.path {
            None => workspace_root
                .join("target")
                .join("ops")
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
        let path = DuckDb::resolve_path(&config, root);
        assert_eq!(path, PathBuf::from("/home/proj/target/ops/data.duckdb"));
    }

    #[test]
    fn resolve_path_relative() {
        let config = DataConfig {
            path: Some(PathBuf::from(".ops-data/project.duckdb")),
        };
        let root = Path::new("/home/proj");
        let path = DuckDb::resolve_path(&config, root);
        assert_eq!(path, PathBuf::from("/home/proj/.ops-data/project.duckdb"));
    }

    #[test]
    fn resolve_path_absolute() {
        let config = DataConfig {
            path: Some(PathBuf::from("/absolute/shared.duckdb")),
        };
        let root = Path::new("/home/proj");
        let path = DuckDb::resolve_path(&config, root);
        assert_eq!(path, PathBuf::from("/absolute/shared.duckdb"));
    }

    /// TQ-004: Test DuckDb error path handling.
    mod error_path_tests {
        use super::*;

        #[test]
        fn duck_db_open_in_memory_succeeds() {
            let result = DuckDb::open_in_memory();
            assert!(result.is_ok(), "in-memory DB should always succeed");
        }

        #[test]
        fn duck_db_open_creates_parent_directory() {
            let dir = tempfile::tempdir().expect("tempdir");
            let db_path = dir.path().join("subdir/nested/db.duckdb");
            let result = DuckDb::open(&db_path);

            assert!(result.is_ok(), "should create parent directories");
            assert!(db_path.exists(), "db file should exist");
        }

        #[test]
        fn duck_db_lock_returns_guard() {
            let db = DuckDb::open_in_memory().expect("open");
            let guard = db.lock();
            assert!(guard.is_ok(), "lock should succeed");
        }

        #[test]
        fn duck_db_path_returns_stored_path() {
            let db = DuckDb::open_in_memory().expect("open");
            assert_eq!(db.path(), Path::new(":memory:"));
        }

        #[test]
        fn duck_db_open_readonly_nonexistent_fails() {
            let result = DuckDb::open_readonly(Path::new("/nonexistent/path/db.duckdb"));
            assert!(result.is_err(), "readonly open of nonexistent should fail");
        }

        /// READ-5 (TASK-0525): pin the asymmetry — `open_readonly` does not
        /// create parent directories, and the resulting error is a duckdb
        /// error rather than the parent-mkdir IO error `open` produces.
        #[test]
        fn duck_db_open_readonly_does_not_create_parent_dir() {
            let dir = tempfile::tempdir().expect("tempdir");
            let missing_parent = dir.path().join("does/not/exist");
            let db_path = missing_parent.join("db.duckdb");
            let result = DuckDb::open_readonly(&db_path);
            assert!(result.is_err(), "readonly open of nonexistent should fail");
            assert!(
                !missing_parent.exists(),
                "open_readonly must not mkdir parent: {:?}",
                missing_parent
            );
        }
    }
}
