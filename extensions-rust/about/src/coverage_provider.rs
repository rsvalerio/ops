//! Rust `project_coverage` data provider.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use ops_about::lru::{next_lru_tick, LruVictimQueue};
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

/// Slack added to the victim-queue compaction threshold.
///
/// PERF-16 / TASK-1723: mirrors the typed-manifest cache in
/// [`crate::manifest_cache`]. Without it a cache holding a single project
/// would compact on every other access.
const COVERAGE_VICTIM_QUEUE_SLACK: usize = 16;

struct CoverageCacheEntry {
    slot: CoverageSlot,
    last_accessed: u64,
}

struct ProjectCoverageCache {
    map: HashMap<u64, CoverageCacheEntry>,
    victim_queue: LruVictimQueue<u64>,
}

impl ProjectCoverageCache {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            victim_queue: LruVictimQueue::new(),
        }
    }

    /// Return the slot for `key`, inserting one and evicting the
    /// least-recently-used entry if the cap would otherwise be exceeded.
    fn slot_for(&mut self, key: u64) -> CoverageSlot {
        let tick = next_lru_tick();
        if let Some(entry) = self.map.get_mut(&key) {
            entry.last_accessed = tick;
            let slot = Arc::clone(&entry.slot);
            self.record_access(key, tick);
            return slot;
        }
        if self.map.len() >= MAX_COVERAGE_CACHE_ENTRIES {
            self.evict_lru();
        }
        let slot: CoverageSlot = Arc::new(OnceLock::new());
        self.map.insert(
            key,
            CoverageCacheEntry {
                slot: Arc::clone(&slot),
                last_accessed: tick,
            },
        );
        self.record_access(key, tick);
        slot
    }

    /// Stamp an access against `key` and keep the victim queue bounded.
    ///
    /// PERF-16 / TASK-1723: same leak as the typed-manifest cache — every
    /// hit pushes a stamp, but the only drain ([`Self::evict_lru`]) runs
    /// solely at the cap. A process that stays below
    /// [`MAX_COVERAGE_CACHE_ENTRIES`] (every CLI run: one project) never
    /// drained the queue at all, so it grew by one `(u64, u64)` per
    /// `cached_query_project_coverage` call for the process lifetime.
    ///
    /// Compaction leaves exactly one stamp per live entry, so at least
    /// `map.len() + COVERAGE_VICTIM_QUEUE_SLACK` further pushes must land
    /// before it can trigger again: amortised `O(1)` per access. It drops
    /// only stamps `pop_lru` would already have skipped as stale, so
    /// eviction ordering is unchanged.
    ///
    /// Must be called *after* the map holds `key` at `tick`, or the
    /// freshness check would compact away the stamp just pushed.
    fn record_access(&mut self, key: u64, tick: u64) {
        let Self { map, victim_queue } = self;
        victim_queue.push(tick, key);
        let threshold = map
            .len()
            .saturating_mul(2)
            .saturating_add(COVERAGE_VICTIM_QUEUE_SLACK);
        if victim_queue.len() > threshold {
            victim_queue
                .retain_fresh(|key, tick| map.get(key).is_some_and(|e| e.last_accessed == tick));
        }
    }

    fn evict_lru(&mut self) {
        let map = &mut self.map;
        if let Some(victim) = self
            .victim_queue
            .pop_lru(|key, tick| map.get(key).is_some_and(|e| e.last_accessed == tick))
        {
            map.remove(&victim);
        }
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
        let mut guard = project_coverage_cache()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
        COVERAGE_VICTIM_QUEUE_SLACK, MAX_COVERAGE_CACHE_ENTRIES,
    };
    use ops_about::test_support::TracingBuf;
    use ops_duckdb::DuckDb;
    use std::sync::Arc;

    fn cache_len() -> usize {
        project_coverage_cache()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .map
            .len()
    }

    fn contains(key: u64) -> bool {
        project_coverage_cache()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .map
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

        let buf = TracingBuf::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf.clone())
            .with_max_level(tracing::Level::WARN)
            .with_ansi(false)
            .finish();

        // Two call-site simulation: both providers run during a single
        // `ops about`, so we invoke the cache helper twice. The first call
        // dispatches the query and logs once; the second must hit the
        // cache and stay silent.
        let (first, second) = tracing::subscriber::with_default(subscriber, || {
            let a = cached_query_project_coverage(&db);
            let b = cached_query_project_coverage(&db);
            (a, b)
        });

        // Both call sites observe the same fallback value (None) — failure
        // memoization is part of the contract.
        assert!(first.is_none(), "first call must hit fallback");
        assert!(second.is_none(), "second call must hit cached fallback");

        let logs = buf.captured();
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

        let map_len = cache.map.len();
        assert_eq!(map_len, usize::try_from(KEYS).unwrap());
        assert!(
            map_len < MAX_COVERAGE_CACHE_ENTRIES,
            "the test must stay below the cap or it stops covering the leak"
        );
        // Without compaction this is KEYS * PASSES == 1500 stamps.
        let bound = map_len
            .saturating_mul(2)
            .saturating_add(COVERAGE_VICTIM_QUEUE_SLACK);
        let queue_len = cache.victim_queue.len();
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
    use super::RustCoverageProvider;
    use ops_about::test_support::TracingBuf;
    use ops_duckdb::DuckDb;
    use ops_extension::{Context, DataProvider};
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::sync::Arc;

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
    #[serial_test::serial(typed_manifest_cache, project_coverage_cache)]
    fn non_utf8_workspace_root_skips_per_crate_coverage_with_warn() {
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

        let buf = TracingBuf::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf.clone())
            .with_max_level(tracing::Level::WARN)
            .with_ansi(false)
            .finish();
        let value = tracing::subscriber::with_default(subscriber, || {
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
        let logs = buf.captured();
        assert!(
            logs.contains("non-UTF-8 workspace root"),
            "the short-circuit must leave a breadcrumb, got: {logs}"
        );
    }
}
