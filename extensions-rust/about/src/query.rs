//! Shared helpers for Rust-specific data providers.
//!
//! ## Why not `MetadataProvider`?
//!
//! `cargo metadata` is the canonical source for resolved workspace members,
//! but invoking it requires running `cargo` (slow, network-touching, and
//! requires a fully-resolvable lockfile). The about/identity/units/coverage
//! providers run on every `ops about` invocation and need to be cheap and
//! offline-tolerant. They therefore parse `Cargo.toml` directly and resolve
//! workspace globs with the static expander below. `MetadataProvider` is used
//! by `deps_provider` where dependency graph data is unavoidable.

use ops_about::lru::{next_lru_tick, LruVictimQueue};
use ops_cargo_toml::{
    find_workspace_root_strict, CargoToml, CargoTomlProvider, FindWorkspaceRootError,
};
use ops_extension::{Context, DataProviderError};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

/// CONC-2 / TASK-0843: soft upper bound on the typed-manifest cache so a
/// long-running daemon process that visits an unbounded set of workspaces
/// (CI worker, language-server-style host) does not accumulate one entry
/// per cwd indefinitely. When the cap is hit on insert we evict the
/// least-recently-used entry (LRU) so steady-state hits remain warm.
const MAX_TYPED_MANIFEST_CACHE_ENTRIES: usize = 64;

/// CONC-2 / TASK-0843: cache entry pairs the parsed manifest with the
/// `Cargo.toml` mtime captured at parse time. On every cache lookup we
/// re-stat the file; if the on-disk mtime differs (or the file disappeared
/// or has become unreadable) the entry is treated as stale and the
/// manifest is reparsed. `None` mtime means we couldn't stat the file at
/// parse time — in that case we keep the legacy "always trust the cache
/// until ctx.refresh" behaviour rather than thrashing on every call.
///
/// CONC-2 / TASK-1023: `last_accessed` is the LRU tick stamped at insert
/// and refreshed on every cache hit. Eviction picks the entry with the
/// smallest tick (least-recently-used).
/// CONC-2 / TASK-1198: cache freshness key. Pairs the file's mtime with
/// its byte length so two writes within the same mtime tick (HFS+, FAT,
/// NFS with old `actimeo` all expose second-resolution mtime) cannot
/// silently keep serving the pre-edit manifest. Length-equal collisions
/// inside one second remain possible but require both the same byte
/// length AND identical mtime — far less likely than mtime alone.
#[derive(Clone, Copy, PartialEq, Eq)]
struct ManifestFreshness {
    mtime: SystemTime,
    len: u64,
}

struct TypedManifestEntry {
    /// `None` means we couldn't stat the file at parse time; the legacy
    /// "always trust the cache until ctx.refresh" behaviour applies.
    freshness: Option<ManifestFreshness>,
    loaded: LoadedManifest,
    last_accessed: u64,
}

/// PERF-1 / TASK-1240: pair the cwd→entry map with a min-heap of
/// `(last_accessed, cwd)` so cap-bound eviction picks the LRU entry in
/// `O(log n)` (heap pop with lazy invalidation) instead of an `O(n)`
/// scan. Kept in lockstep with `manifest_cache::CacheMap` per the
/// module-level lockstep contract.
struct TypedManifestCache {
    map: HashMap<PathBuf, TypedManifestEntry>,
    victim_queue: LruVictimQueue<PathBuf>,
}

impl TypedManifestCache {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            victim_queue: LruVictimQueue::new(),
        }
    }

    fn evict_lru(&mut self) -> Option<PathBuf> {
        let map = &mut self.map;
        let victim = self
            .victim_queue
            .pop_lru(|path, tick| map.get(path).is_some_and(|e| e.last_accessed == tick))?;
        map.remove(&victim);
        Some(victim)
    }
}

/// ERR-1 / TASK-1076: pairs the cached parsed manifest with its resolved
/// `[workspace].members` list so the original glob spec on the cached
/// `CargoToml` is preserved verbatim.
///
/// Before TASK-1076 `load_workspace_manifest` overwrote
/// `manifest.workspace.members` with the resolved list before caching the
/// `Arc<CargoToml>`. That mutation lost the literal `["crates/*"]` spec for
/// every subsequent consumer (a future linter or doc generator wanting the
/// raw spec would see only the expanded list), and any code re-running glob
/// expansion on the cached manifest no-op'd because the list was already
/// flattened. Storing the resolved view in a sibling field keeps `ws.members`
/// immutable post-parse while preserving the PERF-3 / TASK-0969 contract that
/// resolved members survive across calls without re-walking the filesystem.
#[derive(Clone)]
pub(crate) struct LoadedManifest {
    pub(crate) manifest: Arc<CargoToml>,
    pub(crate) resolved_members: Arc<Vec<String>>,
}

impl LoadedManifest {
    /// Resolved workspace members (post glob expansion, deduped, sorted).
    /// Returns an empty slice when the manifest has no `[workspace]` table.
    ///
    /// # Ordering invariant
    ///
    /// PERF-3 / TASK-1251: this slice is produced by
    /// [`resolved_workspace_members`] which sorts (TASK-0794) and dedups
    /// (TASK-1042) the result before returning. Consumers (about, identity,
    /// units, coverage providers) MUST consume this view directly and MUST
    /// NOT re-sort it — re-sorting allocates a fresh `Vec<&str>` on every
    /// call and adds no semantic value.
    pub(crate) fn resolved_members(&self) -> &[String] {
        self.resolved_members.as_slice()
    }
}

impl std::ops::Deref for LoadedManifest {
    type Target = CargoToml;
    fn deref(&self) -> &CargoToml {
        &self.manifest
    }
}

fn cargo_toml_freshness(workspace_root: &Path) -> Option<ManifestFreshness> {
    let meta = std::fs::metadata(workspace_root.join("Cargo.toml")).ok()?;
    Some(ManifestFreshness {
        mtime: meta.modified().ok()?,
        len: meta.len(),
    })
}

/// Log a `load_workspace_manifest` failure differentiating "no manifest /
/// not a Rust project" (silent debug) from a real read/parse error (warn),
/// mirroring `read_crate_metadata` (TASK-0433).
pub(crate) fn log_manifest_load_failure(err: &DataProviderError) {
    if is_manifest_missing(err) {
        tracing::debug!("Cargo.toml not found; Rust providers will produce empty results: {err:#}");
    } else {
        tracing::warn!("failed to load workspace Cargo.toml: {err:#}");
    }
}

fn is_manifest_missing(err: &(dyn std::error::Error + 'static)) -> bool {
    // ARCH-2 / TASK-0871: prefer the typed `FindWorkspaceRootError::NotFound`
    // marker so wrapping context layers added by future callers don't silently
    // mask the "missing manifest" signal. The legacy `io::ErrorKind::NotFound`
    // chain-walk is retained as a fallback for IO errors raised outside the
    // workspace-root walk (e.g. direct `read_to_string` failures).
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(err);
    while let Some(e) = current {
        if let Some(typed) = e.downcast_ref::<FindWorkspaceRootError>() {
            return typed.is_not_found();
        }
        if let Some(io) = e.downcast_ref::<std::io::Error>() {
            return io.kind() == std::io::ErrorKind::NotFound;
        }
        current = e.source();
    }
    false
}

// PERF-1 / TASK-0558: identity, units, and coverage providers each call
// `load_workspace_manifest` during a single `ops about` invocation. The
// previous implementation cloned the cached `serde_json::Value` and
// re-deserialized it every time, even though the resolved manifest is
// identical across providers.
//
// ARCH-2 (TASK-0795): the cache lives in a process-global `Mutex<HashMap>`
// keyed by working directory rather than a `thread_local!`. The previous
// thread-local was invisible to providers scheduled on a different worker
// thread (e.g. a future tokio fan-out), silently degrading the cache to
// "off" with no signal. The mutex is held only for the lookup / insert and
// never across provider work, so contention is bounded; readers clone the
// `Arc<CargoToml>` so the typed manifest is shared across threads with no
// reparse. `ctx.refresh` evicts the entry to preserve `--refresh` semantics.
//
// ERR-1 / TASK-0844: the cache lock is acquired exclusively through
// [`lock_typed_manifest_cache`], which recovers from `PoisonError` via
// `into_inner` + `clear_poison` and warns once per process. Without this,
// a panic in a sibling provider would silently degrade the cache to
// "always-miss" — the same invisibility class the thread-local rewrite
// fought against, just routed through a different mechanism.
//
// CONC-7 / TASK-1163: concurrency contract.
//
// The wrapper is a single `Mutex<HashMap>`, which serialises every probe.
// This is intentional and bounded by the workload it guards:
//
// - **Single-shot CLI (`ops about`):** every provider runs in turn from one
//   thread, so the lock is uncontended. The hot path is HashMap probe + LRU
//   tick under the lock and never holds the lock across IO or parsing.
// - **Daemon / language-server hosts:** when multiple worker threads start
//   running providers in parallel against many distinct workspace roots,
//   this single-mutex shape becomes the bottleneck (CONC-7 forbids
//   `Mutex<Collection>` on hot paths exactly because of this). At that
//   point the cache MUST migrate to `DashMap<PathBuf, TypedManifestEntry>`
//   plus a separate small `parking_lot::Mutex<()>` only for the LRU
//   eviction scan — sharded reads, occasional global serialisation just
//   for the cap-evict step. Keep this comment in sync with
//   `extensions/about/src/manifest_cache.rs` (TASK-1144 already moved that
//   sibling cache to a per-key `OnceLock` shape so distinct paths progress
//   in parallel; the typed-manifest cache here intentionally lags because
//   no daemon caller exists yet).
//
// Reviewer rule: do not add a daemon caller without first making the
// migration above. A new caller that opens parallel `ctx`s against
// distinct cwds and bottlenecks here would silently undo a downstream
// performance fix.
fn typed_manifest_cache() -> &'static Mutex<TypedManifestCache> {
    static CACHE: OnceLock<Mutex<TypedManifestCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(TypedManifestCache::new()))
}

/// ERR-1 / TASK-0844: acquire the typed-manifest cache lock, recovering
/// from a `PoisonError` rather than silently falling through. A poisoned
/// mutex (caused by a panic in another provider while it held the lock)
/// would otherwise degrade the cache to "always-miss" with zero diagnostic
/// — exactly the invisibility class CONC-2 / TASK-0795 fought against in
/// the previous thread_local refactor.
///
/// The cache value type is plain data (`SystemTime` + `LoadedManifest`,
/// which itself is just `Arc<CargoToml>` + `Arc<Vec<String>>`); a panic in
/// a sibling provider cannot leave it in a torn state, so `into_inner()`
/// recovery is safe.
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
            let recovery_count = POISON_RECOVERY_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
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

/// Load and parse `Cargo.toml` for the current context, then resolve any
/// `[workspace].members` globs into a sibling field on [`LoadedManifest`].
/// Reuses any value already cached at the `cargo_toml` key; otherwise reads
/// via [`CargoTomlProvider`]. Centralises the parse + glob-resolve step that
/// identity / units / coverage providers all need (TASK-0381).
///
/// ERR-1 / TASK-1076: the parsed `manifest.workspace.members` is left
/// untouched so the original spec (e.g. `["crates/*"]`) is preserved on the
/// cached `Arc<CargoToml>`. Consumers that want the post-glob-expansion list
/// must read [`LoadedManifest::resolved_members`].
pub(crate) fn load_workspace_manifest(
    ctx: &mut Context,
) -> Result<LoadedManifest, DataProviderError> {
    let cwd: PathBuf = PathBuf::clone(&ctx.working_directory);
    let cache = typed_manifest_cache();

    let current_freshness = cargo_toml_freshness(&cwd);

    if ctx.refresh {
        let mut guard = lock_typed_manifest_cache(cache);
        guard.map.remove(&cwd);
    } else {
        let mut guard = lock_typed_manifest_cache(cache);
        if let Some(entry) = guard.map.get_mut(&cwd) {
            // CONC-2 / TASK-0843 + TASK-1198: serve the cached Arc only
            // when both the mtime AND the byte length match. Pairing
            // mtime with size closes the second-resolution-mtime window
            // (HFS+, FAT, NFS with old `actimeo`): two writes inside the
            // same second can produce identical mtimes, so mtime alone
            // happily served the pre-edit manifest until the next tick.
            // If we couldn't stat at all, fall back to the legacy "trust
            // until refresh" behaviour.
            let still_fresh = match (entry.freshness, current_freshness) {
                (Some(cached), Some(now)) => cached == now,
                _ => true,
            };
            if still_fresh {
                // CONC-2 / TASK-1023: bump LRU tick on hit so frequently
                // accessed entries survive eviction in a daemon visiting
                // many cwds.
                //
                // PERF-1 / TASK-1240: push the new tick onto the victim
                // heap; the older `(prev_tick, cwd)` pair stays in the
                // heap and is discarded as stale during eviction.
                let tick = next_lru_tick();
                entry.last_accessed = tick;
                let loaded = entry.loaded.clone();
                guard.victim_queue.push(tick, cwd.clone());
                return Ok(loaded);
            }
        }
    }

    // SEC-25 / TASK-1204: route through the strict workspace-root finder
    // so a hostile Cargo.toml planted at the symlink target of an ancestor
    // is rejected before it can redirect the about/units/coverage stack.
    // The lenient `find_workspace_root` walk reaches each ancestor via
    // `Path::parent` on the lexical canonical-start path and never
    // re-canonicalises at each step; the strict variant adds a
    // per-candidate canonicalize so a redirected ancestor surfaces a
    // tracing breadcrumb instead of becoming the discovered root. Falls
    // back to the lenient default when strict resolution fails so the
    // caller still receives a typed `FindWorkspaceRootError` that
    // `is_manifest_missing` understands; on success we hand the resolved
    // root to `CargoTomlProvider::with_root` so the inner provider does
    // not redo discovery.
    // PERF-1 / TASK-1195: cache-miss path goes through
    // `CargoTomlProvider::provide_typed` so the typed `CargoToml` arrives
    // here directly — no `serde_json::Value` round-trip. The pre-existing
    // ctx.cached fast path stays for cross-extension consumers that may
    // have populated the JSON cache via `Context::get_or_provide`; that
    // arm still pays one `serde_json::from_value`, but the dominant typed
    // cache miss no longer does.
    let manifest: CargoToml = if let Some(cached) = ctx.cached(ops_cargo_toml::DATA_PROVIDER_NAME) {
        // PERF-3 / TASK-1201: deserialize against a borrowed `&serde_json::Value`
        // instead of `(**cached).clone()` deep-cloning the entire tree before
        // `from_value` consumes it. The clone allocated one Box per nested
        // map/array node — multi-MB workspaces clone 10k+ allocations only
        // to drop them. `serde::Deserialize::deserialize` takes the value by
        // reference via its `IntoDeserializer` impl, so the cached Arc stays
        // shared and only the typed fields are produced.
        CargoToml::deserialize(cached.as_ref()).map_err(DataProviderError::computation_error)?
    } else {
        let provider = match find_workspace_root_strict(&cwd) {
            Ok(root) => CargoTomlProvider::with_root(root),
            Err(err) => {
                tracing::debug!(
                    cwd = ?cwd.display(),
                    error = ?err,
                    "TASK-1204: strict workspace-root resolution failed; surfacing typed error"
                );
                return Err(DataProviderError::from(anyhow::Error::from(err)));
            }
        };
        provider.provide_typed(ctx)?
    };

    // ERR-1 / TASK-1076: resolve workspace members into a sibling field
    // instead of mutating `manifest.workspace.members` in place. The previous
    // mutation flattened `["crates/*"]` to the expanded list on the cached
    // Arc, hiding the original glob spec from any future consumer (linter,
    // doc generator) and silently no-op'ing any re-expansion attempt.
    let resolved_members = Arc::new(resolved_workspace_members(&manifest, &cwd));

    let loaded = LoadedManifest {
        manifest: Arc::new(manifest),
        resolved_members,
    };
    {
        let mut guard = lock_typed_manifest_cache(cache);
        // CONC-2 / TASK-0843 + TASK-1023: bound the cache with LRU
        // eviction. When the soft cap is hit and the key is new, evict
        // the entry with the smallest `last_accessed` tick so the hot
        // working-set survives a daemon visiting many cwds. The previous
        // `keys().next()` policy picked an arbitrary HashMap bucket and
        // could evict the daemon's own workspace.
        // PERF-1 / TASK-1240: O(log n) eviction via the lazy-invalidation
        // min-heap, replacing the previous O(n) `min_by_key` scan.
        if !guard.map.contains_key(&cwd) && guard.map.len() >= MAX_TYPED_MANIFEST_CACHE_ENTRIES {
            let _ = guard.evict_lru();
        }
        let tick = next_lru_tick();
        guard.victim_queue.push(tick, cwd.clone());
        guard.map.insert(
            cwd,
            TypedManifestEntry {
                freshness: current_freshness,
                loaded: loaded.clone(),
                last_accessed: tick,
            },
        );
    }
    Ok(loaded)
}

/// Resolve `[workspace].members` globs to concrete member paths, honoring
/// `[workspace].exclude`. Members without a `*` are passed through verbatim.
///
/// Supports the simple `prefix/*` shape Cargo workspaces use in practice.
/// More elaborate patterns (`prefix/*/suffix`, `**`, `?`, character classes)
/// are not expanded — they are passed through unchanged and a `tracing::warn!`
/// is emitted so the unsupported shape is visible in logs rather than silently
/// producing a wrong member list.
pub(crate) fn resolved_workspace_members(
    manifest: &CargoToml,
    workspace_root: &Path,
) -> Vec<String> {
    let Some(ws) = manifest.workspace.as_ref() else {
        return Vec::new();
    };

    let exclude: std::collections::HashSet<&str> = ws.exclude.iter().map(String::as_str).collect();

    let mut resolved = Vec::new();
    for member in &ws.members {
        // SEC-14 / TASK-1246 AC #2: reject absolute and `..`-traversal
        // member entries before they reach any join.
        if !member_path_is_workspace_safe(member) {
            tracing::warn!(
                member = %member,
                "SEC-14 / TASK-1246: workspace member is absolute or contains `..`; dropping"
            );
            continue;
        }
        match classify_member(member) {
            MemberShape::Literal => resolved.push(member.clone()),
            MemberShape::Unsupported => {
                tracing::warn!(
                    pattern = %member,
                    "workspace member glob shape not supported by ops about; passing through unchanged"
                );
                resolved.push(member.clone());
            }
            MemberShape::Glob { prefix } => {
                let parent = workspace_root.join(prefix);
                resolved.extend(expand_member_glob(member, &parent, workspace_root));
            }
        }
    }

    resolved.retain(|m| !exclude.contains(m.as_str()));
    resolved.sort();
    // PATTERN-1 (TASK-1042): dedup the resolved member list. Cargo treats
    // `[workspace].members` with set semantics — overlapping entries like
    // `members = ["crates/foo", "crates/*"]` resolve to a single `crates/foo`
    // crate. Without this dedup the about pipeline would double-count a member
    // in `module_count` (identity provider) and emit duplicate `ProjectUnit`s
    // in the units / coverage providers, diverging from `cargo metadata`.
    resolved.dedup();
    resolved
}

/// FN-1 / TASK-1156: classified shape of one `[workspace].members` entry,
/// dispatched as a state machine in [`resolved_workspace_members`].
enum MemberShape<'a> {
    /// Pass through verbatim (no glob characters).
    Literal,
    /// Pass through verbatim and emit a warn — shape isn't supported.
    Unsupported,
    /// Expand via `read_dir(workspace_root.join(prefix))`.
    Glob { prefix: &'a str },
}

/// Classify a `[workspace].members` entry into a [`MemberShape`]. Centralises
/// the metacharacter scan so [`resolved_workspace_members`] reads as a flat
/// dispatch instead of a nested-if state machine.
fn classify_member(member: &str) -> MemberShape<'_> {
    let star_idx = member.find('*');
    // PATTERN-1 (TASK-0803): detect glob shapes that lack `*` but still
    // contain class/alternation metacharacters (`crates/{core,cli}`,
    // `crates/[abc]`).
    if star_idx.is_none() {
        if contains_unsupported_glob_meta(member) {
            return MemberShape::Unsupported;
        }
        return MemberShape::Literal;
    }
    let idx = star_idx.expect("star_idx checked above");
    if is_unsupported_glob(member, idx) {
        return MemberShape::Unsupported;
    }
    MemberShape::Glob {
        prefix: &member[..idx],
    }
}

/// Expand a `prefix/*` glob by walking `parent` and returning UTF-8
/// workspace-relative paths to each subdirectory containing a `Cargo.toml`.
/// FN-1 / TASK-1156: extracted from [`resolved_workspace_members`] so the
/// orchestrator stays at the dispatch level and the read_dir + per-entry
/// boundary handling sits in one place.
fn expand_member_glob(member: &str, parent: &Path, workspace_root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(e) => {
            // ERR-7 (TASK-0941): Debug-format pattern / parent / error so
            // embedded newlines / ANSI escapes in attacker-controlled
            // `[workspace].members` entries cannot forge log records.
            tracing::warn!(
                pattern = ?member,
                parent = ?parent.display(),
                error = ?e,
                "workspace glob prefix unreadable; member skipped"
            );
            return out;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(
                    parent = ?parent.display(),
                    error = ?e,
                    "workspace glob entry unreadable; skipped"
                );
                continue;
            }
        };
        let path = entry.path();
        if !(path.is_dir() && path.join("Cargo.toml").exists()) {
            continue;
        }
        let Ok(rel) = path.strip_prefix(workspace_root) else {
            continue;
        };
        // READ-5 (TASK-0946): non-UTF-8 member paths must not be lossily
        // collapsed to U+FFFD.
        match rel.to_str() {
            Some(s) => out.push(s.to_string()),
            None => {
                tracing::warn!(
                    parent = ?parent.display(),
                    relpath = ?rel,
                    "workspace glob member relpath is not valid UTF-8; skipping"
                );
            }
        }
    }
    out
}

/// Returns true if the glob shape goes beyond a single trailing `*` after
/// the prefix — anything we cannot expand correctly with the simple
/// `read_dir(prefix)` approach.
///
/// PATTERN-1 (TASK-0803): also flag character-class closers (`]`) and brace
/// alternation (`{`, `}`). A pattern like `crates/{core,cli}` lacks `*`,
/// `?`, and `[`, so without these checks it would slip through as
/// "supported" and silently produce an empty member list when `read_dir`
/// failed on the literal-as-directory path.
fn is_unsupported_glob(member: &str, first_star: usize) -> bool {
    let after_star = &member[first_star + 1..];
    if !after_star.is_empty() {
        return true;
    }
    contains_unsupported_glob_meta(member)
}

fn contains_unsupported_glob_meta(member: &str) -> bool {
    member.contains(['?', '[', ']', '{', '}'])
}

/// SEC-14 / TASK-1246: a workspace member must be a relative path with
/// no `..` segments. `Path::join` discards `cwd` when the operand is
/// absolute and walks parents on `..`, so a hostile root `Cargo.toml`
/// could otherwise drive `read_capped_to_string` and tracing
/// breadcrumbs at any filesystem location reachable from the workspace
/// root. Rejecting those shapes up front matches the
/// `append_tree_directory` (SEC-14 / TASK-0811) and `scrub_path_segments`
/// (SEC-14 / TASK-1111) policies on the rendering side.
pub(crate) fn member_path_is_workspace_safe(member: &str) -> bool {
    use std::path::Component;
    let p = Path::new(member);
    if p.is_absolute() {
        return false;
    }
    // Reject any segment equal to `..`. We accept `.` segments because
    // they are inert under `Path::join` and Cargo itself emits them in
    // some manifests (e.g. `members = ["./crates/foo"]`).
    !p.components().any(|c| matches!(c, Component::ParentDir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_extension::Context;

    /// ERR-7 (TASK-0941): tracing fields for workspace glob walk paths flow
    /// through the `?` formatter so embedded newlines / ANSI escapes in
    /// attacker-controlled `[workspace].members` cannot forge log records.
    #[test]
    fn workspace_glob_path_debug_escapes_control_characters() {
        let p = Path::new("a\nb\u{1b}[31mc/crates");
        let rendered = format!("{:?}", p.display());
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains('\u{1b}'));
        assert!(rendered.contains("\\n"));
    }

    /// PERF-1 / TASK-0558: load_workspace_manifest caches the typed
    /// CargoToml in a thread-local so identity / units / coverage do not
    /// each pay for a JSON clone + reparse. Verify (a) the second call in
    /// the same context returns an `Arc` that points to the same
    /// allocation as the first, and (b) `ctx.refresh = true` invalidates
    /// the cache and yields a freshly parsed allocation.
    fn evict_cache_for(path: &Path) {
        if let Ok(mut guard) = typed_manifest_cache().lock() {
            guard.map.remove(path);
        }
    }

    /// PERF-3 / TASK-0969: the resolved-members list (post glob expansion)
    /// must survive across `load_workspace_manifest` calls without
    /// re-walking the filesystem. ERR-1 / TASK-1076: the resolved view is
    /// now stored in a sibling field on `LoadedManifest` (the cached
    /// `Arc<CargoToml>` keeps the original glob spec verbatim), so
    /// subsequent providers grab the resolved members from the cached
    /// `LoadedManifest::resolved_members` snapshot — verified here by
    /// mutating a member directory between two cached loads and asserting
    /// the cached view does NOT pick up the change (proving no re-walk).
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn resolved_workspace_members_are_amortised_via_typed_manifest_cache() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n",
        )
        .unwrap();
        let crates = root.join("crates");
        std::fs::create_dir(&crates).unwrap();
        let foo = crates.join("foo");
        std::fs::create_dir(&foo).unwrap();
        std::fs::write(
            foo.join("Cargo.toml"),
            "[package]\nname=\"foo\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        evict_cache_for(root);

        let mut ctx = Context::test_context(root.to_path_buf());
        let first = load_workspace_manifest(&mut ctx).expect("first load");
        let resolved_first = first.resolved_members().to_vec();
        assert_eq!(resolved_first, vec!["crates/foo".to_string()]);

        // Add a sibling crate AFTER the first cache fill. If the second call
        // re-walked the filesystem the resolved list would now include
        // `crates/bar`; the cache must amortise this and keep returning the
        // same Arc with the same resolved members.
        let bar = crates.join("bar");
        std::fs::create_dir(&bar).unwrap();
        std::fs::write(
            bar.join("Cargo.toml"),
            "[package]\nname=\"bar\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();

        let second = load_workspace_manifest(&mut ctx).expect("second load");
        assert!(
            Arc::ptr_eq(&first.manifest, &second.manifest),
            "second call must serve the cached Arc, proving no re-walk"
        );
        let resolved_second = second.resolved_members().to_vec();
        assert_eq!(
            resolved_second, resolved_first,
            "resolved members must be the cached snapshot, not re-walked"
        );

        evict_cache_for(root);
    }

    /// ERR-1 / TASK-1076: `load_workspace_manifest` must NOT mutate the
    /// cached `manifest.workspace.members` to the resolved list. The
    /// original spec (e.g. `["crates/*"]`) must survive on the cached Arc
    /// across repeated calls so future consumers (linters, doc generators)
    /// that want the literal spec can read it. The expanded list is exposed
    /// separately via `LoadedManifest::resolved_members()`.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn cached_manifest_preserves_original_glob_spec_across_calls() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("crates/foo")).unwrap();
        std::fs::write(
            root.join("crates/foo/Cargo.toml"),
            "[package]\nname=\"foo\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        evict_cache_for(root);

        let mut ctx = Context::test_context(root.to_path_buf());
        let first = load_workspace_manifest(&mut ctx).expect("first load");
        let second = load_workspace_manifest(&mut ctx).expect("second load");

        // Same Arc — proves we are inspecting the cached manifest.
        assert!(
            Arc::ptr_eq(&first.manifest, &second.manifest),
            "second call must serve the cached Arc"
        );

        // The cached manifest's literal `[workspace].members` must still be
        // the glob spec, NOT the expanded `["crates/foo"]`. Before TASK-1076
        // this would have been the resolved list because the loader
        // overwrote `ws.members` in place before caching.
        let cached_spec = first
            .workspace
            .as_ref()
            .map(|w| w.members.clone())
            .unwrap_or_default();
        assert_eq!(
            cached_spec,
            vec!["crates/*".to_string()],
            "cached manifest must preserve the original glob spec, not the expanded list"
        );

        // Repeated calls yield consistent inputs: same glob spec on the
        // cached manifest AND the same resolved view.
        assert_eq!(
            first.resolved_members(),
            second.resolved_members(),
            "repeated calls must yield the same resolved members"
        );
        assert_eq!(
            first.resolved_members(),
            &["crates/foo".to_string()][..],
            "resolved view must reflect glob expansion"
        );

        evict_cache_for(root);
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
        evict_cache_for(dir.path());

        let mut ctx = Context::test_context(dir.path().to_path_buf());
        let first = load_workspace_manifest(&mut ctx).expect("load1");
        let second = load_workspace_manifest(&mut ctx).expect("load2");
        assert!(
            Arc::ptr_eq(&first.manifest, &second.manifest),
            "second call must reuse cached Arc"
        );

        ctx.refresh = true;
        let third = load_workspace_manifest(&mut ctx).expect("load3");
        assert!(
            !Arc::ptr_eq(&first.manifest, &third.manifest),
            "refresh=true must invalidate cache and reparse"
        );

        evict_cache_for(dir.path());
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
        evict_cache_for(dir.path());

        let path = dir.path().to_path_buf();
        let path_for_thread = path.clone();
        let primer = std::thread::spawn(move || {
            let mut ctx = Context::test_context(path_for_thread.clone());
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
            Arc::ptr_eq(&first.manifest, &second.manifest),
            "cross-thread callers must share the cached Arc"
        );

        evict_cache_for(&path);
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
        evict_cache_for(dir.path());

        // Drive the first n-1 poison-recover cycles untraced.
        for _ in 0..(n_cycles - 1) {
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
        evict_cache_for(dir.path());
        logs
    }

    /// ERR-1 / TASK-0844: a poisoned cache mutex (caused by a panic in
    /// another provider while it held the lock) must NOT silently degrade
    /// the cache to "always-miss". load_workspace_manifest must recover via
    /// PoisonError::into_inner and emit a warn so operators see the signal
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
        evict_cache_for(dir.path());

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
            let cache = typed_manifest_cache();
            let mut guard = cache.lock().unwrap();
            let entry = guard.map.get_mut(dir.path()).expect("entry");
            let pre_len = entry
                .freshness
                .as_ref()
                .map(|f| f.len)
                .expect("freshness present");
            entry.freshness = Some(ManifestFreshness {
                mtime: new_mtime,
                len: pre_len,
            });
        }

        let second = load_workspace_manifest(&mut ctx).expect("load2");
        assert!(
            !Arc::ptr_eq(&first.manifest, &second.manifest),
            "size change inside the same mtime tick must invalidate the cache"
        );

        evict_cache_for(dir.path());
    }

    /// CONC-2 / TASK-0843: a stale `Cargo.toml` mtime must invalidate the
    /// cached entry without requiring `ctx.refresh = true`.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn typed_manifest_cache_invalidates_on_mtime_change() {
        let dir = tempfile::tempdir().expect("tempdir");
        let manifest_path = dir.path().join("Cargo.toml");
        std::fs::write(&manifest_path, "[package]\nname=\"x\"\nversion=\"0.1.0\"\n").unwrap();
        evict_cache_for(dir.path());

        let mut ctx = Context::test_context(dir.path().to_path_buf());
        let first = load_workspace_manifest(&mut ctx).expect("load1");

        // Bump the mtime by rewriting (with a sleep buffer so filesystems
        // with second-resolution mtimes (HFS+, ext3) advance it).
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(&manifest_path, "[package]\nname=\"y\"\nversion=\"0.2.0\"\n").unwrap();

        let second = load_workspace_manifest(&mut ctx).expect("load2");
        assert!(
            !Arc::ptr_eq(&first.manifest, &second.manifest),
            "mtime change must invalidate cache and reparse"
        );

        evict_cache_for(dir.path());
    }

    /// CONC-2 / TASK-0843: cache size is soft-capped so a long-running
    /// process visiting many cwds never accumulates more than
    /// MAX_TYPED_MANIFEST_CACHE_ENTRIES entries.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn typed_manifest_cache_is_bounded() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Insert MAX + extras directly to exercise the eviction branch.
        // We bypass load_workspace_manifest for speed: the cap is enforced
        // inside the same locked section so the contract holds for the
        // observable behaviour as well.
        let cache = typed_manifest_cache();
        {
            let mut guard = cache.lock().unwrap();
            guard.map.clear();
            guard.victim_queue.clear();
        }
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
        let len = cache.lock().unwrap().map.len();
        assert!(
            len <= MAX_TYPED_MANIFEST_CACHE_ENTRIES,
            "cache size {len} must stay within MAX_TYPED_MANIFEST_CACHE_ENTRIES = {MAX_TYPED_MANIFEST_CACHE_ENTRIES}"
        );
        // Cleanup so we don't pollute later tests in the same process.
        let mut g = cache.lock().unwrap();
        g.map.clear();
        g.victim_queue.clear();
    }

    /// CONC-2 / TASK-1023: eviction must pick the least-recently-used entry,
    /// not an arbitrary HashMap bucket. Insert MAX entries, touch the very
    /// first one to make it "hot", then insert one more to force eviction
    /// and assert (a) the hot key is still present, and (b) the victim is
    /// the actual coldest key — not the hot one and not the new one.
    #[serial_test::serial(typed_manifest_cache)]
    #[test]
    fn typed_manifest_cache_evicts_lru_not_hot_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cache = typed_manifest_cache();
        {
            let mut guard = cache.lock().unwrap();
            guard.map.clear();
            guard.victim_queue.clear();
        }

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
            keys.push(cwd);
        }
        assert_eq!(
            cache.lock().unwrap().map.len(),
            MAX_TYPED_MANIFEST_CACHE_ENTRIES
        );

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

        let guard = cache.lock().unwrap();
        assert_eq!(
            guard.map.len(),
            MAX_TYPED_MANIFEST_CACHE_ENTRIES,
            "cap must hold after eviction"
        );
        assert!(
            guard.map.contains_key(&hot),
            "hot key must survive LRU eviction"
        );
        assert!(
            guard.map.contains_key(&fresh),
            "newly inserted key must remain"
        );
        assert!(
            !guard.map.contains_key(&coldest),
            "victim must be the coldest key (LRU), got cache keys: {:?}",
            guard.map.keys().collect::<Vec<_>>()
        );
        drop(guard);

        let mut g = cache.lock().unwrap();
        g.map.clear();
        g.victim_queue.clear();
    }

    fn manifest_with_members(members: &[&str]) -> CargoToml {
        let toml_str = format!(
            "[workspace]\nmembers = [{}]\n",
            members
                .iter()
                .map(|m| format!("\"{m}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );
        toml::from_str(&toml_str).expect("parse manifest")
    }

    fn manifest_with_members_and_exclude(members: &[&str], exclude: &[&str]) -> CargoToml {
        let toml_str = format!(
            "[workspace]\nmembers = [{}]\nexclude = [{}]\n",
            members
                .iter()
                .map(|m| format!("\"{m}\""))
                .collect::<Vec<_>>()
                .join(", "),
            exclude
                .iter()
                .map(|m| format!("\"{m}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );
        toml::from_str(&toml_str).expect("parse manifest")
    }

    #[test]
    fn resolves_simple_glob() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        std::fs::create_dir_all(root.join("crates/foo")).unwrap();
        std::fs::write(
            root.join("crates/foo/Cargo.toml"),
            "[package]\nname=\"foo\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("crates/bar")).unwrap();
        std::fs::write(
            root.join("crates/bar/Cargo.toml"),
            "[package]\nname=\"bar\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("crates/not-a-crate")).unwrap();

        let manifest = manifest_with_members(&["crates/*"]);
        let resolved = resolved_workspace_members(&manifest, root);

        assert_eq!(
            resolved,
            vec!["crates/bar".to_string(), "crates/foo".to_string()]
        );
    }

    /// PATTERN-1 (TASK-0803): unsupported glob shapes (brace alternation,
    /// character classes, `?`) must pass through unchanged so downstream
    /// rendering surfaces them as-is rather than producing a silently-empty
    /// member list.
    #[test]
    fn unsupported_glob_shapes_pass_through() {
        let root = std::path::Path::new("/nonexistent");
        for pattern in [
            "crates/{core,cli}",
            "crates/[a-z]*",
            "crates/foo?",
            "crates/foo]",
        ] {
            let manifest = manifest_with_members(&[pattern]);
            let resolved = resolved_workspace_members(&manifest, root);
            assert_eq!(
                resolved,
                vec![pattern.to_string()],
                "expected `{pattern}` to pass through unchanged"
            );
        }
    }

    #[test]
    fn passthrough_non_glob_members() {
        let manifest = manifest_with_members(&["crates/core", "crates/cli"]);
        let resolved = resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent"));
        assert_eq!(
            resolved,
            vec!["crates/cli".to_string(), "crates/core".to_string()]
        );
    }

    #[test]
    fn empty_when_no_workspace() {
        let manifest: CargoToml =
            toml::from_str("[package]\nname=\"x\"\nversion=\"0.1.0\"\n").expect("parse");
        let resolved = resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent"));
        assert!(resolved.is_empty());
    }

    #[test]
    fn nonexistent_glob_parent_yields_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let manifest = manifest_with_members(&["crates/*"]);
        let resolved = resolved_workspace_members(&manifest, dir.path());
        assert!(resolved.is_empty());
    }

    #[test]
    fn exclude_filters_explicit_members() {
        let manifest = manifest_with_members_and_exclude(
            &["crates/core", "crates/experimental"],
            &["crates/experimental"],
        );
        let resolved = resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent"));
        assert_eq!(resolved, vec!["crates/core".to_string()]);
    }

    #[test]
    fn exclude_filters_glob_results() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        for name in ["foo", "bar", "experimental"] {
            std::fs::create_dir_all(root.join(format!("crates/{name}"))).unwrap();
            std::fs::write(
                root.join(format!("crates/{name}/Cargo.toml")),
                "[package]\nname=\"x\"\n",
            )
            .unwrap();
        }

        let manifest = manifest_with_members_and_exclude(&["crates/*"], &["crates/experimental"]);
        let resolved = resolved_workspace_members(&manifest, root);
        assert_eq!(
            resolved,
            vec!["crates/bar".to_string(), "crates/foo".to_string()]
        );
    }

    /// Suffix-after-`*` (e.g. `crates/*/sub`) is not supported by the simple
    /// expander. The pattern is passed through unchanged with a warn-log
    /// rather than silently producing a wrong member list (TASK-0410).
    #[test]
    fn unsupported_suffix_after_star_passes_through() {
        let manifest = manifest_with_members(&["crates/*/sub"]);
        let resolved = resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent"));
        assert_eq!(resolved, vec!["crates/*/sub".to_string()]);
    }

    /// PATTERN-1 (TASK-1042): overlapping `[workspace].members` entries
    /// (literal + glob covering the same crate) must collapse to a single
    /// resolved member. Cargo itself dedups, so the about pipeline must too —
    /// otherwise `module_count` and the units / coverage providers would
    /// double-count the duplicated crate.
    #[test]
    fn duplicate_member_from_literal_and_glob_is_deduped() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        std::fs::create_dir_all(root.join("crates/foo")).unwrap();
        std::fs::write(
            root.join("crates/foo/Cargo.toml"),
            "[package]\nname=\"foo\"\n",
        )
        .unwrap();

        let manifest = manifest_with_members(&["crates/foo", "crates/*"]);
        let resolved = resolved_workspace_members(&manifest, root);

        assert_eq!(resolved, vec!["crates/foo".to_string()]);
    }

    #[test]
    fn unsupported_globstar_passes_through() {
        let manifest = manifest_with_members(&["crates/**"]);
        let resolved = resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent"));
        assert_eq!(resolved, vec!["crates/**".to_string()]);
    }
}
