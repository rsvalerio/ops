//! `Sqlite` connection wrapper and path resolution.

use crate::error::{DbError, DbResult};
use ops_core::config::DataConfig;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The SQLite in-memory connection string, stored as `Sqlite::path()` for
/// handles opened via [`Sqlite::open_in_memory`].
///
/// READ-5 / TASK-1867: it is **not** a filesystem path. Code that derives a
/// path from `Sqlite::path()` must reject it rather than treat it as one.
pub const IN_MEMORY_PATH: &str = ":memory:";

/// How long a statement waits for a competing SQLite lock before failing.
///
/// ops is single-process today, so this never fires in practice; it is cheap
/// insurance for the day two ops invocations (or an ops run plus an
/// interactive `sqlite3` session) overlap on the same file. Without it,
/// SQLite's default is to fail *immediately* with `SQLITE_BUSY`.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// ARCH-9 / TASK-1155: process-wide monotonic counter that mints a fresh
/// `Sqlite::id` per instance. Stable for the lifetime of the instance, and
/// guaranteed distinct from every previously-minted id, so callers keying
/// caches on the id avoid the pointer-address ABA hazard the prior
/// `std::ptr::from_ref(db) as usize` scheme had.
fn mint_db_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Thread-safe SQLite connection wrapper.
///
/// # Concurrency Design (EFF-001)
///
/// Uses `Mutex<Connection>` which serializes all database operations.
/// `rusqlite::Connection` is `Send` but not `Sync`, so this design choice:
///
/// - **Pros**: Simple, safe, no risk of data races
/// - **Cons**: All DB operations are serialized, potential bottleneck under load
///
/// If read-heavy concurrent access becomes a performance issue, consider:
/// 1. Opening multiple read-only connections
/// 2. Using connection pooling
/// 3. Moving to `RwLock` over a connection-per-reader pool
///
/// For typical ops usage (single command execution at a time), this is acceptable.
pub struct Sqlite {
    conn: Mutex<rusqlite::Connection>,
    db_path: PathBuf,
    /// ARCH-9 / TASK-1155: stable per-instance identity used by callers
    /// that key process-local caches by `Sqlite` identity. The previous
    /// pattern (`std::ptr::from_ref(db) as usize`) was vulnerable to
    /// pointer-address ABA — a dropped-and-replaced `Sqlite` could
    /// re-allocate at the same address and silently return a previous
    /// instance's cached value. Minted from a process-wide monotonic
    /// counter so two distinct instances always receive distinct ids
    /// regardless of allocation reuse.
    id: u64,
    /// Per-table ingest locks scoped to this `Sqlite` instance.
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

// READ-10 / TASK-1873: no impl-wide `#[allow(dead_code)]`. Suppressing at the
// block level hid every future unused constructor and method as well. Nothing
// in this block is dead today; if something becomes API-surface-only, give it
// its own `#[expect(dead_code, reason = "…")]` rather than restoring a
// container-wide allow.
impl Sqlite {
    /// Open (or create) a database at the given path, read-write.
    ///
    /// # Errors
    ///
    /// [`DbError::Io`] if the parent directory cannot be created, or
    /// [`DbError::Sqlite`] if the database cannot be opened.
    pub fn open(path: &Path) -> DbResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(DbError::Io)?;
        }
        let conn = open_conn(path, rusqlite::OpenFlags::default())?;
        Ok(Self {
            conn: Mutex::new(conn),
            db_path: path.to_path_buf(),
            id: mint_db_id(),
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
    /// `open` is intentional and the resulting `Sqlite`-level error
    /// (rather than a more friendly mkdir error) is the price of that
    /// honesty.
    ///
    /// # Errors
    ///
    /// [`DbError::Sqlite`] if the read-only access mode cannot be configured or
    /// the database cannot be opened.
    pub fn open_readonly(path: &Path) -> DbResult<Self> {
        let path = path.to_path_buf();
        // SQLITE_OPEN_NO_MUTEX: the handle is already serialized behind the
        // crate's own `Mutex`, so SQLite's internal mutex is redundant.
        // SQLITE_OPEN_READ_ONLY implies no-create; URI is off, so a literal
        // path is taken as-is.
        let flags =
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = open_conn(&path, flags)?;
        Ok(Self {
            conn: Mutex::new(conn),
            db_path: path,
            id: mint_db_id(),
            ingest_locks: Mutex::new(HashMap::new()),
        })
    }

    /// Open an in-memory database (for tests).
    ///
    /// READ-5 / TASK-1867: `db_path` is set to the `IN_MEMORY_PATH`
    /// sentinel, which is a SQLite connection string rather than a
    /// filesystem path. Anything deriving a path from `path()` must reject
    /// it — see `data_dir_for_db`.
    ///
    /// # Errors
    ///
    /// [`DbError::Sqlite`] if the in-memory database cannot be opened.
    pub fn open_in_memory() -> DbResult<Self> {
        let conn = rusqlite::Connection::open_in_memory().map_err(DbError::Sqlite)?;
        let conn = tune(conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            db_path: PathBuf::from(IN_MEMORY_PATH),
            id: mint_db_id(),
            ingest_locks: Mutex::new(HashMap::new()),
        })
    }

    /// ARCH-9 / TASK-1155: stable per-instance identity for keying
    /// process-local caches by `Sqlite` identity. Distinct instances always
    /// receive distinct ids regardless of allocation reuse, eliminating
    /// the ABA hazard the prior pointer-address scheme had.
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Resolved absolute path to the database file.
    pub fn path(&self) -> &Path {
        &self.db_path
    }

    /// Lock the connection for exclusive use.
    ///
    /// # Errors
    ///
    /// [`DbError::MutexPoisoned`] if another thread panicked while holding the
    /// connection lock.
    pub fn lock(&self) -> DbResult<std::sync::MutexGuard<'_, rusqlite::Connection>> {
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
    /// [`DbError::MutexPoisoned`] because a poisoned SQLite connection
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
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }

    /// Resolve the DB path from config and workspace root.
    /// If config.data.path is set, resolve it (absolute or relative to `workspace_root`).
    /// Otherwise default to `workspace_root/target/ops/data.db`.
    #[must_use]
    pub fn resolve_path(config: &DataConfig, workspace_root: &Path) -> PathBuf {
        config.path.as_ref().map_or_else(
            || workspace_root.join("target").join("ops").join("data.db"),
            |p| {
                if p.is_absolute() {
                    p.clone()
                } else {
                    workspace_root.join(p)
                }
            },
        )
    }
}

/// Open a connection with `flags` and apply the shared pragmas.
///
/// # Errors
///
/// [`DbError::Sqlite`] if the open fails or a pragma statement errors.
fn open_conn(path: &Path, flags: rusqlite::OpenFlags) -> DbResult<rusqlite::Connection> {
    let conn = rusqlite::Connection::open_with_flags(path, flags).map_err(DbError::Sqlite)?;
    tune(conn)
}

/// Apply connection-level settings shared by every open mode.
///
/// `busy_timeout` converts a would-be immediate `SQLITE_BUSY` failure into a
/// bounded wait (see [`BUSY_TIMEOUT`]). `foreign_keys` is on because the
/// schema uses composite primary keys rather than FKs today, but leaving the
/// default (off) would surprise the first future FK with silent referential
/// drift.
fn tune(conn: rusqlite::Connection) -> DbResult<rusqlite::Connection> {
    conn.busy_timeout(BUSY_TIMEOUT)
        .and_then(|()| conn.pragma_update(None, "foreign_keys", "ON"))
        .map_err(DbError::Sqlite)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_path_default() {
        let config = DataConfig::default();
        let root = Path::new("/home/proj");
        let path = Sqlite::resolve_path(&config, root);
        assert_eq!(path, PathBuf::from("/home/proj/target/ops/data.db"));
    }

    #[test]
    fn resolve_path_relative() {
        let config = DataConfig {
            path: Some(PathBuf::from(".ops-data/project.db")),
            ..DataConfig::default()
        };
        let root = Path::new("/home/proj");
        let path = Sqlite::resolve_path(&config, root);
        assert_eq!(path, PathBuf::from("/home/proj/.ops-data/project.db"));
    }

    #[test]
    fn resolve_path_absolute() {
        let config = DataConfig {
            path: Some(PathBuf::from("/absolute/shared.db")),
            ..DataConfig::default()
        };
        let root = Path::new("/home/proj");
        let path = Sqlite::resolve_path(&config, root);
        assert_eq!(path, PathBuf::from("/absolute/shared.db"));
    }

    /// TQ-004: Test `Sqlite` error path handling.
    mod error_path_tests {
        use super::*;

        #[test]
        fn sqlite_open_in_memory_succeeds() {
            let result = Sqlite::open_in_memory();
            assert!(result.is_ok(), "in-memory DB should always succeed");
        }

        #[test]
        fn sqlite_open_creates_parent_directory() {
            let dir = tempfile::tempdir().expect("tempdir");
            let db_path = dir.path().join("subdir/nested/db.sqlite");
            let result = Sqlite::open(&db_path);

            assert!(result.is_ok(), "should create parent directories");
            assert!(db_path.exists(), "db file should exist");
        }

        #[test]
        fn sqlite_lock_returns_guard() {
            let db = Sqlite::open_in_memory().expect("open");
            assert!(db.lock().is_ok(), "lock should succeed");
        }

        #[test]
        fn sqlite_path_returns_stored_path() {
            let db = Sqlite::open_in_memory().expect("open");
            assert_eq!(db.path(), Path::new(":memory:"));
        }

        #[test]
        fn sqlite_open_readonly_nonexistent_fails() {
            let result = Sqlite::open_readonly(Path::new("/nonexistent/path/db.sqlite"));
            assert!(result.is_err(), "readonly open of nonexistent should fail");
        }

        /// READ-5 (TASK-0525): pin the asymmetry — `open_readonly` does not
        /// create parent directories, and the resulting error is a rusqlite
        /// error rather than the parent-mkdir IO error `open` produces.
        #[test]
        fn sqlite_open_readonly_does_not_create_parent_dir() {
            let dir = tempfile::tempdir().expect("tempdir");
            let missing_parent = dir.path().join("does/not/exist");
            let db_path = missing_parent.join("db.sqlite");
            let result = Sqlite::open_readonly(&db_path);
            assert!(result.is_err(), "readonly open of nonexistent should fail");
            assert!(
                !missing_parent.exists(),
                "open_readonly must not mkdir parent: {missing_parent:?}"
            );
        }
    }
}
