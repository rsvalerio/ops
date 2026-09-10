//! Rust `project_coverage` data provider.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use ops_about::lru::BoundedLruCache;
use ops_core::project_identity::{CoverageStats, ProjectCoverage, UnitCoverage};
use ops_duckdb::sql::{query_crate_coverage, query_or_warn, query_project_coverage, CrateCoverage};
use ops_duckdb::DuckDb;
use ops_extension::{Context, DataProvider, DataProviderError};

use crate::manifest::{load_workspace_manifest, log_manifest_load_failure};
use crate::units::resolve_crate_display_name;

pub const PROVIDER_NAME: &str = "project_coverage";

/// DUP-1 (TASK-1079): per-process memoization for `query_project_coverage`.
///
/// `RustCoverageProvider::provide` and `identity::metrics::query_identity_metrics`
/// both run during a single `ops about` invocation and historically each
/// dispatched their own `query_project_coverage` call against the same
/// `DuckDB`. That doubled the scan and — more visibly — fired any
/// `query_or_warn` schema-drift log line twice.
///
/// ARCH-9 / TASK-1155: dedup with a tiny process-local cache keyed by the
/// `DuckDb` instance's stable `id()` (a monotonic u64 minted on
/// construction). Earlier this used `std::ptr::from_ref(db) as usize` as
/// the key, which was vulnerable to pointer-address ABA — a dropped-and-
/// replaced `DuckDb` could re-allocate at the same address and return a
/// previous instance's cached value. The id-keyed scheme guarantees two
/// distinct instances always receive distinct keys regardless of allocation
/// reuse. `Option<CrateCoverage>` mirrors the `query_or_warn` fallback
/// (None on query failure) so a hard failure is also memoized — the warn
/// fires exactly once per run regardless of how many providers consume the
/// value.
///
/// # PERF-16 / TASK-1764: cache contract
///
/// - **Key**: `DuckDb::id()`, a monotonic per-instance counter. A key is never
///   reused, so an entry outlives the `DuckDb` it describes.
/// - **Value**: `Arc<OnceLock<Option<CrateCoverage>>>` — the memoized project
///   total, or the memoized `None` fallback for a failed query.
/// - **Maximum size**: [`MAX_COVERAGE_CACHE_ENTRIES`], enforced on insert with
///   LRU eviction, mirroring the `manifest_cache` policy. Without a cap this
///   map grew one slot per `DuckDb` ever opened, forever: harmless in the
///   single-shot `ops about` CLI, an unbounded leak in the daemon / CI-worker
///   host shape that opens a handle per project or per refresh, and every
///   leaked entry describes an instance that is already gone.
/// - **Invalidation**: none within the life of a `DuckDb` handle. This is
///   deliberate and is the memoization's whole point (one query, one warn per
///   run), but it means coverage data re-ingested behind a *live* handle keeps
///   serving the pre-ingest number. A caller that re-ingests and needs the new
///   figure must open a fresh `DuckDb`, which mints a fresh key.
type CoverageSlot = Arc<OnceLock<Option<CrateCoverage>>>;

/// PERF-16 / TASK-1764: soft cap on the memoization map. A single `ops about`
/// run touches exactly one `DuckDb`; the headroom exists so a host that
/// interleaves a handful of projects still hits the cache while an unbounded
/// producer cannot grow the map without limit.
const MAX_COVERAGE_CACHE_ENTRIES: usize = 16;

/// DUP-1 / TASK-2150: the LRU scaffold (victim queue, compaction slack,
/// record/evict loop, cap preamble, tick-on-hit) lives in
/// [`ops_about::lru::BoundedLruCache`]; this struct names the key, value
/// and cap for the coverage memoization.
struct ProjectCoverageCache {
    cache: BoundedLruCache<u64, CoverageSlot>,
}

impl ProjectCoverageCache {
    fn new() -> Self {
        Self {
            cache: BoundedLruCache::new(MAX_COVERAGE_CACHE_ENTRIES),
        }
    }

    /// Return the slot for `key`, inserting one and evicting the
    /// least-recently-used entry if the cap would otherwise be exceeded.
    fn slot_for(&mut self, key: u64) -> CoverageSlot {
        if let Some(slot) = self.cache.touch(&key) {
            return Arc::clone(slot);
        }
        let slot: CoverageSlot = Arc::new(OnceLock::new());
        self.cache.insert(key, Arc::clone(&slot));
        slot
    }
}

fn project_coverage_cache() -> &'static Mutex<ProjectCoverageCache> {
    static CACHE: OnceLock<Mutex<ProjectCoverageCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(ProjectCoverageCache::new()))
}

/// Run `query_project_coverage` at most once per `DuckDb` per process.
///
/// Both the identity metrics provider and the coverage provider call this
/// in turn during `ops about`; the second caller gets the cached value
/// (including the cached `None` when the query failed and `query_or_warn`
/// already logged the warn).
///
/// CONC-2 / TASK-1193: keyed by an `Arc<OnceLock<...>>` per `DuckDb` id so
/// concurrent first-callers race only on the inner `OnceLock::get_or_init`
/// (which guarantees the closure runs exactly once). Pre-fix the outer
/// mutex was acquired, the entry checked, the guard dropped, and
/// `query_or_warn` then ran outside any lock — two threads entering at
/// the same time both observed a miss, both dispatched the query, and the
/// "warn fires exactly once" contract advertised by DUP-1 / TASK-1079
/// silently degraded to "warn fires once per concurrent first-caller".
pub fn cached_query_project_coverage(db: &DuckDb) -> Option<CrateCoverage> {
    let slot: CoverageSlot = {
        // DUP-1 / TASK-2150: the poison-recovering lock scaffold lives in
        // `ops_about::lru::lock_recovering`. Recovery is silent here: the
        // guarded value is the plain-data memoization map, and the worst
        // outcome of a missed poison is a recomputed query.
        let mut guard = ops_about::lru::lock_recovering(project_coverage_cache(), || {});
        guard.slot_for(db.id())
    };

    slot.get_or_init(|| {
        query_or_warn(
            "query_project_coverage",
            "reporting empty coverage",
            None,
            || query_project_coverage(db).map(Some),
        )
    })
    .clone()
}

pub struct RustCoverageProvider;

impl DataProvider for RustCoverageProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let manifest = match load_workspace_manifest(ctx) {
            Ok(m) => Some(m),
            Err(e) => {
                log_manifest_load_failure(&e);
                None
            }
        };

        let Some(db) = ops_duckdb::get_db(ctx) else {
            return Ok(serde_json::to_value(ProjectCoverage::default())?);
        };

        // ERR-2 / TASK-0376 / PATTERN-1 (TASK-0608): route through
        // `query_or_warn` so this site matches the convention used by every
        // sister DuckDB call in the crate (units, identity::metrics,
        // deps_provider). Wrapping the return in `Option` preserves the
        // early-return-on-failure semantics — if the project_coverage query
        // fails we return a fully-default `ProjectCoverage` rather than
        // partial data, matching the prior behaviour.
        // DUP-1 / TASK-1079: dispatched via `cached_query_project_coverage`
        // so the parallel call from `identity::metrics` reuses this result
        // (and any warn it already logged) instead of re-querying DuckDB
        // and double-warning per `ops about`.
        let Some(p) = cached_query_project_coverage(db) else {
            return Ok(serde_json::to_value(ProjectCoverage::default())?);
        };
        let total = CoverageStats::new(p.lines_percent, p.lines_covered, p.lines_count);

        let units = match manifest.as_ref() {
            // ERR-1 / TASK-1076: `resolved_members()` is the post-glob-expansion
            // list; the cached manifest preserves the original spec verbatim.
            Some(manifest) if !manifest.resolved_members().is_empty() => {
                // CL-3 / TASK-1762: join and key on the *resolved* workspace
                // root, not `ctx.working_directory` — running from a member
                // crate must not silently produce a blank per-crate table.
                let root = manifest.workspace_root();
                // READ-5 / TASK-0986: short-circuit when the workspace root is
                // not valid UTF-8 instead of piping a U+FFFD-replaced string
                // into the SQL key. The lossy collapse would silently match
                // an unrelated workspace's coverage rows. Sister policy to
                // TASK-0946 (workspace member relpaths in members.rs).
                let Some(root_str) = root.to_str() else {
                    tracing::warn!(
                        workspace_root = ?root.display(),
                        "non-UTF-8 workspace root; skipping per-crate coverage to avoid lossy SQL key collapse"
                    );
                    return Ok(serde_json::to_value(ProjectCoverage::new(
                        total,
                        Vec::new(),
                    ))?);
                };
                per_crate_units(db, manifest.resolved_members(), root, root_str)
            }
            _ => Vec::new(),
        };

        let coverage = ProjectCoverage::new(total, units);
        serde_json::to_value(&coverage).map_err(DataProviderError::from)
    }
}

/// Query per-crate coverage and pair each covered member with its display name.
fn per_crate_units(
    db: &DuckDb,
    members: &[String],
    workspace_root: &std::path::Path,
    workspace_root_str: &str,
) -> Vec<UnitCoverage> {
    let member_strs: Vec<&str> = members.iter().map(String::as_str).collect();
    let per_crate = query_or_warn(
        "query_crate_coverage",
        "per-crate coverage will be blank",
        HashMap::<String, CrateCoverage>::new(),
        || query_crate_coverage(db, &member_strs, workspace_root_str),
    );
    // PERF-1 (TASK-0798): resolve display names up front in one pass over
    // members with coverage rows, so each member's Cargo.toml is read at most
    // once per provide() call.
    let mut display_names: HashMap<&str, String> = HashMap::with_capacity(per_crate.len());
    for member in members {
        if per_crate.contains_key(member.as_str()) {
            display_names.insert(
                member.as_str(),
                resolve_crate_display_name(member, workspace_root),
            );
        }
    }
    members
        .iter()
        .filter_map(|member| {
            let cov = per_crate.get(member)?;
            // TEST-5 / TASK-2154: `query_crate_coverage` zero-fills members
            // whose LEFT JOIN matched no `coverage_files` row (COALESCE over
            // NULL sums), so "no data" arrives here as an all-zero
            // `CrateCoverage` that is indistinguishable from a measured zero.
            // Omit it: rendering a no-data member as "0% covered" claims a
            // measurement that never happened — a partial llvm-cov run would
            // show every uninstrumented crate at 0% instead of absent.
            // `lines_count == 0` is the discriminator: any member with real
            // rows carries a positive instrumented-line count.
            if cov.lines_count == 0 {
                return None;
            }
            let unit_name = display_names.remove(member.as_str())?;
            Some(UnitCoverage::new(
                unit_name,
                member.clone(),
                CoverageStats::new(cov.lines_percent, cov.lines_covered, cov.lines_count),
            ))
        })
        .collect()
}

#[cfg(test)]
mod cache_tests {
    use super::{
        cached_query_project_coverage, project_coverage_cache, ProjectCoverageCache,
        MAX_COVERAGE_CACHE_ENTRIES,
    };
    use ops_about::lru::{lock_recovering, VICTIM_QUEUE_SLACK};
    use ops_about::test_support::{capture_tracing, pin_global_dispatcher, TracingBuf};
    use ops_duckdb::DuckDb;
    use std::sync::Arc;

    fn cache_len() -> usize {
        lock_recovering(project_coverage_cache(), || {}).cache.len()
    }

    fn contains(key: u64) -> bool {
        lock_recovering(project_coverage_cache(), || {})
            .cache
            .contains_key(&key)
    }

    /// DUP-1 / TASK-1079: the identity-metrics and coverage providers used
    /// to dispatch their own `query_project_coverage` against the same
    /// `DuckDB` during a single `ops about`, so any `query_or_warn`
    /// schema-drift line fired twice. Pin that the per-process cache fires
    /// the underlying query (and its warn) exactly once across both call
    /// sites for a forced query failure.
    #[test]
    #[serial_test::serial(project_coverage_cache)]
    fn project_coverage_warn_fires_once_across_both_call_sites() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");

        // Force a hard failure inside `query_project_coverage`:
        // create `coverage_files` with the column `lines_count` typed as
        // VARCHAR, which makes the CASE/SUM aggregation in
        // `coverage_col_select` blow up with a type error. This is the
        // schema-drift scenario the DUP report cites.
        {
            let conn = db.lock().expect("lock");
            conn.execute_batch(
                "CREATE TABLE coverage_files (\
                    filename VARCHAR, \
                    lines_count VARCHAR, \
                    lines_covered VARCHAR, \
                    lines_percent VARCHAR\
                 ); \
                 INSERT INTO coverage_files VALUES ('a.rs', 'x', 'y', 'z');",
            )
            .expect("seed broken-schema coverage_files");
        }

        // Two call-site simulation: both providers run during a single
        // `ops about`, so we invoke the cache helper twice. The first call
        // dispatches the query and logs once; the second must hit the
        // cache and stay silent.
        let (logs, (first, second)) = capture_tracing(tracing::Level::WARN, || {
            let a = cached_query_project_coverage(&db);
            let b = cached_query_project_coverage(&db);
            (a, b)
        });

        // Both call sites observe the same fallback value (None) — failure
        // memoization is part of the contract.
        assert!(first.is_none(), "first call must hit fallback");
        assert!(second.is_none(), "second call must hit cached fallback");

        // Count the `query="query_project_coverage"` tracing field rather
        // than the bare substring: `query_or_warn` includes the label both
        // as a tracing field and (via the shared `query_project_row`
        // helper) inside the error context, so two substring matches per
        // emission is the expected post-DUP-1 shape. The contract being
        // pinned is that the *warn event* fires once, which the field
        // count uniquely identifies.
        let warn_count = logs.matches("query=\"query_project_coverage\"").count();
        assert_eq!(
            warn_count, 1,
            "warn must fire exactly once across both call sites; got {warn_count} in:\n{logs}"
        );
    }

    /// CONC-2 / TASK-1193: the AC #1 contract is that
    /// `cached_query_project_coverage` runs the underlying query exactly
    /// once even when two threads enter concurrently. Pre-fix the outer
    /// mutex was dropped around the query — both threads observed a miss,
    /// both dispatched, and `query_or_warn` fired its warn N times. We
    /// pin AC #2 by running the two call sites from two threads (rather
    /// than sequentially) and asserting the warn count is still 1.
    #[test]
    #[serial_test::serial(project_coverage_cache)]
    fn project_coverage_warn_fires_once_under_concurrent_first_callers() {
        let db = Arc::new(DuckDb::open_in_memory().expect("open in-memory db"));
        // Same broken-schema seed as the sister test.
        {
            let conn = db.lock().expect("lock");
            conn.execute_batch(
                "CREATE TABLE coverage_files (\
                    filename VARCHAR, \
                    lines_count VARCHAR, \
                    lines_covered VARCHAR, \
                    lines_percent VARCHAR\
                 ); \
                 INSERT INTO coverage_files VALUES ('a.rs', 'x', 'y', 'z');",
            )
            .expect("seed broken-schema coverage_files");
        }

        // Per-thread subscribers are the one shape `capture_tracing` cannot
        // provide (its subscriber is the calling thread's default), so this
        // test owns the buffer — and therefore has to pin the global
        // dispatcher itself, or `tracing` can cache `Interest::never()` for
        // the warn callsite and the capture comes back empty at random.
        pin_global_dispatcher();
        let captured = TracingBuf::default();

        let make_subscriber = || {
            tracing_subscriber::fmt()
                .with_writer(captured.clone())
                .with_max_level(tracing::Level::WARN)
                .with_ansi(false)
                .finish()
        };

        let barrier = Arc::new(std::sync::Barrier::new(2));
        let db_a = Arc::clone(&db);
        let bar_a = Arc::clone(&barrier);
        let sub_a = make_subscriber();
        let h_a = std::thread::spawn(move || {
            // Per-thread subscriber: `tracing::subscriber::with_default`
            // is thread-local, so each spawned thread must install its
            // own subscriber pointing at the shared buffer.
            tracing::subscriber::with_default(sub_a, move || {
                bar_a.wait();
                cached_query_project_coverage(&db_a)
            })
        });
        let db_b = Arc::clone(&db);
        let bar_b = Arc::clone(&barrier);
        let sub_b = make_subscriber();
        let h_b = std::thread::spawn(move || {
            tracing::subscriber::with_default(sub_b, move || {
                bar_b.wait();
                cached_query_project_coverage(&db_b)
            })
        });
        let _ = h_a.join().unwrap();
        let _ = h_b.join().unwrap();
        let logs = captured.captured();

        // See sibling test: count the structured `query=` field so the
        // label-appearing-twice (context + tracing field) does not double
        // the substring match.
        let warn_count = logs.matches("query=\"query_project_coverage\"").count();
        assert_eq!(
            warn_count, 1,
            "warn must fire exactly once under concurrent first-callers; got {warn_count} in:\n{logs}"
        );
    }

    /// ARCH-9 / TASK-1155: two distinct `DuckDb` instances must receive
    /// distinct cache keys even when one is dropped and the next is
    /// allocated at the same memory address (the ABA hazard the prior
    /// pointer-address scheme had). With the id-keyed scheme each instance
    /// gets a fresh monotonic id, so a re-allocated address cannot
    /// silently surface a previous instance's cached value.
    ///
    /// TEST-1 / TASK-1571: drive the contract through
    /// `cached_query_project_coverage` itself (the cache aliasing API)
    /// rather than asserting on `DuckDb::id`.
    #[test]
    #[serial_test::serial(project_coverage_cache)]
    fn distinct_db_instances_do_not_alias_cache_via_aba() {
        // Open `a`, prime the cache, then drop it. With an open in-memory
        // DuckDb the `coverage_summary` view doesn't exist, so the
        // primed entry is the `None` from `query_row` returning a
        // QueryReturnedNoRows error path — we record the *fact* of
        // priming via the slot's existence rather than its payload.
        let a_id = {
            let a = DuckDb::open_in_memory().expect("open a");
            let id = a.id();
            let _primed = cached_query_project_coverage(&a);
            assert!(
                contains(id),
                "priming must insert a slot for instance a's id"
            );
            id
        };
        // After `a` drops, a fresh instance must mint a new id even if
        // the allocator reuses the address — and its cache slot must be
        // populated from scratch under that new id, not surface `a`'s.
        let b = DuckDb::open_in_memory().expect("open b");
        let b_id = b.id();
        assert_ne!(
            a_id, b_id,
            "ABA-resistant id allocator: post-drop reallocation must not reuse a's id"
        );
        let _b_payload = cached_query_project_coverage(&b);
        assert!(
            contains(b_id),
            "b's lookup must populate a slot under its own id"
        );
    }

    /// PERF-16 / TASK-1764 AC #3: the memoization map is bounded. Every
    /// `DuckDb` a process opens mints a fresh monotonic id, so without a cap
    /// this map grew one permanent slot per instance — an unbounded leak in a
    /// daemon or CI worker that opens a handle per project or per refresh.
    #[test]
    #[serial_test::serial(project_coverage_cache)]
    fn project_coverage_cache_stays_bounded_across_many_db_instances() {
        // Each instance is dropped immediately; the point is that the cache
        // must not retain a slot for every id it has ever seen.
        for _ in 0..(MAX_COVERAGE_CACHE_ENTRIES * 3) {
            let db = DuckDb::open_in_memory().expect("open in-memory db");
            let _ = cached_query_project_coverage(&db);
        }
        let len = cache_len();
        assert!(
            len <= MAX_COVERAGE_CACHE_ENTRIES,
            "cache size {len} must stay within MAX_COVERAGE_CACHE_ENTRIES = {MAX_COVERAGE_CACHE_ENTRIES}"
        );
    }

    /// PERF-16 / TASK-1723: the *map* was bounded by the test above, but the
    /// LRU victim queue was not. Its only drain runs at the cap, so a process
    /// staying below the cap — every CLI run, which memoizes exactly one
    /// project — pushed one stamp per `slot_for` call and never dropped any.
    /// Drive the cache directly (no global, no `DuckDb`) and pin that the queue
    /// stays proportional to the live entry count, not to the call count.
    #[test]
    fn repeated_hits_below_cap_keep_the_victim_queue_bounded() {
        let mut cache = ProjectCoverageCache::new();
        const KEYS: u64 = 3;
        const PASSES: usize = 500;
        for _ in 0..PASSES {
            for key in 0..KEYS {
                let _ = cache.slot_for(key);
            }
        }

        let map_len = cache.cache.len();
        assert_eq!(map_len, usize::try_from(KEYS).unwrap());
        assert!(
            map_len < MAX_COVERAGE_CACHE_ENTRIES,
            "the test must stay below the cap or it stops covering the leak"
        );
        // Without compaction this is KEYS * PASSES == 1500 stamps.
        let bound = map_len.saturating_mul(2).saturating_add(VICTIM_QUEUE_SLACK);
        let queue_len = cache.cache.victim_queue_len();
        assert!(
            queue_len <= bound,
            "victim queue holds {queue_len} stamps after {} accesses of {map_len} keys; \
             expected at most {bound} — it is growing with the access count again",
            usize::try_from(KEYS).unwrap().saturating_mul(PASSES)
        );
    }
}

#[cfg(all(test, unix))]
mod tests {

    /// READ-5 / TASK-0986 + TEST-25 / TASK-1773: a non-UTF-8 workspace root
    /// must NOT collapse to a U+FFFD-replaced SQL key — the lossy key would
    /// silently match an unrelated workspace's coverage rows.
    ///
    /// This drives `RustCoverageProvider::provide` itself and asserts on the
    /// provider's observable behaviour (project total present, per-crate table
    /// blank, warn emitted). Replacing the `to_str()` short-circuit with
    /// `to_string_lossy()` removes the warn and makes this fail. The previous
    /// shape asserted only that `Path::to_str()` returns `None` for invalid
    /// UTF-8 — a `std` guarantee that stays green through exactly that
    /// regression.
    #[test]
    // macOS-impossible: APFS refuses to create directory names that are not
    // valid UTF-8 (`create_dir` fails with `Illegal byte sequence`), so the
    // non-UTF-8 workspace-root fixture cannot exist there. Linux CI covers it.
    #[cfg(not(target_os = "macos"))]
    #[serial_test::serial(typed_manifest_cache, project_coverage_cache)]
    fn non_utf8_workspace_root_skips_per_crate_coverage_with_warn() {
        use super::RustCoverageProvider;
        use ops_about::test_support::capture_tracing;
        use ops_duckdb::DuckDb;
        use ops_extension::{Context, DataProvider};
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        use std::sync::Arc;
        let dir = tempfile::tempdir().expect("tempdir");
        // 0xC3 0x28 is an invalid UTF-8 sequence.
        let mut bytes = dir.path().as_os_str().as_bytes().to_vec();
        bytes.extend_from_slice(b"/ws-\xC3\x28");
        let root = std::path::PathBuf::from(OsStr::from_bytes(&bytes));
        assert!(root.to_str().is_none(), "test premise: root is not UTF-8");

        std::fs::create_dir_all(root.join("crates/foo")).expect("create ws");
        std::fs::write(
            root.join("crates/foo/Cargo.toml"),
            "[package]\nname=\"foo\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n",
        )
        .unwrap();

        let db = DuckDb::open_in_memory().expect("open in-memory db");
        {
            let conn = db.lock().expect("lock");
            conn.execute_batch(
                "CREATE TABLE coverage_files (\
                    filename VARCHAR, \
                    lines_count BIGINT, \
                    lines_covered BIGINT, \
                    lines_percent DOUBLE\
                 ); \
                 INSERT INTO coverage_files VALUES ('crates/foo/src/lib.rs', 10, 5, 50.0);",
            )
            .expect("seed coverage_files");
        }

        let mut ctx = Context::test_context(root);
        ctx.attach_db(Arc::new(db));

        let (logs, value) = capture_tracing(tracing::Level::WARN, || {
            RustCoverageProvider.provide(&mut ctx).expect("provide")
        });

        assert_eq!(
            value.get("total").and_then(|t| t.get("lines_count")),
            Some(&serde_json::json!(10)),
            "the project total must still be reported, got: {value}"
        );
        assert_eq!(
            value.get("units").and_then(|u| u.as_array()).map(Vec::len),
            Some(0),
            "per-crate coverage must be skipped for a non-UTF-8 root, got: {value}"
        );
        assert!(
            logs.contains("non-UTF-8 workspace root"),
            "the short-circuit must leave a breadcrumb, got: {logs}"
        );
    }
}

/// TEST-5 / TASK-2154: happy-path coverage for `per_crate_units` and
/// `RustCoverageProvider::provide`. Before this module the provider was
/// driven by exactly one test — the non-UTF-8-root skip branch — so the
/// row→unit mapping, the project total, and the default arms of `provide`
/// had no test at all. These tests mirror the coverage shape the sibling
/// `deps_provider` tests already establish (no-DB default, query-failure
/// fallback with warn, multi-row mapping) and are platform-independent.
///
/// Cross-stack note (TASK-2154 AC #4): the Go twin of this coverage shape
/// lives in `extensions-go/about` (`units_provider_*` tests, TASK-2184) —
/// keep the two stacks' provider-level coverage consistent.
#[cfg(test)]
mod provider_tests {
    use super::{per_crate_units, RustCoverageProvider};
    use ops_about::test_support::capture_tracing;
    use ops_duckdb::DuckDb;
    use ops_extension::{Context, DataProvider};
    use std::path::PathBuf;
    use std::sync::Arc;

    /// Seed `coverage_files` with (`filename`, `lines_count`, `lines_covered`)
    /// rows. `lines_percent` is computed by the SUM/CASE aggregation, so the
    /// fixture does not carry it. All values are static test constants
    /// interpolated into one batch string (the crate has no direct `duckdb`
    /// dependency for bound parameters).
    fn seed_coverage(db: &DuckDb, rows: &[(&str, i64, i64)]) {
        use std::fmt::Write as _;
        let mut sql = String::from(
            "CREATE TABLE coverage_files (\
                filename VARCHAR, \
                lines_count BIGINT, \
                lines_covered BIGINT\
             );",
        );
        for (filename, count, covered) in rows {
            write!(
                sql,
                " INSERT INTO coverage_files VALUES ('{filename}', {count}, {covered});"
            )
            .expect("write to String cannot fail");
        }
        let conn = db.lock().expect("lock");
        conn.execute_batch(&sql).expect("seed coverage_files");
    }

    /// A three-member Cargo workspace: `foo` and `bar` get coverage rows,
    /// `baz` gets none. Package names deliberately differ from directory
    /// names so a name/path transposition in the mapping fails the test.
    /// The returned `TempDir` keeps every path alive for the test.
    fn workspace_fixture(tag: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join(tag);
        for (dir_name, pkg) in [
            ("crates/foo", "alpha-crate"),
            ("crates/bar", "beta-crate"),
            ("crates/baz", "gamma-crate"),
        ] {
            std::fs::create_dir_all(root.join(dir_name)).expect("member dir");
            std::fs::write(
                root.join(dir_name).join("Cargo.toml"),
                format!("[package]\nname = \"{pkg}\"\nversion = \"0.1.0\"\n"),
            )
            .expect("member manifest");
        }
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n",
        )
        .expect("root manifest");
        (dir, root)
    }

    /// TEST-5 / TASK-2154 AC #1: rows map onto one `UnitCoverage` per covered
    /// member with the display name from the member manifest, the member
    /// path, and the (percent, covered, count) triple in the right order —
    /// distinct magnitudes per field so a transposition cannot pass.
    #[test]
    #[serial_test::serial(typed_manifest_cache, project_coverage_cache)]
    fn per_crate_units_maps_rows_onto_named_units() {
        let (_dir, root) = workspace_fixture("map-rows");
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        seed_coverage(
            &db,
            &[
                // foo: 150 count, 105 covered → 70.0%
                ("crates/foo/src/lib.rs", 100, 80),
                ("crates/foo/src/util.rs", 50, 25),
                // bar: 10 count, 1 covered → 10.0%
                ("crates/bar/src/lib.rs", 10, 1),
            ],
        );
        let root_str = root.to_str().expect("root is UTF-8");

        let units = per_crate_units(
            &db,
            &["crates/foo".to_string(), "crates/bar".to_string()],
            &root,
            root_str,
        );

        assert_eq!(units.len(), 2, "one unit per covered member: {units:?}");
        let foo = &units[0];
        assert_eq!(foo.unit_name, "alpha-crate", "display name from manifest");
        assert_eq!(foo.unit_path, "crates/foo", "member path verbatim");
        // Distinct magnitudes: percent 70.0, covered 105, count 150 — any
        // argument-order transposition in `CoverageStats::new` fails here.
        assert!((foo.stats.lines_percent - 70.0).abs() < f64::EPSILON);
        assert_eq!(foo.stats.lines_covered, 105);
        assert_eq!(foo.stats.lines_count, 150);
        let bar = &units[1];
        assert_eq!(bar.unit_name, "beta-crate");
        assert_eq!(bar.unit_path, "crates/bar");
        assert!((bar.stats.lines_percent - 10.0).abs() < f64::EPSILON);
        assert_eq!(bar.stats.lines_covered, 1);
        assert_eq!(bar.stats.lines_count, 10);
    }

    /// TEST-5 / TASK-2154 AC #2: a member with no coverage row is omitted
    /// from the per-crate list, not emitted with zeroed stats.
    /// `query_crate_coverage` zero-fills such members (LEFT JOIN + COALESCE),
    /// so the omission has to happen in `per_crate_units` — pin it.
    #[test]
    #[serial_test::serial(typed_manifest_cache, project_coverage_cache)]
    fn per_crate_units_omits_members_without_coverage_rows() {
        let (_dir, root) = workspace_fixture("omit-uncovered");
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        seed_coverage(&db, &[("crates/foo/src/lib.rs", 100, 80)]);
        let root_str = root.to_str().expect("root is UTF-8");

        let units = per_crate_units(
            &db,
            &[
                "crates/foo".to_string(),
                "crates/baz".to_string(), // no coverage rows
            ],
            &root,
            root_str,
        );

        assert_eq!(
            units.len(),
            1,
            "a member with no coverage data must be omitted, not zeroed: {units:?}"
        );
        assert_eq!(units[0].unit_path, "crates/foo");
        assert!(
            !units.iter().any(|u| u.unit_path == "crates/baz"),
            "baz must not appear with zeroed stats: {units:?}"
        );
    }

    /// TEST-5 / TASK-2154 AC #3: `provide` end to end against a real
    /// workspace fixture plus a seeded `DuckDB` — project total and per-crate
    /// table both asserted.
    #[test]
    #[serial_test::serial(typed_manifest_cache, project_coverage_cache)]
    fn provide_reports_project_total_and_per_crate_table() {
        let (_dir, root) = workspace_fixture("end-to-end");
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        seed_coverage(
            &db,
            &[
                ("crates/foo/src/lib.rs", 100, 80),
                ("crates/foo/src/util.rs", 50, 25),
                ("crates/bar/src/lib.rs", 10, 1),
            ],
        );

        let mut ctx = Context::test_context(root);
        ctx.attach_db(Arc::new(db));
        let (logs, value) = capture_tracing(tracing::Level::WARN, || {
            RustCoverageProvider.provide(&mut ctx).expect("provide")
        });
        assert!(logs.is_empty(), "a healthy query must not warn: {logs}");

        // Project total: 160 count, 106 covered → 66.25%.
        assert_eq!(
            value["total"]["lines_count"],
            serde_json::json!(160),
            "project total counts every row: {value}"
        );
        assert_eq!(value["total"]["lines_covered"], serde_json::json!(106));
        assert_eq!(value["total"]["lines_percent"], serde_json::json!(66.25));

        let units = value["units"].as_array().expect("units array");
        assert_eq!(units.len(), 2, "foo and bar covered, baz not: {value}");
        let by_path: std::collections::BTreeMap<&str, &serde_json::Value> = units
            .iter()
            .map(|u| (u["unit_path"].as_str().expect("path"), u))
            .collect();
        let foo = by_path.get("crates/foo").expect("foo unit");
        assert_eq!(foo["unit_name"], "alpha-crate");
        assert_eq!(foo["stats"]["lines_percent"], serde_json::json!(70.0));
        assert_eq!(foo["stats"]["lines_covered"], serde_json::json!(105));
        assert_eq!(foo["stats"]["lines_count"], serde_json::json!(150));
    }

    /// TEST-5 / TASK-2154 AC #4: no `DuckDB` in the context serialises a
    /// default (empty but well-formed) `ProjectCoverage`, not an error —
    /// matching `deps_provider`'s no-DB test.
    #[test]
    #[serial_test::serial(typed_manifest_cache, project_coverage_cache)]
    fn provide_without_duckdb_yields_default_project_coverage() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut ctx = Context::test_context(dir.path().to_path_buf());
        let (logs, value) = capture_tracing(tracing::Level::WARN, || {
            RustCoverageProvider.provide(&mut ctx).expect("provide")
        });
        assert_eq!(
            value,
            serde_json::json!({
                "total": {
                    "lines_percent": 0.0,
                    "lines_covered": 0,
                    "lines_count": 0
                },
                "units": []
            }),
            "no DuckDB must yield the default ProjectCoverage: {value}"
        );
        assert!(
            logs.is_empty(),
            "an absent DuckDB is not a degraded mode; no warn expected: {logs}"
        );
    }

    /// TEST-5 / TASK-2154 AC #4: a `query_project_coverage` failure warns and
    /// falls back to the default `ProjectCoverage` (ERR-2 convention shared
    /// with `deps_provider`). The seeded `coverage_files` table types
    /// `lines_count` as VARCHAR so the SUM aggregation fails while
    /// `table_exists` still passes.
    #[test]
    #[serial_test::serial(typed_manifest_cache, project_coverage_cache)]
    fn provide_warns_and_falls_back_when_the_query_fails() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        {
            let conn = db.lock().expect("lock");
            conn.execute_batch(
                "CREATE TABLE coverage_files (\
                    filename VARCHAR, \
                    lines_count VARCHAR, \
                    lines_covered VARCHAR\
                 ); \
                 INSERT INTO coverage_files VALUES ('a.rs', 'x', 'y');",
            )
            .expect("seed broken-schema coverage_files");
        }

        let mut ctx = Context::test_context(dir.path().to_path_buf());
        ctx.attach_db(Arc::new(db));
        let (logs, value) = capture_tracing(tracing::Level::WARN, || {
            RustCoverageProvider.provide(&mut ctx).expect("provide")
        });

        assert_eq!(
            value["units"].as_array().map(Vec::len),
            Some(0),
            "the fallback must be a valid empty ProjectCoverage: {value}"
        );
        assert_eq!(value["total"]["lines_count"], serde_json::json!(0));
        assert!(
            logs.contains("query=\"query_project_coverage\""),
            "the failure must warn before falling back: {logs}"
        );
    }

    /// TEST-5 / TASK-2154: the manifest-failed arm — a `DuckDB` with rows but
    /// no workspace manifest at cwd reports the project total with an empty
    /// per-crate table, not an error.
    #[test]
    #[serial_test::serial(typed_manifest_cache, project_coverage_cache)]
    fn provide_without_manifest_reports_total_with_empty_units() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        seed_coverage(&db, &[("crates/foo/src/lib.rs", 100, 80)]);

        let mut ctx = Context::test_context(dir.path().to_path_buf());
        ctx.attach_db(Arc::new(db));
        let (_logs, value) = capture_tracing(tracing::Level::WARN, || {
            RustCoverageProvider.provide(&mut ctx).expect("provide")
        });

        assert_eq!(
            value["total"]["lines_count"],
            serde_json::json!(100),
            "the project total must survive a missing manifest: {value}"
        );
        assert_eq!(
            value["units"].as_array().map(Vec::len),
            Some(0),
            "no manifest → no per-crate table: {value}"
        );
    }
}
