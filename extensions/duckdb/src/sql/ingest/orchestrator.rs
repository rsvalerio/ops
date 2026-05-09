//! `provide_via_ingestor` orchestrator: per-table mutex, refresh, poison recovery.

use crate::sql::validation::quoted_ident;
use crate::{DbError, DuckDb};

use super::dir::{create_ingest_dir, data_dir_for_db};
use super::sql::table_has_data;

// CONC-2 / TASK-1143: thread-local set of `&'static str` table names that
// the current thread already holds the ingest mutex for.
#[cfg(debug_assertions)]
thread_local! {
    static HELD_INGEST_TABLES: std::cell::RefCell<std::collections::HashSet<&'static str>> =
        std::cell::RefCell::new(std::collections::HashSet::new());
}

// CONC-2 / TASK-1143: RAII guard that records the current thread's
// ownership of the per-table ingest lock and detects re-entry on
// construction. Release builds compile to a zero-sized stub.
pub(super) struct ReentryGuard {
    #[cfg(debug_assertions)]
    table: &'static str,
}

impl ReentryGuard {
    pub(super) fn new(table: &'static str) -> Self {
        #[cfg(debug_assertions)]
        {
            HELD_INGEST_TABLES.with(|set| {
                let inserted = set.borrow_mut().insert(table);
                debug_assert!(
                    inserted,
                    "CONC-2 / TASK-1143: provide_via_ingestor re-entered on the same thread for table `{table}`; std::sync::Mutex is non-reentrant and would deadlock in release builds"
                );
            });
            Self { table }
        }
        #[cfg(not(debug_assertions))]
        {
            let _ = table;
            Self {}
        }
    }
}

#[cfg(debug_assertions)]
impl Drop for ReentryGuard {
    fn drop(&mut self) {
        HELD_INGEST_TABLES.with(|set| {
            set.borrow_mut().remove(self.table);
        });
    }
}

/// DUP-028/029/030: Refresh an ingestor (collect + load) and return query results.
///
/// Orchestrates the full pipeline: check if table has data, if not collect and load,
/// then query.
///
/// CONC-2 (TASK-0728/1073): a per-table ingest lock prevents duplicate
/// collect+load cycles AND extends across the trailing `query_fn` so a
/// concurrent refresh cannot DROP the table mid-query.
///
/// # Non-reentrancy contract (CONC-2 / TASK-1143)
///
/// `std::sync::Mutex` is non-reentrant. A `query_fn` that recursively calls
/// `provide_via_ingestor` for the **same** `table_name` on the **same thread**
/// deadlocks silently. In `debug_assertions` builds we track the per-thread
/// set of currently held tables and `debug_assert!` that the table is not
/// already held; release builds do not pay the bookkeeping cost.
pub fn provide_via_ingestor<I, Q>(
    db: &DuckDb,
    ctx: &ops_extension::Context,
    table_name: &'static str,
    ingestor: &I,
    query_fn: Q,
) -> Result<serde_json::Value, anyhow::Error>
where
    I: crate::DataIngestor,
    Q: FnOnce(&DuckDb) -> Result<serde_json::Value, anyhow::Error>,
{
    // CONC-2 / TASK-1143: detect same-thread re-entry on the same table
    // before acquiring the lock.
    let _reentry_guard = ReentryGuard::new(table_name);

    let ingest_mutex = db.ingest_mutex_for(table_name);
    // ERR-5 (TASK-0780): poisoning the per-table mutex must not become a
    // permanent denial of service for that table — recover via `into_inner`.
    let _ingest_guard = ingest_mutex.lock().unwrap_or_else(|poisoned| {
        tracing::warn!(
            table = %table_name,
            "per-table ingest mutex was poisoned by a prior panic; recovered"
        );
        poisoned.into_inner()
    });

    // CONC-2 / TASK-0909: drop_table_if_exists MUST run inside the per-table
    // ingest_mutex critical section, before the table_has_data probe.
    if ctx.refresh {
        drop_table_if_exists(db, table_name)?;
    }

    if !table_has_data(db, table_name)? {
        let data_dir = data_dir_for_db(db.path());
        create_ingest_dir(&data_dir).map_err(DbError::Io)?;
        ingestor.collect(ctx, &data_dir)?;
        crate::init_schema(db)?;
        let _load_result = ingestor.load(&data_dir, db)?;
    }

    query_fn(db)
}

/// Drop a table if it exists (used by refresh to force re-collection).
pub(super) fn drop_table_if_exists(db: &DuckDb, table_name: &str) -> Result<(), anyhow::Error> {
    use anyhow::Context;
    let quoted = quoted_ident(table_name)?;
    let conn = db.lock().context("acquiring db lock for drop")?;
    conn.execute_batch(&format!("DROP TABLE IF EXISTS {quoted}"))
        .with_context(|| format!("dropping table {table_name}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql::ingest::sql::create_table_from_json_sql;
    use crate::{init_schema, DbResult};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    // --- drop_table_if_exists validation (SEC-12) ---

    #[test]
    fn drop_table_rejects_whitespace() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        assert!(drop_table_if_exists(&db, "my table").is_err());
    }

    #[test]
    fn drop_table_rejects_dots() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        assert!(drop_table_if_exists(&db, "schema.table").is_err());
    }

    #[test]
    fn drop_table_rejects_dashes() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        assert!(drop_table_if_exists(&db, "my-table").is_err());
    }

    #[test]
    fn drop_table_rejects_injection() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        assert!(drop_table_if_exists(&db, "t; DROP TABLE users; --").is_err());
    }

    /// CONC-2 (TASK-0728): two threads invoking `provide_via_ingestor` against
    /// the same empty table must run collect at most once.
    #[test]
    fn concurrent_provide_via_ingestor_collects_once() {
        use crate::DataIngestor;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static COLLECT_COUNT: AtomicUsize = AtomicUsize::new(0);

        struct CountingIngestor;

        impl DataIngestor for CountingIngestor {
            fn name(&self) -> &'static str {
                "counting"
            }
            fn collect(&self, _ctx: &ops_extension::Context, data_dir: &Path) -> DbResult<()> {
                COLLECT_COUNT.fetch_add(1, Ordering::SeqCst);
                let path = data_dir.join("counting.json");
                std::fs::write(&path, "[{\"id\": 1}]").map_err(DbError::Io)?;
                Ok(())
            }
            fn load(&self, data_dir: &Path, db: &DuckDb) -> DbResult<crate::LoadResult> {
                let json_path = data_dir.join("counting.json");
                let create_sql = create_table_from_json_sql("counting_test", &json_path, None)?;
                let conn = db.lock()?;
                conn.execute(&create_sql, [])
                    .map_err(|e| DbError::query_failed("counting_test create", e))?;
                drop(conn);
                Ok(crate::LoadResult::success("counting", 1))
            }
        }

        let db_dir = tempfile::tempdir().expect("tempdir");
        let db_path = db_dir.path().join("counting.duckdb");
        let db = Arc::new(DuckDb::open(&db_path).expect("db"));
        init_schema(&db).expect("init_schema");

        let db1 = Arc::clone(&db);
        let db2 = Arc::clone(&db);
        let ctx1 = ops_extension::Context::new(
            Arc::new(ops_core::config::Config::empty()),
            PathBuf::from("/tmp"),
        );
        let ctx2 = ops_extension::Context::new(
            Arc::new(ops_core::config::Config::empty()),
            PathBuf::from("/tmp"),
        );

        let h1 = std::thread::spawn(move || {
            provide_via_ingestor(&db1, &ctx1, "counting_test", &CountingIngestor, |_| {
                Ok(serde_json::Value::Null)
            })
        });
        let h2 = std::thread::spawn(move || {
            provide_via_ingestor(&db2, &ctx2, "counting_test", &CountingIngestor, |_| {
                Ok(serde_json::Value::Null)
            })
        });

        h1.join().expect("join 1").expect("ingest 1");
        h2.join().expect("join 2").expect("ingest 2");

        assert_eq!(
            COLLECT_COUNT.load(Ordering::SeqCst),
            1,
            "collect must run exactly once, not twice"
        );
    }

    /// CONC-7 (TASK-0779): per-table ingest registry is scoped to the DuckDb
    /// instance and bounded by the table count.
    #[test]
    fn ingest_lock_map_is_scoped_to_duckdb_instance_and_bounded_by_table_count() {
        use crate::DataIngestor;

        struct TrivialIngestor;
        impl DataIngestor for TrivialIngestor {
            fn name(&self) -> &'static str {
                "trivial"
            }
            fn collect(&self, _ctx: &ops_extension::Context, data_dir: &Path) -> DbResult<()> {
                let path = data_dir.join("trivial.json");
                std::fs::write(&path, "[{\"id\":1}]").map_err(DbError::Io)?;
                Ok(())
            }
            fn load(&self, data_dir: &Path, db: &DuckDb) -> DbResult<crate::LoadResult> {
                let json_path = data_dir.join("trivial.json");
                let create_sql = create_table_from_json_sql("trivial_table", &json_path, None)?;
                let conn = db.lock()?;
                conn.execute(&create_sql, [])
                    .map_err(|e| DbError::query_failed("trivial create", e))?;
                drop(conn);
                Ok(crate::LoadResult::success("trivial", 1))
            }
        }

        let db_dir = tempfile::tempdir().expect("tempdir");
        let db_path = db_dir.path().join("bounded.duckdb");
        let db = DuckDb::open(&db_path).expect("db");
        init_schema(&db).expect("init_schema");

        let ctx = ops_extension::Context::new(
            Arc::new(ops_core::config::Config::empty()),
            PathBuf::from("/tmp"),
        );

        for name in ["t_a", "t_b", "t_c"] {
            let _ = db.ingest_mutex_for(name);
            let _ = db.ingest_mutex_for(name);
        }
        assert_eq!(
            db.ingest_lock_count(),
            3,
            "registry should hold one entry per distinct table name"
        );

        let before = db.ingest_lock_count();
        provide_via_ingestor(&db, &ctx, "trivial_table", &TrivialIngestor, |_| {
            Ok(serde_json::Value::Null)
        })
        .expect("ingest");
        assert_eq!(
            db.ingest_lock_count(),
            before + 1,
            "one new entry for the freshly ingested table"
        );

        drop(db);
        let db2 = DuckDb::open(&db_path).expect("db reopen");
        assert_eq!(db2.ingest_lock_count(), 0, "fresh instance has no entries");
    }

    /// ERR-5 (TASK-0780): a panic inside an ingestor's `collect` must not
    /// permanently brick the table.
    #[test]
    fn panic_in_collect_does_not_brick_subsequent_ingest() {
        use crate::DataIngestor;
        use std::sync::atomic::{AtomicBool, Ordering};

        struct PanickyIngestor {
            should_panic: AtomicBool,
        }
        impl DataIngestor for PanickyIngestor {
            fn name(&self) -> &'static str {
                "panicky"
            }
            fn collect(&self, _ctx: &ops_extension::Context, data_dir: &Path) -> DbResult<()> {
                if self.should_panic.swap(false, Ordering::SeqCst) {
                    panic!("simulated transient ingest panic");
                }
                let path = data_dir.join("panicky.json");
                std::fs::write(&path, "[{\"id\":1}]").map_err(DbError::Io)?;
                Ok(())
            }
            fn load(&self, data_dir: &Path, db: &DuckDb) -> DbResult<crate::LoadResult> {
                let json_path = data_dir.join("panicky.json");
                let create_sql = create_table_from_json_sql("panicky_table", &json_path, None)?;
                let conn = db.lock()?;
                conn.execute(&create_sql, [])
                    .map_err(|e| DbError::query_failed("panicky create", e))?;
                drop(conn);
                Ok(crate::LoadResult::success("panicky", 1))
            }
        }

        let db_dir = tempfile::tempdir().expect("tempdir");
        let db_path = db_dir.path().join("panicky.duckdb");
        let db = Arc::new(DuckDb::open(&db_path).expect("db"));
        init_schema(&db).expect("init_schema");
        let ingestor = Arc::new(PanickyIngestor {
            should_panic: AtomicBool::new(true),
        });

        let db1 = Arc::clone(&db);
        let ing1 = Arc::clone(&ingestor);
        let h = std::thread::spawn(move || {
            let ctx = ops_extension::Context::new(
                Arc::new(ops_core::config::Config::empty()),
                PathBuf::from("/tmp"),
            );
            provide_via_ingestor(&db1, &ctx, "panicky_table", &*ing1, |_| {
                Ok(serde_json::Value::Null)
            })
        });
        assert!(h.join().is_err(), "first call must have panicked");

        let ctx = ops_extension::Context::new(
            Arc::new(ops_core::config::Config::empty()),
            PathBuf::from("/tmp"),
        );
        provide_via_ingestor(&db, &ctx, "panicky_table", &*ingestor, |_| {
            Ok(serde_json::Value::Null)
        })
        .expect("recovery ingest must not panic");
    }

    /// TASK-0861: poison recovery emits a warn log.
    #[test]
    fn poison_recovery_emits_warn_log() {
        use crate::DataIngestor;
        use std::io::Write;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Mutex as StdMutex;
        use tracing_subscriber::fmt::MakeWriter;

        #[derive(Clone, Default)]
        struct BufWriter(Arc<StdMutex<Vec<u8>>>);
        impl Write for BufWriter {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(b);
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl<'a> MakeWriter<'a> for BufWriter {
            type Writer = BufWriter;
            fn make_writer(&'a self) -> Self::Writer {
                self.clone()
            }
        }

        struct PanickyIngestor {
            should_panic: AtomicBool,
        }
        impl DataIngestor for PanickyIngestor {
            fn name(&self) -> &'static str {
                "panicky_warn"
            }
            fn collect(&self, _ctx: &ops_extension::Context, data_dir: &Path) -> DbResult<()> {
                if self.should_panic.swap(false, Ordering::SeqCst) {
                    panic!("simulated transient ingest panic");
                }
                let path = data_dir.join("panicky_warn.json");
                std::fs::write(&path, "[{\"id\":1}]").map_err(DbError::Io)?;
                Ok(())
            }
            fn load(&self, data_dir: &Path, db: &DuckDb) -> DbResult<crate::LoadResult> {
                let json_path = data_dir.join("panicky_warn.json");
                let create_sql =
                    create_table_from_json_sql("panicky_warn_table", &json_path, None)?;
                let conn = db.lock()?;
                conn.execute(&create_sql, [])
                    .map_err(|e| DbError::query_failed("panicky create", e))?;
                drop(conn);
                Ok(crate::LoadResult::success("panicky_warn", 1))
            }
        }

        let db_dir = tempfile::tempdir().expect("tempdir");
        let db_path = db_dir.path().join("panicky_warn.duckdb");
        let db = Arc::new(DuckDb::open(&db_path).expect("db"));
        init_schema(&db).expect("init_schema");
        let ingestor = Arc::new(PanickyIngestor {
            should_panic: AtomicBool::new(true),
        });

        let db1 = Arc::clone(&db);
        let ing1 = Arc::clone(&ingestor);
        let h = std::thread::spawn(move || {
            let ctx = ops_extension::Context::new(
                Arc::new(ops_core::config::Config::empty()),
                PathBuf::from("/tmp"),
            );
            provide_via_ingestor(&db1, &ctx, "panicky_warn_table", &*ing1, |_| {
                Ok(serde_json::Value::Null)
            })
        });
        assert!(h.join().is_err(), "first call must have panicked");

        let buf = BufWriter::default();
        let captured = buf.0.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf)
            .with_max_level(tracing::Level::WARN)
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            let ctx = ops_extension::Context::new(
                Arc::new(ops_core::config::Config::empty()),
                PathBuf::from("/tmp"),
            );
            provide_via_ingestor(&db, &ctx, "panicky_warn_table", &*ingestor, |_| {
                Ok(serde_json::Value::Null)
            })
            .expect("recovery ingest must not panic");
        });

        let logs = String::from_utf8(captured.lock().unwrap().clone()).unwrap();
        assert!(
            logs.contains("per-table ingest mutex was poisoned"),
            "expected poison-recovery warn, got: {logs}"
        );
        assert!(
            logs.contains("panicky_warn_table"),
            "expected table name in warn, got: {logs}"
        );
    }

    /// CONC-2 (TASK-1073): refresh-driven DROP serializes behind in-flight query_fn.
    #[test]
    fn refresh_during_query_fn_is_serialized_by_ingest_mutex() {
        use crate::DataIngestor;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Barrier;
        use std::time::Duration;

        struct TrivialIngestor;
        impl DataIngestor for TrivialIngestor {
            fn name(&self) -> &'static str {
                "race"
            }
            fn collect(&self, _ctx: &ops_extension::Context, data_dir: &Path) -> DbResult<()> {
                let path = data_dir.join("race.json");
                std::fs::write(&path, "[{\"id\":1}]").map_err(DbError::Io)?;
                Ok(())
            }
            fn load(&self, data_dir: &Path, db: &DuckDb) -> DbResult<crate::LoadResult> {
                let json_path = data_dir.join("race.json");
                let create_sql = create_table_from_json_sql("race_table", &json_path, None)?;
                let conn = db.lock()?;
                conn.execute(&create_sql, [])
                    .map_err(|e| DbError::query_failed("race create", e))?;
                drop(conn);
                Ok(crate::LoadResult::success("race", 1))
            }
        }

        let db_dir = tempfile::tempdir().expect("tempdir");
        let db_path = db_dir.path().join("race.duckdb");
        let db = Arc::new(DuckDb::open(&db_path).expect("db"));
        init_schema(&db).expect("init_schema");

        let prime_ctx = ops_extension::Context::new(
            Arc::new(ops_core::config::Config::empty()),
            PathBuf::from("/tmp"),
        );
        provide_via_ingestor(&db, &prime_ctx, "race_table", &TrivialIngestor, |_| {
            Ok(serde_json::Value::Null)
        })
        .expect("prime ingest");

        let inside_query = Arc::new(Barrier::new(2));
        let query_done = Arc::new(AtomicBool::new(false));
        let db1 = Arc::clone(&db);
        let bar1 = Arc::clone(&inside_query);
        let done1 = Arc::clone(&query_done);
        let h1 = std::thread::spawn(move || {
            let ctx = ops_extension::Context::new(
                Arc::new(ops_core::config::Config::empty()),
                PathBuf::from("/tmp"),
            );
            provide_via_ingestor(&db1, &ctx, "race_table", &TrivialIngestor, |db| {
                bar1.wait();
                std::thread::sleep(Duration::from_millis(150));
                let conn = db.lock().expect("lock");
                let count: i64 = conn
                    .query_row("SELECT COUNT(*) FROM race_table", [], |r| r.get(0))
                    .expect("table must still exist mid-query");
                done1.store(true, Ordering::SeqCst);
                Ok(serde_json::json!({ "count": count }))
            })
        });

        let db2 = Arc::clone(&db);
        let bar2 = Arc::clone(&inside_query);
        let done2 = Arc::clone(&query_done);
        let h2 = std::thread::spawn(move || {
            bar2.wait();
            let mut ctx = ops_extension::Context::new(
                Arc::new(ops_core::config::Config::empty()),
                PathBuf::from("/tmp"),
            );
            ctx.refresh = true;
            let result = provide_via_ingestor(&db2, &ctx, "race_table", &TrivialIngestor, |_| {
                Ok(serde_json::Value::Null)
            });
            (done2.load(Ordering::SeqCst), result)
        });

        let r1 = h1.join().expect("join 1").expect("query 1 succeeded");
        let (q1_was_done, r2) = h2.join().expect("join 2");
        r2.expect("ingest 2 succeeded");

        assert_eq!(r1["count"], 1, "thread 1 saw the row mid-query");
        assert!(
            q1_was_done,
            "thread 2 must have been serialized behind thread 1's query_fn",
        );
    }

    /// CONC-2 / TASK-1143: same-thread re-entry on the same table panics.
    #[cfg(debug_assertions)]
    #[test]
    fn reentry_guard_panics_on_same_thread_same_table_reentry() {
        let _outer = ReentryGuard::new("conc2_reentry_test");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _inner = ReentryGuard::new("conc2_reentry_test");
        }));
        assert!(
            result.is_err(),
            "ReentryGuard must panic on same-thread re-entry"
        );
    }

    /// CONC-2 / TASK-1143: distinct tables on the same thread are fine.
    #[cfg(debug_assertions)]
    #[test]
    fn reentry_guard_allows_distinct_tables_on_same_thread() {
        let _a = ReentryGuard::new("conc2_table_a");
        let _b = ReentryGuard::new("conc2_table_b");
    }
}
