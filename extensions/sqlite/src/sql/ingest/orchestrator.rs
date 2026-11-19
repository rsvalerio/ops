//! `provide_via_ingestor` orchestrator: per-table mutex, refresh, poison propagation.

use crate::sql::validation::quoted_ident;
use crate::Sqlite;

use super::dir::{data_dir_for_db, IngestDir};
use super::sql::source_is_loaded;

// Thread-local set of `&'static str` table names that the current thread
// already holds the ingest mutex for.
#[cfg(debug_assertions)]
thread_local! {
    static HELD_INGEST_TABLES: std::cell::RefCell<std::collections::HashSet<&'static str>> =
        std::cell::RefCell::new(std::collections::HashSet::new());
}

// RAII guard that records the current thread's ownership of the per-table
// ingest lock and detects re-entry on construction. Release builds compile
// to a zero-sized stub.
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
                    "provide_via_ingestor re-entered on the same thread for table `{table}`; std::sync::Mutex is non-reentrant and would deadlock in release builds"
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

/// Refresh an ingestor (collect + load) and return query results.
///
/// Orchestrates the full pipeline: check whether the source is loaded, if
/// not collect and load, then query.
///
/// "Loaded" means `table_name` exists **and** `data_sources` holds a
/// provenance row for `ingestor.name()` (see
/// [`source_is_loaded`](super::sql::source_is_loaded)). The row count is not
/// the signal: a source that legitimately loaded zero rows stays loaded and
/// is not re-collected on every call, and a table left without its
/// provenance row by an incomplete load is collected and loaded again.
///
/// A per-table ingest lock prevents duplicate collect+load cycles AND
/// extends across the trailing `query_fn` so a concurrent refresh cannot
/// DROP the table mid-query.
///
/// # Non-reentrancy contract
///
/// `std::sync::Mutex` is non-reentrant. A `query_fn` that recursively calls
/// `provide_via_ingestor` for the **same** `table_name` on the **same thread**
/// deadlocks silently. In `debug_assertions` builds we track the per-thread
/// set of currently held tables and `debug_assert!` that the table is not
/// already held; release builds do not pay the bookkeeping cost.
///
/// # Errors
///
/// If the ingestor cannot collect or load its data, or if `query_fn` fails.
pub fn provide_via_ingestor<I, Q>(
    db: &Sqlite,
    ctx: &ops_extension::Context,
    table_name: &'static str,
    ingestor: &I,
    query_fn: Q,
) -> Result<serde_json::Value, anyhow::Error>
where
    I: crate::DataIngestor,
    Q: FnOnce(&Sqlite) -> Result<serde_json::Value, anyhow::Error>,
{
    // Detect same-thread re-entry on the same table before acquiring the
    // lock.
    let _reentry_guard = ReentryGuard::new(table_name);

    use anyhow::Context;

    // A poisoned registry or per-table mutex means an earlier ingest
    // panicked inside the drop → probe → collect → load → query section, so
    // the table may be dropped or half-loaded and the staging dir
    // half-cleaned. Surface it as `DbError::MutexPoisoned`, the same stance
    // `Sqlite::lock` takes for the connection, instead of carrying on.
    let ingest_mutex = db
        .ingest_mutex_for(table_name)
        .with_context(|| format!("provide_via_ingestor({table_name}): ingest lock registry"))?;
    let _ingest_guard = ingest_mutex
        .lock()
        .map_err(|e| crate::DbError::MutexPoisoned(e.to_string()))
        .with_context(|| format!("provide_via_ingestor({table_name}): per-table ingest lock"))?;

    // Every `?` inside the orchestrator carries `.with_context` that names
    // *both* the phase (drop / probe / create-dir / collect / init-schema /
    // load / query) and the `table_name`. Without the label, two ingestors
    // failing at the same syscall produce identical raw `DbError::Io` /
    // `anyhow::Error` messages and operators have to grep the codebase to
    // localise the failure. The closure cost is one short allocation per
    // `?`; benign on the cold ingest path.

    // drop_table_if_exists MUST run inside the per-table ingest_mutex
    // critical section, before the load probe.
    if ctx.is_refreshing() {
        drop_table_if_exists(db, table_name)
            .with_context(|| format!("provide_via_ingestor({table_name}): drop phase"))?;
    }

    let loaded = source_is_loaded(db, table_name, ingestor.name())
        .with_context(|| format!("provide_via_ingestor({table_name}): load probe"))?;
    if !loaded {
        // An in-memory handle has no staging area; deriving the relative
        // `:memory:.ingest` from the sentinel would litter the process
        // working directory.
        let data_dir = data_dir_for_db(db.path())
            .with_context(|| format!("provide_via_ingestor({table_name}): ingest data dir"))?;
        // Create, harden and *verify* the staging directory once, then keep
        // the descriptor open for the whole collect→load cycle. Every
        // staged write below resolves against this handle, so the directory
        // the pipeline writes into is provably the one that was verified —
        // verifying and then dropping the handle would hand the bare path
        // on to be resolved again at every write.
        let dir = IngestDir::open(&data_dir)
            .with_context(|| format!("provide_via_ingestor({table_name}): create ingest dir"))?;
        if let Err(err) = ingestor.collect(ctx, &dir) {
            let label = format!("provide_via_ingestor({table_name}): ingestor collect");
            return Err(normalise_pipeline_errors(err, label));
        }
        crate::init_schema(db)
            .with_context(|| format!("provide_via_ingestor({table_name}): init_schema"))?;
        let _load_result = ingestor
            .load(&dir, db)
            .with_context(|| format!("provide_via_ingestor({table_name}): ingestor load"))?;
    }

    query_fn(db).with_context(|| format!("provide_via_ingestor({table_name}): query_fn"))
}

/// Re-wrap a failed pipeline call so a typed error payload survives
/// anyhow's context wrapping.
///
/// `.with_context` on a `DbError` erases any typed `DataProviderError`
/// travelling inside `DbError::External`: anyhow's context wrapper around a
/// *foreign* error type downcasts by matching only that type — it cannot
/// recurse into the `External` payload — so a spent deadline in an
/// ingestor's walk would reach the operator as
/// `DataProviderError::ComputationFailed` once
/// `From<anyhow::Error> for DataProviderError` fails its downcast.
/// Re-raising a typed payload through an *anyhow-internal* context instead
/// gives a chain that downcasts recursively, so the variant survives for
/// every caller that matches on it, and the phase label survives with it.
/// Every sidecar ingestor's `collect` funnels through
/// [`provide_via_ingestor`], so this covers them all, not just tokei.
fn normalise_pipeline_errors(err: crate::DbError, label: String) -> anyhow::Error {
    match err {
        crate::DbError::External(payload) => {
            match payload.downcast::<ops_extension::DataProviderError>() {
                // `Box::downcast` hands back the *unboxed* `DataProviderError`,
                // so `Error::new` wraps the concrete type and later
                // `downcast_ref::<DataProviderError>()` calls hit.
                Ok(typed) => anyhow::Error::new(typed).context(label),
                Err(payload) => {
                    anyhow::Error::new(crate::DbError::External(payload)).context(label)
                }
            }
        }
        other => anyhow::Error::new(other).context(label),
    }
}

/// Drop a table if it exists (used by refresh to force re-collection).
pub(super) fn drop_table_if_exists(db: &Sqlite, table_name: &str) -> Result<(), anyhow::Error> {
    use anyhow::Context;
    let quoted = quoted_ident(table_name)?;
    let conn = db.lock().context("acquiring db lock for drop")?;
    conn.execute_batch(&format!("DROP TABLE IF EXISTS {quoted}"))
        .with_context(|| format!("dropping table {table_name}"))?;
    drop(conn);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql::{execute_json_load, in_transaction, JsonColumn, JsonTableLoad};
    use crate::DbError;
    use crate::{init_schema, DbResult};
    use std::path::PathBuf;
    use std::sync::Arc;

    /// Shared one-column spec for the mock ingestors' loads.
    /// `flat_array` takes `&'static [JsonColumn]`; an inline `&[…]` literal
    /// is not rvalue-promotable (const-fn calls never are), so the slice is
    /// a named `const`.
    const ID_COLS: &[JsonColumn] = &[JsonColumn::integer("id", "$.id")];

    /// Loads the staged `file` into `table` and records the `data_sources`
    /// provenance row for `source` in one transaction, the way the real
    /// loaders publish, so `provide_via_ingestor` sees the source as loaded.
    fn mock_load(
        dir: &IngestDir,
        db: &Sqlite,
        table: &'static str,
        file: &'static str,
        source: &'static str,
    ) -> DbResult<crate::LoadResult> {
        let load = JsonTableLoad::flat_array(table, ID_COLS);
        let conn = db.lock()?;
        let rows = in_transaction(&conn, source, |conn| {
            execute_json_load(conn, dir, &load, file)?;
            let rows: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .map_err(|e| DbError::query_failed("mock count", e))?;
            let rows = u64::try_from(rows).unwrap_or(0);
            crate::upsert_data_source_with(
                conn,
                &crate::DataSourceMetadata::new(
                    crate::SourceName::new(source),
                    crate::WorkspaceRoot::new(std::ffi::OsStr::new("/ws")),
                    &dir.entry_path(file),
                    rows,
                    "checksum",
                ),
            )?;
            Ok(rows)
        })?;
        drop(conn);
        Ok(crate::LoadResult::success(source, rows))
    }

    // --- drop_table_if_exists validation ---

    #[test]
    fn drop_table_rejects_whitespace() {
        let db = Sqlite::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        assert!(drop_table_if_exists(&db, "my table").is_err());
    }

    #[test]
    fn drop_table_rejects_dots() {
        let db = Sqlite::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        assert!(drop_table_if_exists(&db, "schema.table").is_err());
    }

    #[test]
    fn drop_table_rejects_dashes() {
        let db = Sqlite::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        assert!(drop_table_if_exists(&db, "my-table").is_err());
    }

    #[test]
    fn drop_table_rejects_injection() {
        let db = Sqlite::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        assert!(drop_table_if_exists(&db, "t; DROP TABLE users; --").is_err());
    }

    /// Two threads invoking `provide_via_ingestor` against the same empty
    /// table must run collect at most once.
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
            fn collect(&self, _ctx: &ops_extension::Context, dir: &IngestDir) -> DbResult<()> {
                COLLECT_COUNT.fetch_add(1, Ordering::SeqCst);
                dir.write_atomic("counting.json", b"[{\"id\": 1}]")?;
                Ok(())
            }
            fn load(&self, dir: &IngestDir, db: &Sqlite) -> DbResult<crate::LoadResult> {
                mock_load(dir, db, "counting_test", "counting.json", "counting")
            }
        }

        let db_dir = tempfile::tempdir().expect("tempdir");
        let db_path = db_dir.path().join("counting.db");
        let db = Arc::new(Sqlite::open(&db_path).expect("db"));
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

    /// The per-table ingest registry is scoped to the `Sqlite` instance
    /// and bounded by the table count.
    #[test]
    fn ingest_lock_map_is_scoped_to_sqlite_instance_and_bounded_by_table_count() {
        use crate::DataIngestor;

        struct TrivialIngestor;
        impl DataIngestor for TrivialIngestor {
            fn name(&self) -> &'static str {
                "trivial"
            }
            fn collect(&self, _ctx: &ops_extension::Context, dir: &IngestDir) -> DbResult<()> {
                dir.write_atomic("trivial.json", b"[{\"id\":1}]")?;
                Ok(())
            }
            fn load(&self, dir: &IngestDir, db: &Sqlite) -> DbResult<crate::LoadResult> {
                mock_load(dir, db, "trivial_table", "trivial.json", "trivial")
            }
        }

        let db_dir = tempfile::tempdir().expect("tempdir");
        let db_path = db_dir.path().join("bounded.db");
        let db = Sqlite::open(&db_path).expect("db");
        init_schema(&db).expect("init_schema");

        let ctx = ops_extension::Context::new(
            Arc::new(ops_core::config::Config::empty()),
            PathBuf::from("/tmp"),
        );

        for name in ["t_a", "t_b", "t_c"] {
            let _ = db.ingest_mutex_for(name).expect("healthy registry");
            let _ = db.ingest_mutex_for(name).expect("healthy registry");
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
        let db2 = Sqlite::open(&db_path).expect("db reopen");
        assert_eq!(db2.ingest_lock_count(), 0, "fresh instance has no entries");
    }

    /// A panic inside an ingestor's `collect` poisons the per-table
    /// mutex mid-ingest. The next ingest of that table must surface that as
    /// a `DbError::MutexPoisoned` error naming the table, every time, rather
    /// than clearing the poison and carrying on over a possibly half-loaded
    /// table.
    #[test]
    fn panic_in_collect_surfaces_as_poisoned_ingest_lock_error() {
        use crate::DataIngestor;

        struct PanickyIngestor;
        impl DataIngestor for PanickyIngestor {
            fn name(&self) -> &'static str {
                "panicky"
            }
            // The panic *is* the behaviour under test: this mock stands in for
            // an ingestor that blows up mid-collect.
            #[allow(clippy::panic_in_result_fn)]
            fn collect(&self, _ctx: &ops_extension::Context, _dir: &IngestDir) -> DbResult<()> {
                panic!("simulated ingest panic");
            }
            fn load(&self, _dir: &IngestDir, _db: &Sqlite) -> DbResult<crate::LoadResult> {
                Ok(crate::LoadResult::success("panicky", 0))
            }
        }

        let db_dir = tempfile::tempdir().expect("tempdir");
        let db_path = db_dir.path().join("panicky.db");
        let db = Arc::new(Sqlite::open(&db_path).expect("db"));
        init_schema(&db).expect("init_schema");
        let ctx = || {
            ops_extension::Context::new(
                Arc::new(ops_core::config::Config::empty()),
                PathBuf::from("/tmp"),
            )
        };

        let db1 = Arc::clone(&db);
        let h = std::thread::spawn(move || {
            provide_via_ingestor(&db1, &ctx(), "panicky_table", &PanickyIngestor, |_| {
                Ok(serde_json::Value::Null)
            })
        });
        assert!(h.join().is_err(), "first call must have panicked");

        for attempt in 0..2 {
            let err = provide_via_ingestor(&db, &ctx(), "panicky_table", &PanickyIngestor, |_| {
                Ok(serde_json::Value::Null)
            })
            .expect_err("a poisoned ingest lock must surface as an error, not recover");
            assert!(
                matches!(err.downcast_ref::<DbError>(), Some(DbError::MutexPoisoned(_))),
                "attempt {attempt}: expected DbError::MutexPoisoned, got: {err:#}"
            );
            assert!(
                format!("{err:#}").contains("panicky_table"),
                "attempt {attempt}: the error must name the table, got: {err:#}"
            );
        }
    }

    /// Refresh-driven DROP serializes behind in-flight `query_fn`.
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
            fn collect(&self, _ctx: &ops_extension::Context, dir: &IngestDir) -> DbResult<()> {
                dir.write_atomic("race.json", b"[{\"id\":1}]")?;
                Ok(())
            }
            fn load(&self, dir: &IngestDir, db: &Sqlite) -> DbResult<crate::LoadResult> {
                mock_load(dir, db, "race_table", "race.json", "race")
            }
        }

        let db_dir = tempfile::tempdir().expect("tempdir");
        let db_path = db_dir.path().join("race.db");
        let db = Arc::new(Sqlite::open(&db_path).expect("db"));
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
                drop(conn);
                done1.store(true, Ordering::SeqCst);
                Ok(serde_json::json!({ "count": count }))
            })
        });

        let db2 = Arc::clone(&db);
        let bar2 = Arc::clone(&inside_query);
        let done2 = Arc::clone(&query_done);
        let h2 = std::thread::spawn(move || {
            bar2.wait();
            let ctx = ops_extension::Context::new(
                Arc::new(ops_core::config::Config::empty()),
                PathBuf::from("/tmp"),
            )
            .with_refresh();
            let result = provide_via_ingestor(&db2, &ctx, "race_table", &TrivialIngestor, |_| {
                Ok(serde_json::Value::Null)
            });
            (done2.load(Ordering::SeqCst), result)
        });

        let r1 = h1.join().expect("join 1").expect("query 1 succeeded");
        let (q1_was_done, r2) = h2.join().expect("join 2");
        r2.expect("ingest 2 succeeded");

        assert_eq!(r1["count"], 1, "thread 1 saw the row mid-query");
        assert!(q1_was_done, "thread 2 must have been serialized behind thread 1's query_fn");
    }

    /// Same-thread re-entry on the same table panics.
    #[cfg(debug_assertions)]
    #[test]
    fn reentry_guard_panics_on_same_thread_same_table_reentry() {
        let _outer = ReentryGuard::new("conc2_reentry_test");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _inner = ReentryGuard::new("conc2_reentry_test");
        }));
        assert!(result.is_err(), "ReentryGuard must panic on same-thread re-entry");
    }

    /// Distinct tables on the same thread are fine, and a released table
    /// can be re-acquired on the same thread.
    #[cfg(debug_assertions)]
    #[test]
    fn reentry_guard_allows_distinct_tables_on_same_thread() {
        let a = ReentryGuard::new("conc2_table_a");
        let _b = ReentryGuard::new("conc2_table_b");
        // Release `conc2_table_a`, then re-acquire the same table on this
        // thread: the held-set tracks live guards, so a dropped guard must
        // not trip the same-thread re-entry assert.
        drop(a);
        let reacquired = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _a_again = ReentryGuard::new("conc2_table_a");
        }));
        assert!(reacquired.is_ok(), "release-then-reacquire of the same table must be allowed");
    }

    /// `normalise_pipeline_errors` must preserve the concrete
    /// `DataProviderError` type: `downcast_ref::<DataProviderError>()` on the
    /// re-raised error has to hit, which requires unboxing before
    /// `anyhow::Error::new` — a `Box<T>` payload only downcasts to `Box<T>`,
    /// so every caller matching on the variant would miss.
    #[test]
    fn normalise_pipeline_errors_preserves_the_typed_payload() {
        let err = normalise_pipeline_errors(
            DbError::External(anyhow::Error::new(
                ops_extension::DataProviderError::ComputationMessage("spent deadline".into()),
            )),
            "provide_via_ingestor(t): ingestor collect".to_string(),
        );
        assert!(
            err.downcast_ref::<ops_extension::DataProviderError>().is_some(),
            "the unboxed concrete type must survive for downcast_ref"
        );
        let rendered = format!("{err:#}");
        assert!(rendered.contains("ingestor collect"), "phase label must survive: {rendered}");
    }

    /// A failing `collect` must surface the table name
    /// and the phase label in the anyhow error chain so operators can
    /// localise the failure without grepping the codebase.
    #[test]
    fn failing_collect_surfaces_table_name_in_error_chain() {
        use crate::DataIngestor;

        struct FailingIngestor;
        impl DataIngestor for FailingIngestor {
            fn name(&self) -> &'static str {
                "failing"
            }
            fn collect(&self, _ctx: &ops_extension::Context, _dir: &IngestDir) -> DbResult<()> {
                Err(DbError::Io(std::io::Error::other("simulated collect failure")))
            }
            // The panic *is* the assertion: this double exists to prove `load` is
            // never reached once `collect` fails. Returning an `Err` instead would
            // let a regression that calls `load` anyway flow into the error-chain
            // assertions below and pass. `allow-panic-in-tests` does not cover
            // `panic_in_result_fn`, so the exception is spelled here.
            #[allow(clippy::panic_in_result_fn)]
            fn load(&self, _dir: &IngestDir, _db: &Sqlite) -> DbResult<crate::LoadResult> {
                panic!("load must not run: collect failed first")
            }
        }

        let db_dir = tempfile::tempdir().expect("tempdir");
        let db_path = db_dir.path().join("ctx.db");
        let db = Sqlite::open(&db_path).expect("db");
        init_schema(&db).expect("init_schema");
        let ctx = ops_extension::Context::new(
            Arc::new(ops_core::config::Config::empty()),
            PathBuf::from("/tmp"),
        );

        let err = provide_via_ingestor(&db, &ctx, "ctx_err_table", &FailingIngestor, |_| {
            Ok(serde_json::Value::Null)
        })
        .expect_err("collect failure must propagate");

        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("ctx_err_table"),
            "table name must appear in error chain: {rendered}"
        );
        assert!(
            rendered.contains("ingestor collect"),
            "phase label must appear in error chain: {rendered}"
        );
    }

    /// Driving the pipeline with an in-memory handle must fail loudly
    /// instead of creating a relative `:memory:.ingest` directory in the
    /// process working directory — an artifact left behind there can even
    /// get committed to the repository by accident.
    #[test]
    fn provide_via_ingestor_rejects_in_memory_db_without_touching_the_cwd() {
        use crate::DataIngestor;

        struct NeverCollects;
        impl DataIngestor for NeverCollects {
            fn name(&self) -> &'static str {
                "never"
            }
            // Reaching either method means the sentinel guard did not fire.
            #[allow(clippy::panic_in_result_fn)]
            fn collect(&self, _ctx: &ops_extension::Context, _dir: &IngestDir) -> DbResult<()> {
                panic!("collect must not run for an in-memory database")
            }
            #[allow(clippy::panic_in_result_fn)]
            fn load(&self, _dir: &IngestDir, _db: &Sqlite) -> DbResult<crate::LoadResult> {
                panic!("load must not run for an in-memory database")
            }
        }

        let db = Sqlite::open_in_memory().expect("in-memory db");
        init_schema(&db).expect("init_schema");
        let ctx = ops_extension::Context::new(
            Arc::new(ops_core::config::Config::empty()),
            PathBuf::from("/tmp"),
        );

        let err = provide_via_ingestor(&db, &ctx, "memory_table", &NeverCollects, |_| {
            Ok(serde_json::Value::Null)
        })
        .expect_err("an in-memory database has no ingest staging area");

        let rendered = format!("{err:#}");
        assert!(rendered.contains("not file-backed"), "error must name the real cause: {rendered}");

        // Nothing named after the sentinel may appear in the process CWD.
        let cwd = std::env::current_dir().expect("cwd");
        assert!(
            !cwd.join(":memory:.ingest").exists(),
            "the pipeline created {}/:memory:.ingest",
            cwd.display()
        );
    }

    fn file_backed_ctx() -> ops_extension::Context {
        ops_extension::Context::new(
            Arc::new(ops_core::config::Config::empty()),
            PathBuf::from("/tmp"),
        )
    }

    /// A source that loaded zero rows is still loaded: the provenance row,
    /// not the row count, is the marker, so a second call does not collect
    /// again.
    #[test]
    fn empty_dataset_is_collected_once_across_calls() {
        use crate::DataIngestor;
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct EmptyIngestor {
            collects: AtomicUsize,
        }
        impl DataIngestor for EmptyIngestor {
            fn name(&self) -> &'static str {
                "empty"
            }
            fn collect(&self, _ctx: &ops_extension::Context, dir: &IngestDir) -> DbResult<()> {
                self.collects.fetch_add(1, Ordering::SeqCst);
                dir.write_atomic("empty.json", b"[]")
            }
            fn load(&self, dir: &IngestDir, db: &Sqlite) -> DbResult<crate::LoadResult> {
                mock_load(dir, db, "empty_table", "empty.json", "empty")
            }
        }

        let db_dir = tempfile::tempdir().expect("tempdir");
        let db = Sqlite::open(&db_dir.path().join("empty.db")).expect("db");
        let ingestor = EmptyIngestor { collects: AtomicUsize::new(0) };
        for _ in 0..2 {
            provide_via_ingestor(&db, &file_backed_ctx(), "empty_table", &ingestor, |_| {
                Ok(serde_json::Value::Null)
            })
            .expect("ingest");
        }
        assert_eq!(ingestor.collects.load(Ordering::SeqCst), 1, "an empty load must stick");
    }

    /// A sidecar load whose view batch fails after the table insert rolls
    /// back completely, so the next call collects and loads again instead of
    /// skipping over a half-built table.
    #[test]
    fn failed_sidecar_load_is_retried_by_the_next_call() {
        use crate::sql::CreateViewSql;
        use crate::{DataIngestor, SidecarIngestorConfig};
        use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

        const CONFIG: SidecarIngestorConfig =
            SidecarIngestorConfig::new("flaky", "flaky.json", "flaky_table");

        struct FlakyViewIngestor {
            collects: AtomicUsize,
            fail_view: AtomicBool,
        }
        impl DataIngestor for FlakyViewIngestor {
            fn name(&self) -> &'static str {
                CONFIG.name
            }
            fn collect(&self, _ctx: &ops_extension::Context, dir: &IngestDir) -> DbResult<()> {
                self.collects.fetch_add(1, Ordering::SeqCst);
                CONFIG.collect_sidecar(
                    dir,
                    &serde_json::json!([{ "id": 1 }, { "id": 2 }]),
                    std::path::Path::new("/ws"),
                )
            }
            fn load(&self, dir: &IngestDir, db: &Sqlite) -> DbResult<crate::LoadResult> {
                let view = if self.fail_view.swap(false, Ordering::SeqCst) {
                    CreateViewSql::from_literal_for_tests("CREATE VIEW flaky_v AS SELEC 1")
                } else {
                    CreateViewSql::from_literal_for_tests(
                        "DROP VIEW IF EXISTS flaky_v; \
                         CREATE VIEW flaky_v AS SELECT * FROM flaky_table",
                    )
                };
                let load = JsonTableLoad::flat_array("flaky_table", ID_COLS);
                CONFIG.load_with_sidecar(db, dir, &load, &view)
            }
        }

        let db_dir = tempfile::tempdir().expect("tempdir");
        let db = Sqlite::open(&db_dir.path().join("flaky.db")).expect("db");
        let ingestor =
            FlakyViewIngestor { collects: AtomicUsize::new(0), fail_view: AtomicBool::new(true) };

        provide_via_ingestor(&db, &file_backed_ctx(), "flaky_table", &ingestor, |_| {
            Ok(serde_json::Value::Null)
        })
        .expect_err("the broken view batch must fail the load");
        let conn = db.lock().expect("lock");
        assert!(
            !super::super::sql::table_exists(&conn, "flaky_table").expect("probe"),
            "the failed load must leave no populated table behind"
        );
        drop(conn);

        let rows = provide_via_ingestor(&db, &file_backed_ctx(), "flaky_table", &ingestor, |db| {
            let conn = db.lock().expect("lock");
            let n: i64 =
                conn.query_row("SELECT COUNT(*) FROM flaky_v", [], |r| r.get(0)).expect("view");
            drop(conn);
            Ok(serde_json::json!(n))
        })
        .expect("the retry must load");
        assert_eq!(ingestor.collects.load(Ordering::SeqCst), 2, "the retry must collect again");
        assert_eq!(rows, serde_json::json!(2));
    }
}
