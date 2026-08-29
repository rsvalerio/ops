//! Bounded, freshness-checked process cache of parsed workspace manifests.
//!
//! ARCH-1 / TASK-1791: extracted verbatim from the former `query.rs` so the
//! concurrency contract below sits at the top of the file it governs rather
//! than in the middle of a module that also expanded workspace globs.
//!
//! # Cache contract
//!
//! - **Key**: the resolved workspace root (`find_workspace_root_strict` output,
//!   canonicalised). CL-3 / TASK-1762: keying by the *root* rather than by
//!   `ctx.working_directory` means two cwds inside the same workspace share one
//!   entry and one freshness key.
//! - **Value**: [`LoadedManifest`] — the parsed `Arc<CargoToml>`, its resolved
//!   member list, and the lazily-built canonical member-manifest map.
//! - **Maximum size**: [`MAX_TYPED_MANIFEST_CACHE_ENTRIES`], enforced on insert
//!   with LRU eviction.
//! - **Invalidation**: the `<root>/Cargo.toml` mtime+len pair is re-stat'ed on
//!   every probe; a mismatch reparses. `ctx.refresh` evicts the entry outright.
//!
//! PERF-1 / TASK-2028: keying by the root means the root must be resolved
//! *before* the probe, which would otherwise put a canonicalizing ancestor walk
//! in front of every hit and blow the one-`stat` hot-path budget the freshness
//! design above is chosen for. [`crate::workspace_root_cache`] memoizes that
//! walk per cwd, so the budget still describes what the hit path actually does.
//!
//! # Why a cache at all
//!
//! PERF-1 / TASK-0558: identity, units, and coverage providers each call
//! `load_workspace_manifest` during a single `ops about` invocation. The
//! previous implementation cloned the cached `serde_json::Value` and
//! re-deserialized it every time, even though the resolved manifest is
//! identical across providers.
//!
//! ARCH-2 (TASK-0795): the cache lives in a process-global `Mutex<HashMap>`
//! rather than a `thread_local!`. The previous thread-local was invisible to
//! providers scheduled on a different worker thread (e.g. a future tokio
//! fan-out), silently degrading the cache to "off" with no signal. The mutex is
//! held only for the lookup / insert and never across provider work, so
//! contention is bounded; readers clone the `Arc<CargoToml>` so the typed
//! manifest is shared across threads with no reparse.
//!
//! ERR-1 / TASK-0844: the cache lock is acquired exclusively through
//! [`lock_typed_manifest_cache`], which recovers from `PoisonError` via
//! `into_inner` + `clear_poison` and warns on every recovery. Without this, a
//! panic in a sibling provider would silently degrade the cache to
//! "always-miss" — the same invisibility class the thread-local rewrite fought
//! against, just routed through a different mechanism.
//!
//! # CONC-7 / TASK-1163: concurrency contract
//!
//! The wrapper is a single `Mutex<HashMap>`, which serialises every probe.
//! This is intentional and bounded by the workload it guards:
//!
//! - **Single-shot CLI (`ops about`):** every provider runs in turn from one
//!   thread, so the lock is uncontended. The hot path is `HashMap` probe + LRU
//!   tick under the lock and never holds the lock across IO or parsing.
//! - **Daemon / language-server hosts:** when multiple worker threads start
//!   running providers in parallel against many distinct workspace roots, this
//!   single-mutex shape becomes the bottleneck (CONC-7 forbids
//!   `Mutex<Collection>` on hot paths exactly because of this). At that point
//!   the cache MUST migrate to `DashMap<PathBuf, TypedManifestEntry>` plus a
//!   separate small `parking_lot::Mutex<()>` only for the LRU eviction scan —
//!   sharded reads, occasional global serialisation just for the cap-evict
//!   step. Keep this comment in sync with
//!   `extensions/about/src/manifest_cache.rs` (TASK-1144 already moved that
//!   sibling cache to a per-key `OnceLock` shape so distinct paths progress in
//!   parallel; the typed-manifest cache here intentionally lags because no
//!   daemon caller exists yet).
//!
//! Reviewer rule: do not add a daemon caller without first making the migration
//! above. A new caller that opens parallel `ctx`s against distinct roots and
//! bottlenecks here would silently undo a downstream performance fix.
//!
//! TEST-15 / TASK-1664: **every test that reaches this cache must carry
//! `#[serial_test::serial(typed_manifest_cache)]`** — including the ones that
//! reach it indirectly through a provider's `provide()` →
//! `load_workspace_manifest`. `lock_typed_manifest_cache` recovers by calling
//! `clear_poison()`, so a poisoned lock produces exactly one warn and the first
//! caller to recover consumes it. The poison tests in this module were
//! serialised against each other but raced 14 unserialised tests in
//! `identity/mod.rs` and `units.rs`, which reach the same static through their
//! providers. That passed on a workstation and failed on a 2-core CI runner.

use ops_about::lru::{next_lru_tick, LruVictimQueue};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

use crate::manifest::LoadedManifest;

/// CONC-2 / TASK-0843: soft upper bound on the typed-manifest cache so a
/// long-running daemon process that visits an unbounded set of workspaces
/// (CI worker, language-server-style host) does not accumulate one entry
/// per root indefinitely. When the cap is hit on insert we evict the
/// least-recently-used entry (LRU) so steady-state hits remain warm.
pub const MAX_TYPED_MANIFEST_CACHE_ENTRIES: usize = 64;

/// Slack added to the victim-queue compaction threshold.
///
/// PERF-16 / TASK-1723: without it a cache holding a single root would
/// compact on every other access. Sixteen stale stamps is a few hundred
/// bytes and buys amortisation for the small-root-count shape the CLI
/// actually runs. Matches the sibling raw-text cache in
/// `extensions/about/src/manifest_cache.rs`, per the lockstep contract.
const VICTIM_QUEUE_SLACK: usize = 16;

/// CONC-2 / TASK-1198: cache freshness key. Pairs the file's mtime with
/// its byte length so two writes within the same mtime tick (HFS+, FAT,
/// NFS with old `actimeo` all expose second-resolution mtime) cannot
/// silently keep serving the pre-edit manifest. Length-equal collisions
/// inside one second remain possible but require both the same byte
/// length AND identical mtime — far less likely than mtime alone.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ManifestFreshness {
    mtime: SystemTime,
    len: u64,
}

struct TypedManifestEntry {
    /// `None` means we couldn't stat the file at parse time; the legacy
    /// "always trust the cache until ctx.refresh" behaviour applies.
    freshness: Option<ManifestFreshness>,
    loaded: LoadedManifest,
    last_accessed: u64,
    /// PERF-3 / TASK-1572: `Arc<PathBuf>` key shared with the victim
    /// queue so the cache-hit path can `Arc::clone` instead of cloning
    /// the underlying `PathBuf` on every LRU tick refresh. The map key
    /// is still `PathBuf` (`HashMap` doesn't accept `&Path` lookups via
    /// `Arc`); this is the same allocation, referenced twice.
    key: Arc<PathBuf>,
}

/// PERF-1 / TASK-1240: pair the root→entry map with a min-heap of
/// `(last_accessed, root)` so cap-bound eviction picks the LRU entry in
/// `O(log n)` (heap pop with lazy invalidation) instead of an `O(n)`
/// scan. Kept in lockstep with `manifest_cache::CacheMap` per the
/// module-level lockstep contract.
pub struct TypedManifestCache {
    map: HashMap<PathBuf, TypedManifestEntry>,
    /// PERF-3 / TASK-1572: queue keys are `Arc<PathBuf>` so the
    /// cache-hit path (the dominant case once any provider has primed
    /// the cache) can refresh the LRU tick by cloning an Arc rather
    /// than allocating a fresh `PathBuf`. The Arc is shared with the
    /// `TypedManifestEntry::key` slot so both refer to the same heap
    /// allocation.
    victim_queue: LruVictimQueue<Arc<PathBuf>>,
}

impl TypedManifestCache {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            victim_queue: LruVictimQueue::new(),
        }
    }

    /// Stamp an access against `key` and keep the victim queue bounded.
    ///
    /// PERF-16 / TASK-1723: every probe hit and every insert pushes a fresh
    /// `(tick, key)` stamp, but the only drain ([`Self::evict_lru`]) runs
    /// solely once the map is at [`MAX_TYPED_MANIFEST_CACHE_ENTRIES`]. Below
    /// the cap — the overwhelmingly common case, since a single `ops about`
    /// run touches one root and every provider hits it — the queue was never
    /// drained at all and grew by one `(u64, Arc<PathBuf>)` per access
    /// forever. A long-running embedder re-probing a handful of roots kept a
    /// tiny map behind a queue that grew linearly with its uptime.
    ///
    /// Compacting once the queue passes `2 * map.len() + VICTIM_QUEUE_SLACK`
    /// bounds it at that multiple of the live entry count while staying
    /// amortised `O(1)` per access: compaction leaves exactly one stamp per
    /// live entry, so at least `map.len() + VICTIM_QUEUE_SLACK` further
    /// pushes must land before it can trigger again, and each compaction is
    /// `O(queue len)`.
    ///
    /// Call this *after* the map has been updated — the freshness check reads
    /// `map[key].last_accessed`, so a pre-update call would compact away the
    /// stamp it just pushed. Compaction preserves eviction ordering: it drops
    /// only stamps `pop_lru` would already have skipped as stale.
    fn record_access(&mut self, key: Arc<PathBuf>, tick: u64) {
        let Self { map, victim_queue } = self;
        victim_queue.push(tick, key);
        let threshold = map
            .len()
            .saturating_mul(2)
            .saturating_add(VICTIM_QUEUE_SLACK);
        if victim_queue.len() > threshold {
            victim_queue.retain_fresh(|key, tick| {
                map.get(key.as_ref())
                    .is_some_and(|e| e.last_accessed == tick)
            });
        }
    }

    fn evict_lru(&mut self) -> Option<PathBuf> {
        let map = &mut self.map;
        let victim = self.victim_queue.pop_lru(|path, tick| {
            map.get(path.as_ref())
                .is_some_and(|e| e.last_accessed == tick)
        })?;
        map.remove(victim.as_ref());
        // PERF-3 / TASK-1572: the only outstanding Arc references at
        // this point are the queue entry we just popped and the map
        // entry we just removed (now dropped), so `try_unwrap` succeeds
        // on the common path. On the rare contention case (a clone
        // outliving the eviction) we fall back to a single clone of
        // the inner PathBuf.
        Some(Arc::try_unwrap(victim).unwrap_or_else(|arc| (*arc).clone()))
    }

    /// FN-1 / TASK-1780: the freshness comparison plus the LRU tick refresh,
    /// as one named operation so the probe's lock scope is verifiable in
    /// isolation.
    ///
    /// Returns the cached [`LoadedManifest`] only when the entry exists and is
    /// still fresh; a stale entry is left in place for the insert path to
    /// overwrite.
    fn probe(&mut self, root: &Path, current: Option<ManifestFreshness>) -> Option<LoadedManifest> {
        let entry = self.map.get_mut(root)?;
        // CONC-2 / TASK-0843 + TASK-1198: serve the cached Arc only
        // when both the mtime AND the byte length match. Pairing
        // mtime with size closes the second-resolution-mtime window
        // (HFS+, FAT, NFS with old `actimeo`): two writes inside the
        // same second can produce identical mtimes, so mtime alone
        // happily served the pre-edit manifest until the next tick.
        // If we couldn't stat at all, fall back to the legacy "trust
        // until refresh" behaviour.
        let still_fresh = match (entry.freshness, current) {
            (Some(cached), Some(now)) => cached == now,
            _ => true,
        };
        if !still_fresh {
            return None;
        }
        // CONC-2 / TASK-1023: bump the LRU tick on hit so frequently accessed
        // entries survive eviction in a daemon visiting many roots.
        //
        // PERF-1 / TASK-1240: push the new tick onto the victim heap; the older
        // `(prev_tick, root)` pair stays in the heap and is discarded as stale
        // during eviction.
        //
        // PERF-3 / TASK-1572: the queue holds the `Arc<PathBuf>` already shared
        // with the entry, so the per-hit refresh is an `Arc::clone` (atomic
        // bump) instead of a `PathBuf` clone.
        //
        // PERF-16 / TASK-1723: route the push through `record_access`, which
        // compacts the queue on a growth threshold. Pushing directly here
        // leaked one stamp per hit for every process that never reached the
        // cap — which is every CLI run.
        let tick = next_lru_tick();
        entry.last_accessed = tick;
        let key = Arc::clone(&entry.key);
        let loaded = entry.loaded.clone();
        self.record_access(key, tick);
        Some(loaded)
    }

    /// FN-1 / TASK-1780: the cap check, LRU eviction, tick mint, victim-queue
    /// push and map insert, as one named operation.
    fn insert(
        &mut self,
        root: &Path,
        freshness: Option<ManifestFreshness>,
        loaded: &LoadedManifest,
    ) {
        // CONC-2 / TASK-0843 + TASK-1023: bound the cache with LRU
        // eviction. When the soft cap is hit and the key is new, evict
        // the entry with the smallest `last_accessed` tick so the hot
        // working-set survives a daemon visiting many roots. The previous
        // `keys().next()` policy picked an arbitrary HashMap bucket and
        // could evict the daemon's own workspace.
        // PERF-1 / TASK-1240: O(log n) eviction via the lazy-invalidation
        // min-heap, replacing the previous O(n) `min_by_key` scan.
        if !self.map.contains_key(root) && self.map.len() >= MAX_TYPED_MANIFEST_CACHE_ENTRIES {
            let _ = self.evict_lru();
        }
        // PERF-3 / TASK-1572: wrap the owned root once in `Arc<PathBuf>` so the
        // entry and the victim-queue push reference the same heap allocation
        // (the map needs its own owned `PathBuf` slot).
        let key_arc: Arc<PathBuf> = Arc::new(root.to_path_buf());
        let tick = next_lru_tick();
        self.map.insert(
            root.to_path_buf(),
            TypedManifestEntry {
                freshness,
                loaded: loaded.clone(),
                last_accessed: tick,
                key: Arc::clone(&key_arc),
            },
        );
        // PERF-16 / TASK-1723: stamp after the map update — `record_access`
        // validates stamps against `map[key].last_accessed`, so compacting
        // before the entry exists would discard the stamp just pushed.
        self.record_access(key_arc, tick);
    }
}

/// Stat `<workspace_root>/Cargo.toml` for the mtime+len freshness key.
pub fn cargo_toml_freshness(workspace_root: &Path) -> Option<ManifestFreshness> {
    let meta = std::fs::metadata(workspace_root.join("Cargo.toml")).ok()?;
    Some(ManifestFreshness {
        mtime: meta.modified().ok()?,
        len: meta.len(),
    })
}

fn typed_manifest_cache() -> &'static Mutex<TypedManifestCache> {
    static CACHE: OnceLock<Mutex<TypedManifestCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(TypedManifestCache::new()))
}

/// ERR-1 / TASK-0844: acquire the typed-manifest cache lock, recovering
/// from a `PoisonError` rather than silently falling through. A poisoned
/// mutex (caused by a panic in another provider while it held the lock)
/// would otherwise degrade the cache to "always-miss" with zero diagnostic
/// — exactly the invisibility class CONC-2 / TASK-0795 fought against in
/// the previous `thread_local` refactor.
///
/// The cache value type is plain data (`SystemTime` + `LoadedManifest`,
/// which itself is just `Arc`s); a panic in a sibling provider cannot leave
/// it in a torn state, so `into_inner()` recovery is safe.
///
/// TASK-0962: every observed poisoning emits a warn carrying a monotonic
/// `recovery_count`. The previous OnceLock-gated log fired only once per
/// process, so a second panic in a different provider was invisible —
/// defeating the "schema drift surfaces" intent. After clearing the sticky
/// poison flag, `clear_poison()` makes subsequent callers see a healthy
/// mutex; only an *actual* re-poisoning by a fresh panic increments the
/// counter.
fn lock_typed_manifest_cache(
    cache: &'static Mutex<TypedManifestCache>,
) -> std::sync::MutexGuard<'static, TypedManifestCache> {
    static POISON_RECOVERY_COUNT: AtomicU64 = AtomicU64::new(0);
    match cache.lock() {
        Ok(g) => g,
        Err(poison) => {
            // `saturating_add` is exact here: the counter advances once per
            // observed poisoning, so reaching `u64::MAX` would take 2^64
            // panics inside a single process.
            let recovery_count = POISON_RECOVERY_COUNT
                .fetch_add(1, Ordering::Relaxed)
                .saturating_add(1);
            tracing::warn!(
                recovery_count,
                "typed_manifest_cache mutex was poisoned by a panic in another provider; \
                 recovering via PoisonError::into_inner — cached entries are plain data \
                 and not torn by the panic"
            );
            let guard = poison.into_inner();
            // Clear the sticky poison flag so subsequent callers don't
            // re-enter the recovery path on every call after a single
            // panic. A fresh panic in another provider re-poisons the
            // mutex and increments `recovery_count` again.
            cache.clear_poison();
            guard
        }
    }
}

/// Drop the cached entry for `root` (`ctx.refresh` semantics).
///
/// One lock scope, no IO: the CONC-7 contract is checkable at a glance.
pub fn evict(root: &Path) {
    let mut guard = lock_typed_manifest_cache(typed_manifest_cache());
    guard.map.remove(root);
}

/// Probe the cache for a fresh entry under `root`.
///
/// One lock scope, no IO (the freshness stat is taken by the caller before the
/// lock is acquired).
pub fn probe(root: &Path, current: Option<ManifestFreshness>) -> Option<LoadedManifest> {
    let mut guard = lock_typed_manifest_cache(typed_manifest_cache());
    guard.probe(root, current)
}

/// Insert (or replace) the entry for `root`, evicting the LRU entry if the cap
/// is reached.
///
/// One lock scope, no IO or parsing.
pub fn insert(root: &Path, freshness: Option<ManifestFreshness>, loaded: &LoadedManifest) {
    let mut guard = lock_typed_manifest_cache(typed_manifest_cache());
    guard.insert(root, freshness, loaded);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::load_workspace_manifest;
    use ops_extension::Context;

    /// The cache is keyed by the *canonical* workspace root, so tests that
    /// poke at the map directly must canonicalise the tempdir path first
    /// (`/tmp` is a symlink on macOS).
    fn canonical(path: &Path) -> PathBuf {
        std::fs::canonicalize(path).expect("canonicalize tempdir")
    }

    fn cache_len() -> usize {
        lock_typed_manifest_cache(typed_manifest_cache()).map.len()
    }

    fn clear_cache() {
        let mut guard = lock_typed_manifest_cache(typed_manifest_cache());
        guard.map.clear();
        guard.victim_queue.clear();
    }

    fn contains(root: &Path) -> bool {
        lock_typed_manifest_cache(typed_manifest_cache())
            .map
            .contains_key(root)
    }

    fn victim_queue_len() -> usize {
        lock_typed_manifest_cache(typed_manifest_cache())
            .victim_queue
            .len()
    }

    /// PERF-16 / TASK-1723: `probe` stamps a fresh `(tick, root)` pair on
    /// every cache hit, but the only drain (`evict_lru`) runs solely once the
    /// map reaches `MAX_TYPED_MANIFEST_CACHE_ENTRIES`. A process below the
    /// cap — every `ops about` run, which touches one root from several
    /// providers — therefore grew the victim queue by one stamp per probe and
    /// never dropped one. Hammer a single root and pin that the queue stays
    /// proportional to the live entry count, not to the probe count.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn repeated_hits_below_cap_keep_the_victim_queue_bounded() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname=\"x\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        clear_cache();

        const PROBES: usize = 500;
        let mut ctx = Context::test_context(dir.path().to_path_buf());
        for _ in 0..PROBES {
            let _ = load_workspace_manifest(&mut ctx).expect("load");
        }

        let map_len = cache_len();
        let queue_len = victim_queue_len();
        assert_eq!(map_len, 1, "one root, so one live entry");
        assert!(
            map_len < MAX_TYPED_MANIFEST_CACHE_ENTRIES,
            "the test must stay below the cap or it stops covering the leak"
        );
        // Without compaction this is PROBES == 500 stamps.
        let bound = map_len.saturating_mul(2).saturating_add(VICTIM_QUEUE_SLACK);
        assert!(
            queue_len <= bound,
            "victim queue holds {queue_len} stamps after {PROBES} probes of {map_len} root(s); \
             expected at most {bound} — it is growing with the probe count again"
        );

        clear_cache();
    }

    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn typed_manifest_cache_returns_same_arc_then_invalidates_on_refresh() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname=\"x\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        evict(&canonical(dir.path()));

        let mut ctx = Context::test_context(dir.path().to_path_buf());
        let first = load_workspace_manifest(&mut ctx).expect("load1");
        let second = load_workspace_manifest(&mut ctx).expect("load2");
        assert!(
            first.shares_manifest_with(&second),
            "second call must reuse cached Arc"
        );

        let mut ctx = ctx.with_refresh();
        let third = load_workspace_manifest(&mut ctx).expect("load3");
        assert!(
            !first.shares_manifest_with(&third),
            "refresh=true must invalidate cache and reparse"
        );

        evict(&canonical(dir.path()));
    }

    /// ARCH-2 (TASK-0795): the cache must be visible to callers on other
    /// threads. The previous `thread_local!` keyed each entry to the
    /// inserting thread, so a parallel-provider refactor would have
    /// silently re-parsed the manifest per worker. Drive a load on one
    /// thread, then assert a sibling `Context` on another thread sees the
    /// same `Arc` allocation.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn typed_manifest_cache_is_shared_across_threads() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname=\"x\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        evict(&canonical(dir.path()));

        let path = dir.path().to_path_buf();
        let path_for_thread = path.clone();
        let primer = std::thread::spawn(move || {
            let mut ctx = Context::test_context(path_for_thread);
            load_workspace_manifest(&mut ctx).expect("primer load")
        });
        let first = primer.join().expect("primer thread");

        let path_for_reader = path.clone();
        let reader = std::thread::spawn(move || {
            let mut ctx = Context::test_context(path_for_reader);
            load_workspace_manifest(&mut ctx).expect("reader load")
        });
        let second = reader.join().expect("reader thread");

        assert!(
            first.shares_manifest_with(&second),
            "cross-thread callers must share the cached Arc"
        );

        evict(&canonical(&path));
    }

    /// CL-3 / TASK-1762 AC #3: the cache is keyed by the resolved workspace
    /// root, so a load driven from a subdirectory shares the root's entry
    /// (one slot, one freshness key) rather than minting a second one.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn cache_is_keyed_by_workspace_root_not_cwd() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::create_dir_all(root.join("crates/foo/src")).unwrap();
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
        clear_cache();

        let mut root_ctx = Context::test_context(root.to_path_buf());
        let from_root = load_workspace_manifest(&mut root_ctx).expect("load from root");

        let mut sub_ctx = Context::test_context(root.join("crates/foo/src"));
        let from_sub = load_workspace_manifest(&mut sub_ctx).expect("load from subdirectory");

        assert!(
            from_root.shares_manifest_with(&from_sub),
            "a subdirectory cwd must hit the root's cache entry"
        );
        assert_eq!(
            cache_len(),
            1,
            "two cwds inside one workspace must occupy a single cache slot"
        );
        assert!(contains(&canonical(root)), "the key must be the root");

        clear_cache();
    }

    /// TEST-12 / TASK-1162: shared body for the poison-recovery tests below.
    /// `n_cycles` poison-and-recover iterations: each cycle panics inside a
    /// held lock on a sibling thread, then drives `load_workspace_manifest`
    /// to observe the recovery warn. Returns the captured WARN-level logs
    /// emitted during the FINAL recovery so each caller can assert against
    /// its own contract (one-cycle vs. second-cycle wording).
    fn assert_poison_warn_after(n_cycles: usize, manifest_name: &str) -> String {
        assert!(n_cycles >= 1, "at least one poison cycle required");
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("Cargo.toml"),
            format!("[package]\nname=\"{manifest_name}\"\nversion=\"0.1.0\"\n"),
        )
        .unwrap();
        evict(&canonical(dir.path()));

        // Drive the first n-1 poison-recover cycles untraced.
        // `n_cycles >= 1` is asserted above, so `saturating_sub(1)` equals
        // `- 1` exactly.
        for _ in 0..n_cycles.saturating_sub(1) {
            let _ = std::thread::spawn(|| {
                let _g = typed_manifest_cache().lock().unwrap();
                panic!("poison cycle (warmup)");
            })
            .join();
            let mut ctx = Context::test_context(dir.path().to_path_buf());
            let _ = load_workspace_manifest(&mut ctx).expect("warmup recovery");
        }

        // Poison once more and capture this cycle's warn.
        let cache = typed_manifest_cache();
        let _ = std::thread::spawn(|| {
            let _g = typed_manifest_cache().lock().unwrap();
            panic!("intentional poison for test");
        })
        .join();
        assert!(
            cache.lock().is_err(),
            "mutex must be poisoned for the test premise"
        );

        let buf = ops_about::test_support::TracingBuf::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf.clone())
            .with_max_level(tracing::Level::WARN)
            .with_ansi(false)
            .finish();

        let mut ctx = Context::test_context(dir.path().to_path_buf());
        let result =
            tracing::subscriber::with_default(subscriber, || load_workspace_manifest(&mut ctx));
        assert!(result.is_ok(), "poisoned cache must recover, not propagate");

        // After recovery the cache itself is no longer poisoned-blocking
        // (clear_poison + into_inner cleared the sticky flag).
        assert!(
            cache.lock().is_ok(),
            "cache must be unpoisoned after into_inner recovery"
        );

        let logs = buf.captured();
        evict(&canonical(dir.path()));
        logs
    }

    /// ERR-1 / TASK-0844: a poisoned cache mutex (caused by a panic in
    /// another provider while it held the lock) must NOT silently degrade
    /// the cache to "always-miss". `load_workspace_manifest` must recover via
    /// `PoisonError::into_inner` and emit a warn so operators see the signal
    /// instead of paying a silent perf cliff.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn typed_manifest_cache_recovers_from_poison_with_warn() {
        // TASK-0962: poison-recovery now logs every cycle (with a monotonic
        // recovery_count) instead of one-shot via OnceLock, so the warn is
        // always observable here regardless of sibling-test ordering.
        let logs = assert_poison_warn_after(1, "poisoned");
        assert!(
            logs.contains("poisoned"),
            "poison warn must mention poisoning, got: {logs}"
        );
        assert!(
            logs.contains("recovery_count"),
            "poison warn must include monotonic recovery_count, got: {logs}"
        );
    }

    /// TASK-0962: a *second* poison cycle (panic in a different provider
    /// after the first recovery) must still produce an observable signal.
    /// The previous OnceLock-gated warn fired only on the first poisoning,
    /// silently swallowing every subsequent one.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn typed_manifest_cache_second_poison_still_logs() {
        let logs = assert_poison_warn_after(2, "poisoned-twice");
        assert!(
            logs.contains("poisoned") && logs.contains("recovery_count"),
            "second poison cycle must still emit a warn with recovery_count, got: {logs}"
        );
    }

    /// CONC-2 / TASK-1198: two writes inside the same mtime tick (HFS+,
    /// FAT, NFS with old `actimeo` all expose second-resolution mtime)
    /// must NOT serve the pre-edit manifest. The freshness key now
    /// includes the file byte length, so a write that changes the
    /// content size invalidates the cache even when mtime is unchanged.
    ///
    /// Direct-cache simulation: manually pin the cached entry's
    /// `freshness` to the *post-write* length-equal mtime to mimic a
    /// same-second second write that the second-resolution filesystem
    /// would silently keep stale. Then write a new manifest with a
    /// different byte length and assert the next load reparses.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn typed_manifest_cache_invalidates_on_size_change_within_same_mtime_tick() {
        let dir = tempfile::tempdir().expect("tempdir");
        let manifest_path = dir.path().join("Cargo.toml");
        // Initial body (small).
        std::fs::write(&manifest_path, "[package]\nname=\"x\"\nversion=\"0.1.0\"\n").unwrap();
        let root = canonical(dir.path());
        evict(&root);

        let mut ctx = Context::test_context(dir.path().to_path_buf());
        let first = load_workspace_manifest(&mut ctx).expect("load1");

        // Capture the just-cached freshness so we can simulate a
        // same-tick second write: rewrite the file with a *different*
        // byte length, then patch the cached entry's freshness.mtime to
        // match what we expect the new write to produce. The test fails
        // if the cache pre-fix relies on mtime alone — len differs, so
        // the freshness comparison must reject the cached entry.
        let new_body = "[package]\nname=\"x\"\nversion=\"0.1.0\"\n# trailing comment that bumps len significantly so the freshness comparison can detect the change\n";
        std::fs::write(&manifest_path, new_body).unwrap();
        let new_meta = std::fs::metadata(&manifest_path).unwrap();
        let new_mtime = new_meta.modified().unwrap();

        // Splice the cached entry's freshness so the mtime matches
        // post-write but the len stays at the *pre-write* value (the
        // same-second-tick illusion). Without the size component, the
        // cache would happily serve `first` again.
        {
            let mut guard = lock_typed_manifest_cache(typed_manifest_cache());
            let entry = guard.map.get_mut(&root).expect("entry");
            let pre_len = entry
                .freshness
                .as_ref()
                .map(|f| f.len)
                .expect("freshness present");
            entry.freshness = Some(ManifestFreshness {
                mtime: new_mtime,
                len: pre_len,
            });
            drop(guard);
        }

        let second = load_workspace_manifest(&mut ctx).expect("load2");
        assert!(
            !first.shares_manifest_with(&second),
            "size change inside the same mtime tick must invalidate the cache"
        );

        evict(&root);
    }

    /// CONC-2 / TASK-0843: a stale `Cargo.toml` mtime must invalidate the
    /// cached entry without requiring `ctx.refresh = true`.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn typed_manifest_cache_invalidates_on_mtime_change() {
        let dir = tempfile::tempdir().expect("tempdir");
        let manifest_path = dir.path().join("Cargo.toml");
        std::fs::write(&manifest_path, "[package]\nname=\"x\"\nversion=\"0.1.0\"\n").unwrap();
        evict(&canonical(dir.path()));

        let mut ctx = Context::test_context(dir.path().to_path_buf());
        let first = load_workspace_manifest(&mut ctx).expect("load1");

        // TEST-15 / TASK-1159: bump mtime explicitly via
        // `File::set_modified` instead of sleeping for >1s to outrun
        // second-resolution filesystems (HFS+, ext3). Wall-clock sleep
        // adds ~1.1s to every CI run and is unreliable on filesystems
        // with even-coarser resolution (NFS); an explicit timestamp two
        // seconds in the future is deterministic in microseconds.
        std::fs::write(&manifest_path, "[package]\nname=\"y\"\nversion=\"0.2.0\"\n").unwrap();
        // `checked_add` cannot fail for `now + 2s` on any real clock. The
        // `UNIX_EPOCH` fallback still yields an mtime that differs from the
        // one the cache captured, which is all the assertion below needs.
        let bumped = std::time::SystemTime::now()
            .checked_add(std::time::Duration::from_secs(2))
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        std::fs::OpenOptions::new()
            .write(true)
            .open(&manifest_path)
            .expect("open for set_modified")
            .set_modified(bumped)
            .expect("set mtime");

        let second = load_workspace_manifest(&mut ctx).expect("load2");
        assert!(
            !first.shares_manifest_with(&second),
            "mtime change must invalidate cache and reparse"
        );

        evict(&canonical(dir.path()));
    }

    /// CONC-2 / TASK-0843: cache size is soft-capped so a long-running
    /// process visiting many roots never accumulates more than
    /// `MAX_TYPED_MANIFEST_CACHE_ENTRIES` entries.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn typed_manifest_cache_is_bounded() {
        let dir = tempfile::tempdir().expect("tempdir");
        clear_cache();
        for i in 0..(MAX_TYPED_MANIFEST_CACHE_ENTRIES + 10) {
            let cwd = dir.path().join(format!("ws-{i}"));
            std::fs::create_dir_all(&cwd).unwrap();
            std::fs::write(
                cwd.join("Cargo.toml"),
                format!("[package]\nname=\"w{i}\"\nversion=\"0.1.0\"\n"),
            )
            .unwrap();
            let mut ctx = Context::test_context(cwd);
            let _ = load_workspace_manifest(&mut ctx).expect("load");
        }
        let len = cache_len();
        assert!(
            len <= MAX_TYPED_MANIFEST_CACHE_ENTRIES,
            "cache size {len} must stay within MAX_TYPED_MANIFEST_CACHE_ENTRIES = {MAX_TYPED_MANIFEST_CACHE_ENTRIES}"
        );
        // Cleanup so we don't pollute later tests in the same process.
        clear_cache();
    }

    /// CONC-2 / TASK-1023: eviction must pick the least-recently-used entry,
    /// not an arbitrary `HashMap` bucket. Insert MAX entries, touch the very
    /// first one to make it "hot", then insert one more to force eviction
    /// and assert (a) the hot key is still present, and (b) the victim is
    /// the actual coldest key — not the hot one and not the new one.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn typed_manifest_cache_evicts_lru_not_hot_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        clear_cache();

        // Fill the cache to MAX with a deterministic insertion order.
        let mut keys = Vec::with_capacity(MAX_TYPED_MANIFEST_CACHE_ENTRIES);
        for i in 0..MAX_TYPED_MANIFEST_CACHE_ENTRIES {
            let cwd = dir.path().join(format!("ws-{i:03}"));
            std::fs::create_dir_all(&cwd).unwrap();
            std::fs::write(
                cwd.join("Cargo.toml"),
                format!("[package]\nname=\"w{i}\"\nversion=\"0.1.0\"\n"),
            )
            .unwrap();
            let mut ctx = Context::test_context(cwd.clone());
            let _ = load_workspace_manifest(&mut ctx).expect("load");
            keys.push(canonical(&cwd));
        }
        assert_eq!(cache_len(), MAX_TYPED_MANIFEST_CACHE_ENTRIES);

        // Touch the FIRST inserted key to mark it "hot" (highest LRU tick).
        // Under the buggy `HashMap::keys().next()` policy this hot key was
        // a plausible eviction victim because hash-bucket order has no
        // recency signal; under LRU it is now the most-recent.
        let hot = keys[0].clone();
        let mut hot_ctx = Context::test_context(hot.clone());
        let _ = load_workspace_manifest(&mut hot_ctx).expect("hot reload");

        // The coldest key is now keys[1] — it was inserted second-earliest
        // and never touched again.
        let coldest = keys[1].clone();

        // Force eviction by inserting one fresh key.
        let fresh = dir.path().join("ws-fresh");
        std::fs::create_dir_all(&fresh).unwrap();
        std::fs::write(
            fresh.join("Cargo.toml"),
            "[package]\nname=\"fresh\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        let mut fresh_ctx = Context::test_context(fresh.clone());
        let _ = load_workspace_manifest(&mut fresh_ctx).expect("fresh load");

        assert_eq!(
            cache_len(),
            MAX_TYPED_MANIFEST_CACHE_ENTRIES,
            "cap must hold after eviction"
        );
        assert!(contains(&hot), "hot key must survive LRU eviction");
        assert!(
            contains(&canonical(&fresh)),
            "newly inserted key must remain"
        );
        assert!(
            !contains(&coldest),
            "victim must be the coldest key (LRU), got cache keys: {:?}",
            lock_typed_manifest_cache(typed_manifest_cache())
                .map
                .keys()
                .collect::<Vec<_>>()
        );

        clear_cache();
    }
}
