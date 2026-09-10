//! Command-building helpers: cwd resolution, workspace-escape detection, and
//! tokio [`Command`] construction from an [`ExecCommandSpec`].
//!
//! [`resolve_spec_cwd`] documents the workspace-escape policy and the trust
//! model behind it.

use super::secret_patterns::warn_if_sensitive_env;
use ops_core::config::ExecCommandSpec;
use ops_core::expand::{ExpandError, Variables};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
#[cfg(test)]
use std::sync::{Arc, OnceLock};
use tokio::process::Command;

/// Return the next monotonic LRU access tick, stamped on every cache hit and
/// insert.
///
/// Mirrors the `next_lru_tick` pattern in
/// `extensions/about/src/manifest_cache.rs`. `Relaxed` ordering suffices
/// because every access happens under the cache mutex; victim selection only
/// needs a strictly increasing stamp, not cross-thread ordering.
fn next_workspace_lru_tick() -> u64 {
    static LRU_TICK: AtomicU64 = AtomicU64::new(0);
    LRU_TICK.fetch_add(1, Ordering::Relaxed)
}

/// Cap on the number of distinct workspace paths held resident.
/// Production runs see `1` key; tests inject many tempdirs.
/// The cap is a high-water mark so embedders and integration tests
/// cannot grow the cache without limit.
pub const WORKSPACE_CANONICAL_CACHE_CAP: usize = 256;

#[derive(Clone)]
struct WorkspaceCacheEntry {
    canonical: Option<PathBuf>,
    last_accessed: u64,
}

/// Bounded, runner-scoped cache of `canonicalize(workspace)` results.
///
/// The cache is an instance type owned by [`super::CommandRunner`]
/// (see `mod.rs`). When the runner is dropped, the cache and its entries go
/// with it. The runner exposes
/// [`super::CommandRunner::invalidate_workspace_cache`]
/// and [`super::CommandRunner::clear_workspace_cache`] for hosts that need to
/// react to a known on-disk change without dropping the runner — a cached
/// canonical path is otherwise never re-resolved, so a symlink swap under a
/// cached workspace would leave subsequent containment decisions stale.
///
/// Eviction policy mirrors `extensions/about/src/manifest_cache.rs`:
/// least-recently-used by access tick, evicted one entry at a time when the
/// cap is reached.
pub struct WorkspaceCanonicalCache {
    /// Map of raw workspace path to its cached canonicalization.
    ///
    /// A `Mutex` (rather than an `RwLock`) is sufficient: the hot path is
    /// dominated by lock-free reads downstream of the canonicalize syscall,
    /// and the per-spawn cost of the mutex acquisition is negligible against
    /// the work it guards. `Mutex` also matches `ArcTextCache`'s pattern,
    /// keeping the poison-recovery shape consistent across caches in this
    /// codebase.
    inner: Mutex<HashMap<PathBuf, WorkspaceCacheEntry>>,
    cap: usize,
}

impl WorkspaceCanonicalCache {
    /// Create an empty cache with the given residency cap.
    pub(crate) fn with_capacity(cap: usize) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            cap,
        }
    }

    /// Default-capacity constructor used by [`super::CommandRunner::new`].
    pub(crate) fn new() -> Self {
        Self::with_capacity(WORKSPACE_CANONICAL_CACHE_CAP)
    }

    /// Forget the cached canonicalization for `workspace` and any joined-path
    /// descendants so the next call re-runs `canonicalize`. Used by hosts that
    /// observe an on-disk swap.
    ///
    /// [`detect_workspace_escape`] caches joined-path canonicalizations in
    /// this same cache, keyed by the (uncanonicalised) joined path. A
    /// workspace symlink swap therefore invalidates not just the workspace
    /// entry but every joined-path entry underneath it, so both shapes are
    /// dropped in one pass and callers retain a single invalidate API.
    pub(crate) fn invalidate(&self, workspace: &Path) {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|e| recover_workspace_cache(e));
        guard.remove(workspace);
        guard.retain(|k, _| !k.starts_with(workspace));
    }

    /// Drop every cached entry. Useful for tests and for embedders that
    /// know the workspace layout has changed wholesale.
    #[allow(dead_code)]
    pub(crate) fn clear(&self) {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|e| recover_workspace_cache(e));
        guard.clear();
    }

    /// Look up — or compute and insert — the canonical form of `workspace`,
    /// using the supplied closure as the canonicalize implementation.
    ///
    /// # Concurrency contract
    ///
    /// The cache mutex is held across the `canonicalize` closure. This is
    /// **intentional** — the thundering-herd dedup the closure-under-lock
    /// shape provides is the whole reason for this cache: concurrent
    /// callers for the same uncached path collapse onto a single
    /// `canonicalize` syscall. The cost is that concurrent first-time
    /// lookups for **distinct** workspace paths also serialise on this
    /// mutex during the syscall.
    ///
    /// For `ops` today this is acceptable:
    /// - **Single-shot CLI:** one workspace per invocation; the cache
    ///   takes a single miss at startup and is uncontended thereafter.
    /// - **Test fixtures / embedders:** burst-startup with many workspace
    ///   paths can serialise on the closure, but the canonicalize cost
    ///   is bounded by the path depth and any contention is bounded by
    ///   `WORKSPACE_CANONICAL_CACHE_CAP` distinct keys.
    ///
    /// If a future high-concurrency caller (a multi-workspace daemon,
    /// or `MAX_PARALLEL=32` against many distinct workspaces) becomes
    /// the dominant workload, migrate to a per-key in-flight sentinel
    /// pattern: keep the outer mutex for the get-or-insert of an
    /// `Arc<OnceLock<Option<PathBuf>>>`, drop it before calling the
    /// closure, and let same-path readers serialise on the inner once-init
    /// while distinct paths run in parallel. The sibling cache in
    /// `extensions/about/src/manifest_cache.rs` uses that shape; this cache
    /// deliberately keeps the simpler one because the workload does not
    /// justify the extra indirection.
    pub(crate) fn get_or_compute<F>(&self, workspace: &Path, canonicalize: F) -> Option<PathBuf>
    where
        F: FnOnce(&Path) -> std::io::Result<PathBuf>,
    {
        let mut guard = self
            .inner
            .lock()
            .unwrap_or_else(|e| recover_workspace_cache(e));
        if let Some(entry) = guard.get_mut(workspace) {
            // ARCH-1 / TASK-1106: bump LRU tick on hit so frequently
            // accessed workspaces survive eviction.
            entry.last_accessed = next_workspace_lru_tick();
            return entry.canonical.clone();
        }
        // Cap-evict the LRU victim before inserting, mirroring
        // `ArcTextCache`'s policy in extensions/about/src/manifest_cache.rs.
        if guard.len() >= self.cap {
            if let Some(victim) = guard
                .iter()
                .min_by_key(|(_, e)| e.last_accessed)
                .map(|(k, _)| k.clone())
            {
                tracing::debug!(
                    cap = self.cap,
                    victim = ?victim.display(),
                    "workspace canonicalize cache reached cap; evicting LRU entry"
                );
                guard.remove(&victim);
            }
        }
        let canonical = canonicalize(workspace).ok();
        guard.insert(
            workspace.to_path_buf(),
            WorkspaceCacheEntry {
                canonical: canonical.clone(),
                last_accessed: next_workspace_lru_tick(),
            },
        );
        debug_assert!(
            guard.len() <= self.cap,
            "workspace canonicalize cache exceeded cap of {}",
            self.cap
        );
        // CONC-1: release the cache lock before returning; `canonical` is an
        // owned clone and needs no further access to the map.
        drop(guard);
        canonical
    }
}

impl Default for WorkspaceCanonicalCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Recover a poisoned cache guard instead of propagating the panic.
///
/// The cached value is the pure result of `canonicalize`, not authoritative
/// state, so a panic in a lock holder cannot leave a torn invariant; treating
/// poison as fatal would brick the cache for every other caller in the
/// process.
fn recover_workspace_cache<T>(
    err: std::sync::PoisonError<std::sync::MutexGuard<'_, T>>,
) -> std::sync::MutexGuard<'_, T> {
    tracing::warn!("workspace canonicalize cache mutex was poisoned by a prior panic; recovered");
    err.into_inner()
}

/// Return the canonical form of `workspace`, cached by raw path.
///
/// The cache instance is a parameter rather than a process-global static so
/// the spawn path consults exactly the cache that
/// [`super::CommandRunner::invalidate_workspace_cache`] and
/// [`super::CommandRunner::clear_workspace_cache`] mutate; otherwise those
/// public APIs would be no-ops against the cache that actually decides
/// escape outcomes.
fn canonical_workspace_cached(
    cache: &WorkspaceCanonicalCache,
    workspace: &Path,
) -> Option<PathBuf> {
    cache.get_or_compute(workspace, |p| std::fs::canonicalize(p))
}

/// Testable seam for the cache: lets tests inject a canonicalize counter and
/// verify that a burst-startup thundering herd collapses to a single syscall
/// per workspace path.
#[cfg(test)]
fn canonical_workspace_cached_with<F>(
    cache: &WorkspaceCanonicalCache,
    workspace: &Path,
    canonicalize: F,
) -> Option<PathBuf>
where
    F: FnOnce(&Path) -> std::io::Result<PathBuf>,
{
    cache.get_or_compute(workspace, canonicalize)
}

/// Test-only ambient cache, so `resolve_spec_cwd` / `detect_workspace_escape`
/// tests need not construct one per assertion.
///
/// Production callers MUST thread the runner-scoped
/// `Arc<WorkspaceCanonicalCache>` and never reach this static.
#[cfg(test)]
pub fn test_default_workspace_cache() -> &'static Arc<WorkspaceCanonicalCache> {
    static CACHE: OnceLock<Arc<WorkspaceCanonicalCache>> = OnceLock::new();
    CACHE.get_or_init(|| Arc::new(WorkspaceCanonicalCache::new()))
}

/// Convert a strict-expansion error into an `io::Error` so build failures
/// share the spawn-error pipeline and surface as a `StepFailed` event.
///
/// The produced `io::Error` is the source of `StepFailed.message` and the TAP
/// file body, both of which round-trip to CI artifacts. The full chain
/// (including the offending variable name and the underlying `VarError`) is
/// logged at `tracing::debug!`, mirroring `log_and_redact_spawn_error`, while
/// the returned message stays generic so a variable name from a
/// `.ops.toml`-supplied `${OPS_TOKEN}` reference cannot leak into uploaded CI
/// logs. Operators chasing the detail use the same `RUST_LOG=debug` path as
/// for spawn-error redaction.
// Taken by value so the four call sites can stay point-free
// (`.map_err(expand_err_to_io)`); a `&ExpandError` parameter would force a
// closure at each. Consuming the error also matches `From<E> for io::Error`.
#[allow(clippy::needless_pass_by_value)]
fn expand_err_to_io(err: ExpandError) -> std::io::Error {
    tracing::debug!(
        error = ?err,
        var_name = ?err.var_name,
        "expand: variable expansion failed (full error)"
    );
    std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "variable expansion failed",
    )
}

/// Lexically normalize a path by resolving `.` and `..` components without I/O.
fn normalize_path(p: &std::path::Path) -> std::path::PathBuf {
    use std::path::Component;
    let mut out = std::path::PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push(c);
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Policy for how to treat spec `cwd` values that escape the workspace root.
///
/// Interactive invocations (`ops <cmd>`) tolerate escapes with a warning —
/// `.ops.toml` is trusted the way a Makefile is trusted. Hook-triggered
/// invocations (`run-before-commit`, `run-before-push`) are strict: a
/// co-worker's PR can land a `.ops.toml` that runs on every commit the
/// maintainer makes, so the hook path fails closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CwdEscapePolicy {
    /// Log a warning and spawn anyway. Default for interactive `ops run`.
    #[default]
    WarnAndAllow,
    /// Refuse to spawn; return an error. Used by git-hook-triggered paths.
    ///
    /// Hook-triggered entry points (`run-before-commit`, `run-before-push`)
    /// construct a `CommandRunner` with this policy so a `.ops.toml` landed
    /// by a coworker PR cannot escape the workspace on the next commit. The
    /// interactive path is `WarnAndAllow`.
    ///
    /// # Residual TOCTOU window
    ///
    /// The check happens in
    /// [`detect_workspace_escape`], which calls `std::fs::canonicalize`,
    /// while the actual `chdir` is performed by the OS when the child is
    /// spawned. To shrink the window, [`resolve_spec_cwd`] canonicalizes
    /// the joined path on a best-effort basis under *both* policies and
    /// hands the symlink-free result to `current_dir`, so the kernel does
    /// not re-resolve any symlinks at exec time. The two policies share
    /// the same TOCTOU surface; `Deny` differs only in failing closed on
    /// detected escapes. A narrow race remains: an attacker who can
    /// replace a component of the canonical path (e.g. by mounting over it
    /// or swapping a directory they own) between canonicalization and exec
    /// can still divert the child. Closing this fully would require an
    /// `openat`/`fchdir`-style fd handoff to the child, which neither
    /// `std::process::Command` nor `tokio::process::Command` exposes
    /// today.
    ///
    /// Two fail-closed properties this policy delivers:
    ///
    /// 1. **Fail closed on an unresolvable path.** A `canonicalize`
    ///    failure that is not `NotFound` (`EACCES` on an intermediate
    ///    directory, `ELOOP`, `ENAMETOOLONG`, an unstattable mount point)
    ///    is refused and logged — collapsing it to "does not escape"
    ///    would leave only the lexical check, which cannot see symlinks,
    ///    the very case the canonical check exists for. `NotFound` is
    ///    exempt: a path that does not exist cannot be a symlink out of
    ///    the workspace, and the spawn fails on its own.
    /// 2. **No runner-lifetime memo of the decision.** `Deny`
    ///    canonicalizes both sides per spawn rather than reading the
    ///    [`WorkspaceCanonicalCache`], so a symlink swapped after the
    ///    first spawn of a path is re-detected on the next spawn. The
    ///    cache still serves `WarnAndAllow`, whose check is advisory.
    Deny,
}

/// Classification of how a spec `cwd` relates to the workspace root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeKind {
    /// Path stays inside the workspace under both lexical and canonical checks.
    Inside,
    /// Path escapes the workspace (lexically and/or via symlink canonicalization).
    Escapes,
}

/// The three outcomes of canonicalizing a path, kept distinct so a
/// *fail-closed* policy can tell "this path does not exist" from "this path
/// could not be resolved".
enum CanonicalOutcome {
    /// Resolved to a symlink-free absolute path.
    Resolved(PathBuf),
    /// `NotFound`. A path that does not exist cannot be a symlink pointing
    /// out of the workspace, so the lexical check alone is authoritative —
    /// and the spawn will fail on its own with a clearer error.
    Missing,
    /// Anything else — `EACCES` on an intermediate directory, `ELOOP`,
    /// `ENAMETOOLONG`, a mount point the process cannot stat. The
    /// containment question is *unanswerable*, which is not the same as
    /// answering "no".
    Undetermined(std::io::ErrorKind),
}

/// Canonicalize `p` right now, classifying the failure kind.
///
/// Deliberately uncached — see [`detect_workspace_escape`].
fn canonicalize_now(p: &Path) -> CanonicalOutcome {
    match std::fs::canonicalize(p) {
        Ok(c) => CanonicalOutcome::Resolved(c),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => CanonicalOutcome::Missing,
        Err(e) => CanonicalOutcome::Undetermined(e.kind()),
    }
}

/// Classify `joined` against `workspace`. Fast lexical check first, then a
/// canonical check so a symlink inside the workspace pointing outside is
/// still caught.
///
/// # Policy-dependent canonicalization
///
/// The canonical half behaves differently under the two policies, because
/// they want different things from it:
///
/// - **`Deny`** (the hook path) is fail-closed, so it canonicalizes **both
///   sides per call, uncached**, and treats an *unresolvable* path
///   (`EACCES`, `ELOOP`, `ENAMETOOLONG`, an unstattable mount point) as an
///   escape — swallowing the error would leave only the lexical check,
///   which cannot see symlinks, the exact case this function exists for.
///   Re-resolving per spawn also keeps the TOCTOU window no wider than
///   "between this spawn's canonicalize and this spawn's exec". The cost
///   is two `canonicalize` syscalls per spawn on the hook path, which is
///   single-shot and short.
/// - **`WarnAndAllow`** (interactive) warns and proceeds by design, so its
///   check is advisory and reads the [`WorkspaceCanonicalCache`]: a
///   composite fanning the same `cwd = "sub"` over many parallel spawns
///   pays one canonicalize per distinct path rather than one per spawn.
///   An unresolvable path stays non-escaping here — failing closed would
///   turn an advisory warning into a refusal the policy does not promise.
///
/// Because `Deny` does not read the cache, a fan of distinct `cwd` values
/// evicting the workspace entry under
/// [`WORKSPACE_CANONICAL_CACHE_CAP`] cannot change a `Deny` outcome; under
/// `WarnAndAllow` an eviction only costs a re-canonicalize, which yields
/// the same classification.
pub fn detect_workspace_escape(
    cache: &WorkspaceCanonicalCache,
    joined: &std::path::Path,
    workspace: &std::path::Path,
    policy: CwdEscapePolicy,
) -> EscapeKind {
    let lexically_escapes = !normalize_path(joined).starts_with(workspace);
    let canonically_escapes = match policy {
        CwdEscapePolicy::Deny => {
            match (canonicalize_now(joined), canonicalize_now(workspace)) {
                (CanonicalOutcome::Resolved(a), CanonicalOutcome::Resolved(b)) => {
                    !a.starts_with(&b)
                }
                // Fail closed: "cannot determine" is treated as "escapes",
                // and is logged so an operator hitting a permissions
                // problem sees why the spawn was refused.
                (CanonicalOutcome::Undetermined(kind), _)
                | (_, CanonicalOutcome::Undetermined(kind)) => {
                    // SEC-21 / TASK-1937: paths are `.ops.toml`-derived.
                    tracing::warn!(
                        joined = ?joined.display(),
                        workspace = ?workspace.display(),
                        error_kind = ?kind,
                        "SEC-25: cwd could not be canonicalized under the Deny policy; \
                         treating it as an escape"
                    );
                    true
                }
                // `NotFound` on either side: the lexical check governs.
                _ => false,
            }
        }
        CwdEscapePolicy::WarnAndAllow => match (
            canonical_workspace_cached(cache, joined),
            canonical_workspace_cached(cache, workspace),
        ) {
            (Some(a), Some(b)) => !a.starts_with(&b),
            _ => false,
        },
    };
    if lexically_escapes || canonically_escapes {
        EscapeKind::Escapes
    } else {
        EscapeKind::Inside
    }
}

/// Apply an escape policy to a detected escape.
///
/// `Deny` converts it to an `io::Error`; `WarnAndAllow` emits a tracing
/// warning and lets the caller continue.
pub fn apply_escape_policy(
    policy: CwdEscapePolicy,
    spec_cwd: &std::path::Path,
    workspace_cwd: &std::path::Path,
    joined: &std::path::Path,
) -> Result<(), std::io::Error> {
    match policy {
        CwdEscapePolicy::Deny => Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!(
                "SEC-14: refusing to spawn: spec cwd {} escapes workspace root {}",
                spec_cwd.display(),
                workspace_cwd.display()
            ),
        )),
        CwdEscapePolicy::WarnAndAllow => {
            // SEC-21 / TASK-1937: `spec_cwd` is `.ops.toml`-supplied and
            // `joined` is derived from it, so both are Debug-formatted —
            // `Path::display()` renders embedded newlines and ANSI escapes
            // verbatim, which would let a config forge log records on the
            // very warning that reports it escaping the workspace. Matches
            // the policy already applied to `program` (TASK-1127) and tap
            // paths (TASK-0940).
            tracing::warn!(
                cwd = ?workspace_cwd.display(),
                spec_cwd = ?spec_cwd.display(),
                resolved = ?joined.display(),
                "SEC-004: spec cwd escapes workspace root"
            );
            Ok(())
        }
    }
}

/// Resolve an exec spec's `cwd` field against the workspace root, canonicalizing
/// both sides before the containment check so symlinks cannot smuggle an
/// absolute path past the check lexically.
///
/// Returns an error when the resolved path escapes the workspace root **and**
/// `policy` is [`CwdEscapePolicy::Deny`] (the hook path). Otherwise logs and
/// continues.
pub fn resolve_spec_cwd(
    cache: &WorkspaceCanonicalCache,
    spec_cwd: Option<&std::path::Path>,
    workspace_cwd: &std::path::Path,
    vars: &Variables,
    policy: CwdEscapePolicy,
) -> Result<std::path::PathBuf, std::io::Error> {
    let Some(p) = spec_cwd else {
        return Ok(workspace_cwd.to_path_buf());
    };
    // READ-5 / TASK-0900: previously this called `p.to_string_lossy()`
    // before variable expansion, silently replacing non-UTF-8 bytes with
    // U+FFFD and then spawning the child in the wrong-but-similar
    // directory. Reject non-UTF-8 cwd values loudly so the operator
    // sees a real error instead of a quiet redirect.
    // `{p:?}` is deliberate: the path is not valid UTF-8, which is exactly
    // what this error reports, so `Display` is not available losslessly.
    #[allow(clippy::unnecessary_debug_formatting)]
    let s = p.to_str().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "spec cwd contains non-UTF-8 bytes; refusing to lossy-expand and chdir into a wrong-but-similar path: {p:?}"
            ),
        )
    })?;
    let expanded = vars.try_expand(s).map_err(expand_err_to_io)?;
    let ep = std::path::PathBuf::from(expanded.as_ref());
    // SEC-23 / TASK-0500: an absolute spec_cwd must still be checked against
    // the workspace root. A malicious `cwd = "/etc"` would previously bypass
    // the policy entirely because it short-circuited here without invoking
    // detect_workspace_escape. Run the check against the absolute path
    // unchanged (it is its own joined form) and let `apply_escape_policy`
    // decide whether to allow or deny.
    let joined = if ep.is_relative() {
        workspace_cwd.join(&ep)
    } else {
        ep.clone()
    };
    if detect_workspace_escape(cache, &joined, workspace_cwd, policy) == EscapeKind::Escapes {
        apply_escape_policy(policy, &ep, workspace_cwd, &joined)?;
    }
    // SEC-25 / SEC-23 / READ-5 / TASK-0773 / TASK-1140: hand the kernel a
    // symlink-free canonical path so it does not re-resolve symlinks at
    // chdir time. Narrows (but does not close) the TOCTOU window — see
    // `CwdEscapePolicy::Deny` docs. Applied uniformly to relative *and*
    // absolute spec_cwd values *and* under both policies: TASK-1140 lifts
    // the previous `Deny`-only gate because `detect_workspace_escape`
    // already pays the canonicalize cost regardless, and gating the
    // symlink-free handoff on `Deny` left the interactive `WarnAndAllow`
    // path uniquely exposed to symlink swap between the escape check and
    // the spawn. Best effort: if canonicalize fails (e.g. cwd does not
    // exist yet), fall back to the joined path and let the OS surface the
    // spawn error.
    if let Ok(canonical) = std::fs::canonicalize(&joined) {
        return Ok(canonical);
    }
    if !ep.is_relative() {
        return Ok(ep);
    }
    Ok(joined)
}

/// Build a tokio Command from an exec spec and working directory.
///
/// ## Cwd traversal guard
///
/// Delegates to [`resolve_spec_cwd`] with [`CwdEscapePolicy::WarnAndAllow`],
/// which warns but still spawns (interactive trust model). Callers that
/// need fail-closed behaviour (git hooks) should call [`build_command_with`]
/// with [`CwdEscapePolicy::Deny`].
///
/// Note: `current_dir` is validated by the OS when the command is spawned — if the
/// path does not exist, `Command::output()` returns an `io::Error` that propagates
/// through the existing error handling in `exec_command`.
// The no-panic guarantee is structural in the return type: `build_command`
// returns `Result` because variable expansion is fallible (a non-UTF-8 env
// var must surface as a step failure rather than crashing the runner), and
// every caller threads the error into a `StepFailed` event.
#[cfg(test)]
pub fn build_command(
    spec: &ExecCommandSpec,
    cwd: &std::path::Path,
    vars: &Variables,
) -> Result<Command, std::io::Error> {
    let cache = WorkspaceCanonicalCache::new();
    build_command_with(&cache, spec, cwd, vars, CwdEscapePolicy::WarnAndAllow)
}

/// Async variant that runs the synchronous filesystem work in
/// `build_command` (notably `std::fs::canonicalize` calls inside
/// [`detect_workspace_escape`] and [`resolve_spec_cwd`]) on the blocking
/// thread pool.
///
/// Without this, every parallel command spawn blocks a tokio worker on
/// `canonicalize` syscalls — slow on NFS or symlink-heavy paths and
/// proportional to the spec cwd's depth. Under high `MAX_PARALLEL` counts
/// that starves other tasks scheduled on the same worker.
///
/// `spec`, `vars` and `cwd` are all taken as `Arc`, so the only per-spawn
/// allocations on the parallel hot path are `Arc::clone` refcount bumps —
/// never a deep clone of `Variables`, of a `PathBuf`, or of the spec's
/// `args: Vec<String>` / `env: IndexMap` / `program: String`. Callers must
/// share one instance across spawns rather than building a fresh `Arc` per
/// call; the debug assertions below pin that invariant.
pub async fn build_command_async(
    cache: std::sync::Arc<WorkspaceCanonicalCache>,
    spec: std::sync::Arc<ExecCommandSpec>,
    cwd: std::sync::Arc<std::path::PathBuf>,
    vars: std::sync::Arc<Variables>,
    policy: CwdEscapePolicy,
) -> Result<Command, std::io::Error> {
    // API-2 / TASK-0659: pin the Arc-only invariant in debug builds. Production
    // call sites pass `Arc::clone(cwd_ref)` from a `&Arc<...>` held by the
    // caller, so the strong_count is always ≥ 2 on the parallel hot path.
    // A future caller that reverts to `Arc::new(fresh_pathbuf)` per spawn
    // (re-introducing the deep-clone regression that TASK-0462 fixed) will
    // trip this assert in test runs.
    debug_assert!(
        std::sync::Arc::strong_count(&cwd) > 1,
        "OWN-2 / API-2: cwd Arc must be shared across spawns (strong_count > 1); fresh Arc::new per call defeats the Arc-only invariant"
    );
    debug_assert!(
        std::sync::Arc::strong_count(&vars) > 1,
        "OWN-2 / API-2: vars Arc must be shared across spawns (strong_count > 1); fresh Arc::new per call defeats the Arc-only invariant"
    );
    // OWN-2 / TASK-0462: emit a trace event on every spawn so we can
    // confirm in `RUST_LOG=trace` runs that the only allocations per
    // spawn are Arc::clone counts (logged here as the existing
    // strong_count) and the spec move — no Variables/PathBuf deep
    // clones. Strong counts > 1 prove the parallel path is sharing the
    // same instance across MAX_PARALLEL workers.
    // SEC-21 / TASK-1127: spec.program is `.ops.toml`-supplied; format via Debug so
    // embedded newlines/ANSI cannot forge log entries on this trace event either.
    tracing::trace!(
        program = ?spec.program,
        vars_strong = std::sync::Arc::strong_count(&vars),
        cwd_strong = std::sync::Arc::strong_count(&cwd),
        "build_command_async: Arc-only inputs, no deep clone"
    );
    // ERR-5 / TASK-0456: a panicking blocking task previously surfaced
    // here as a runner-wide panic via `.expect`. Now we downgrade to a
    // `tracing::error!` plus a synthesized `io::Error` so the calling
    // step fails gracefully (StepFailed) instead of aborting the runner.
    // Cancellation of the blocking task is treated identically — it can
    // only happen if the runtime is shutting down, in which case
    // returning Err is no worse than a hard panic.
    match tokio::task::spawn_blocking(move || {
        build_command_with(&cache, spec.as_ref(), cwd.as_ref(), vars.as_ref(), policy)
    })
    .await
    {
        Ok(result) => result,
        Err(join_err) => {
            tracing::error!(
                error = %join_err,
                "ERR-5: build_command panicked on blocking pool; converting to step failure"
            );
            Err(std::io::Error::other(format!(
                "build_command panicked on blocking pool: {join_err}"
            )))
        }
    }
}

/// Build a tokio Command with an explicit cwd-escape policy. Returns `Err`
/// only when `policy == Deny` and the spec's cwd escapes the workspace root.
pub fn build_command_with(
    cache: &WorkspaceCanonicalCache,
    spec: &ExecCommandSpec,
    cwd: &std::path::Path,
    vars: &Variables,
    policy: CwdEscapePolicy,
) -> Result<Command, std::io::Error> {
    let program = vars.try_expand(&spec.program).map_err(expand_err_to_io)?;
    let mut cmd = Command::new(program.as_ref());
    // PERF-2 / TASK-0772: stream expanded args directly into `cmd.arg`. The
    // common case (no `${VAR}` substitution) returns `Cow::Borrowed`, so
    // `arg(expanded.as_ref())` does not allocate at all — the prior path
    // collected into a fresh `Vec<String>` regardless. Errors short-circuit
    // on the first failing arg, matching the previous behaviour.
    for a in &spec.args {
        let expanded = vars.try_expand(a).map_err(expand_err_to_io)?;
        cmd.arg(expanded.as_ref());
    }
    let resolved_cwd = resolve_spec_cwd(cache, spec.cwd.as_deref(), cwd, vars, policy)?;
    cmd.current_dir(&resolved_cwd);
    for (k, v) in &spec.env {
        let expanded_v = vars.try_expand(v).map_err(expand_err_to_io)?;
        warn_if_sensitive_env(k, &expanded_v);
        cmd.env(k, expanded_v.as_ref());
    }
    // CONC-9 / TASK-1919: `kill_on_drop` reaches the *direct child only*, so
    // it is no longer the primary cancellation mechanism on unix — captured
    // spawns are made process-group leaders in `spawn_capped` and the group
    // is signalled by `ChildGroup` (see `super::process_group`). It stays as
    // the last-resort reaper for the leader itself, and remains the only
    // mechanism on non-unix targets, where process groups do not exist.
    cmd.kill_on_drop(true);
    Ok(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::test_utils::{exec_spec, exec_spec_with_cwd};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// The async variant dispatches its canonicalize work to the blocking
    /// pool, so a single-threaded runtime can still drive other tasks while
    /// `build_command` runs.
    ///
    /// The test uses a `current_thread` runtime — one worker — and asserts a
    /// concurrent counter task makes progress while `build_command_async` is
    /// in flight. Running the filesystem work on the worker itself would
    /// block it for the duration of every canonicalize syscall and starve
    /// every other task scheduled there.
    #[test]
    fn build_command_async_does_not_starve_concurrent_tokio_task() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let counter = Arc::new(AtomicUsize::new(0));
            let c = counter.clone();
            let counting = tokio::spawn(async move {
                for _ in 0..200 {
                    tokio::task::yield_now().await;
                    c.fetch_add(1, Ordering::Relaxed);
                }
            });

            let tmp = tempfile::tempdir().unwrap();
            std::fs::create_dir(tmp.path().join("sub")).unwrap();
            let vars = Variables::from_env(tmp.path()).expect("UTF-8 path");

            // Run several build_command_async invocations. Each dispatches
            // canonicalize to the blocking pool, leaving the runtime
            // worker free to poll the counting task between awaits.
            // API-2 / TASK-0659: hold the Arcs in the test so each call's
            // strong_count > 1, mirroring the production call pattern
            // (`Arc::clone` from a held reference) and satisfying the
            // debug_assert pinned in build_command_async.
            let cwd_arc = std::sync::Arc::new(tmp.path().to_path_buf());
            let vars_arc = std::sync::Arc::new(vars.clone());
            let cache_arc = std::sync::Arc::clone(test_default_workspace_cache());
            for _ in 0..5 {
                let spec = std::sync::Arc::new(exec_spec_with_cwd(
                    "echo",
                    &["x"],
                    Some(std::path::PathBuf::from("sub")),
                ));
                let _cmd = build_command_async(
                    std::sync::Arc::clone(&cache_arc),
                    spec,
                    std::sync::Arc::clone(&cwd_arc),
                    std::sync::Arc::clone(&vars_arc),
                    CwdEscapePolicy::WarnAndAllow,
                )
                .await
                .unwrap();
            }

            counting.await.unwrap();
            assert_eq!(
                counter.load(Ordering::Relaxed),
                200,
                "concurrent task must run to completion despite repeated build_command_async calls"
            );
        });
    }

    /// Functional parity: the async wrapper must produce a Command with
    /// the same observable program as the sync version. Catches refactors
    /// that accidentally rewrite the spec inside `spawn_blocking`.
    #[tokio::test]
    async fn build_command_async_preserves_program_name() {
        let tmp = tempfile::tempdir().unwrap();
        let vars = Variables::from_env(tmp.path()).expect("UTF-8 path");
        let spec = std::sync::Arc::new(exec_spec("echo", &["hello"]));
        // API-2 / TASK-0659: hold the Arcs locally so strong_count > 1
        // when the call clones them, satisfying the Arc-only debug_assert.
        let cwd_arc = std::sync::Arc::new(tmp.path().to_path_buf());
        let vars_arc = std::sync::Arc::new(vars);
        let cache_arc = std::sync::Arc::clone(test_default_workspace_cache());
        let cmd = build_command_async(
            std::sync::Arc::clone(&cache_arc),
            spec,
            std::sync::Arc::clone(&cwd_arc),
            std::sync::Arc::clone(&vars_arc),
            CwdEscapePolicy::WarnAndAllow,
        )
        .await
        .unwrap();
        // tokio::process::Command exposes the program via as_std()
        let program = cmd.as_std().get_program().to_string_lossy().into_owned();
        assert_eq!(program, "echo");
    }

    // SEC-14 / FN-1 regression tests for the extracted resolve_spec_cwd.
    #[test]
    fn resolve_spec_cwd_none_returns_workspace() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let vars = Variables::from_env(&ws).expect("UTF-8 path");
        let out = resolve_spec_cwd(
            test_default_workspace_cache(),
            None,
            &ws,
            &vars,
            CwdEscapePolicy::WarnAndAllow,
        )
        .unwrap();
        assert_eq!(out, ws);
    }

    #[test]
    fn resolve_spec_cwd_absolute_inside_workspace_is_returned_verbatim() {
        // SEC-23 / TASK-0500: absolute paths still go through the escape
        // check. A path lexically inside the workspace is allowed under
        // Deny; verbatim because absolute paths are not joined.
        let ws = std::path::PathBuf::from("/tmp/ws");
        let vars = Variables::from_env(&ws).expect("UTF-8 path");
        let abs = std::path::Path::new("/tmp/ws/inside");
        let out = resolve_spec_cwd(
            test_default_workspace_cache(),
            Some(abs),
            &ws,
            &vars,
            CwdEscapePolicy::Deny,
        )
        .unwrap();
        assert_eq!(out, std::path::PathBuf::from("/tmp/ws/inside"));
    }

    /// An absolute `spec_cwd` outside the workspace is rejected under `Deny`:
    /// an absolute path is checked against the workspace root like a relative
    /// one, so `cwd = "/etc"` cannot spawn at `/etc` on the hook path.
    #[test]
    fn resolve_spec_cwd_absolute_outside_workspace_is_denied() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let vars = Variables::from_env(&ws).expect("UTF-8 path");
        let abs = std::path::Path::new("/etc");
        let err = resolve_spec_cwd(
            test_default_workspace_cache(),
            Some(abs),
            &ws,
            &vars,
            CwdEscapePolicy::Deny,
        )
        .expect_err("absolute path outside workspace must be denied under Deny");
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(err.to_string().contains("SEC-14"));
    }

    /// Under `WarnAndAllow` the absolute path is still returned (the
    /// interactive trust model lets `.ops.toml` choose its cwd) but the
    /// escape is logged.
    #[test]
    fn resolve_spec_cwd_absolute_outside_workspace_warns_under_warn_and_allow() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let vars = Variables::from_env(&ws).expect("UTF-8 path");
        let abs = std::path::Path::new("/opt/elsewhere");
        let out = resolve_spec_cwd(
            test_default_workspace_cache(),
            Some(abs),
            &ws,
            &vars,
            CwdEscapePolicy::WarnAndAllow,
        )
        .unwrap();
        assert_eq!(out, std::path::PathBuf::from("/opt/elsewhere"));
    }

    #[test]
    fn resolve_spec_cwd_deny_rejects_escape() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let vars = Variables::from_env(&ws).expect("UTF-8 path");
        let escaping = std::path::Path::new("../etc");
        let err = resolve_spec_cwd(
            test_default_workspace_cache(),
            Some(escaping),
            &ws,
            &vars,
            CwdEscapePolicy::Deny,
        )
        .expect_err("escape should fail under Deny");
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(err.to_string().contains("SEC-14"));
    }

    #[test]
    fn resolve_spec_cwd_warn_allows_escape() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let vars = Variables::from_env(&ws).expect("UTF-8 path");
        let escaping = std::path::Path::new("../etc");
        let out = resolve_spec_cwd(
            test_default_workspace_cache(),
            Some(escaping),
            &ws,
            &vars,
            CwdEscapePolicy::WarnAndAllow,
        )
        .unwrap();
        // Still joined; caller trusts `.ops.toml` in interactive mode.
        assert_eq!(out, ws.join("../etc"));
    }

    #[test]
    fn resolve_spec_cwd_relative_inside_workspace_is_joined() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let vars = Variables::from_env(&ws).expect("UTF-8 path");
        let inside = std::path::Path::new("sub/dir");
        let out = resolve_spec_cwd(
            test_default_workspace_cache(),
            Some(inside),
            &ws,
            &vars,
            CwdEscapePolicy::Deny,
        )
        .unwrap();
        assert_eq!(out, ws.join("sub/dir"));
    }

    /// A non-UTF-8 cwd surfaces a loud `InvalidInput` error rather than
    /// being lossy-expanded into a wrong-but-similar path that would chdir
    /// the child into the wrong directory.
    #[cfg(unix)]
    #[test]
    fn resolve_spec_cwd_rejects_non_utf8_cwd_loudly() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        let ws = std::path::PathBuf::from("/tmp/ws");
        let bad: OsString = OsString::from_vec(vec![b's', b'u', b'b', 0xff]);
        let bad_path: std::path::PathBuf = bad.into();
        let vars = Variables::from_env(&ws).expect("UTF-8 path");
        let err = resolve_spec_cwd(
            test_default_workspace_cache(),
            Some(bad_path.as_path()),
            &ws,
            &vars,
            CwdEscapePolicy::WarnAndAllow,
        )
        .expect_err("non-UTF-8 cwd must surface as an error");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(
            err.to_string().contains("non-UTF-8"),
            "error message must mention the cause, got: {err}"
        );
    }

    #[test]
    fn detect_workspace_escape_inside_is_inside() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let inside = ws.join("sub/dir");
        assert_eq!(
            detect_workspace_escape(
                test_default_workspace_cache(),
                &inside,
                &ws,
                CwdEscapePolicy::WarnAndAllow
            ),
            EscapeKind::Inside
        );
    }

    #[test]
    fn detect_workspace_escape_parent_escapes() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let escaping = ws.join("../etc");
        assert_eq!(
            detect_workspace_escape(
                test_default_workspace_cache(),
                &escaping,
                &ws,
                CwdEscapePolicy::WarnAndAllow
            ),
            EscapeKind::Escapes
        );
    }

    #[test]
    fn apply_escape_policy_deny_returns_permission_denied() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let spec = std::path::Path::new("../etc");
        let joined = ws.join(spec);
        let err = apply_escape_policy(CwdEscapePolicy::Deny, spec, &ws, &joined)
            .expect_err("Deny should produce an error");
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn apply_escape_policy_warn_is_ok() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let spec = std::path::Path::new("../etc");
        let joined = ws.join(spec);
        assert!(apply_escape_policy(CwdEscapePolicy::WarnAndAllow, spec, &ws, &joined).is_ok());
    }

    /// An absolute `spec_cwd` inside the workspace goes through the same
    /// canonicalize-under-`Deny` narrowing as a relative one. Pins that
    /// symmetry so a refactor cannot leave absolute hook-path cwds
    /// unprotected.
    #[cfg(unix)]
    #[test]
    fn deny_canonicalizes_absolute_inside_workspace() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = std::fs::canonicalize(tmp.path()).unwrap();
        let inside = ws.join("sub");
        std::fs::create_dir(&inside).unwrap();
        let escape_target = tempfile::tempdir().unwrap();
        let escape_target_canonical = std::fs::canonicalize(escape_target.path()).unwrap();

        let vars = Variables::from_env(&ws).expect("UTF-8 path");
        let resolved = resolve_spec_cwd(
            test_default_workspace_cache(),
            Some(&inside),
            &ws,
            &vars,
            CwdEscapePolicy::Deny,
        )
        .expect("absolute path inside workspace must be allowed under Deny");
        assert_eq!(resolved, inside, "Deny should return the canonical path");

        // Swap the absolute target for a symlink to outside the workspace.
        // The already-resolved canonical path is unaffected — that is the
        // protection canonicalize-under-Deny grants to absolute paths.
        std::fs::remove_dir(&inside).unwrap();
        std::os::unix::fs::symlink(&escape_target_canonical, &inside).unwrap();
        assert_ne!(
            resolved, escape_target_canonical,
            "resolved path must not be the post-swap escape target"
        );
    }

    /// Under burst startup with N threads asking for the same fresh
    /// workspace path, the cache collapses the thundering herd to exactly
    /// one canonicalize call.
    ///
    /// The test builds a fresh `WorkspaceCanonicalCache` per invocation, so
    /// the property is pinned at the cache API surface rather than against
    /// the `test_default_workspace_cache()` static, where a broken
    /// runner-scoped cache could still pass.
    ///
    /// Because the cache holds a single mutex across the closure, the
    /// burst-dedup property holds regardless of how long the closure takes:
    /// racers either queue on the mutex or arrive after population, and both
    /// branches return the cached value without re-running the closure. A
    /// `Barrier(N)` at thread start is therefore the only rendezvous needed —
    /// no wall-clock sleep — and `temp_dir()` keeps the test portable.
    #[test]
    fn canonical_workspace_cached_collapses_burst_to_single_canonicalize() {
        use std::sync::Barrier;

        let cache = Arc::new(WorkspaceCanonicalCache::new());
        let key = std::env::temp_dir().join("ops-task-1095-burst");
        let n = 32usize;
        let counter = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(n));
        let mut handles = Vec::with_capacity(n);
        let expected = std::path::PathBuf::from("/expected/canonical");
        for _ in 0..n {
            let cache = Arc::clone(&cache);
            let counter = Arc::clone(&counter);
            let barrier = Arc::clone(&barrier);
            let key = key.clone();
            let expected = expected.clone();
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                canonical_workspace_cached_with(&cache, &key, |_p| {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(expected.clone())
                })
            }));
        }
        for h in handles {
            let got = h.join().unwrap();
            assert_eq!(got, Some(expected.clone()));
        }
        assert_eq!(
            counter.load(Ordering::SeqCst),
            1,
            "mutex-guarded get_or_compute must collapse N concurrent first-callers to a single canonicalize"
        );
    }

    /// Caching the workspace canonicalization does not change the escape
    /// verdict: a symlink **inside the workspace** that points outside is
    /// still flagged as an escape once the cache is populated.
    #[cfg(unix)]
    #[test]
    fn detect_workspace_escape_via_symlink_still_fires_with_cached_workspace() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = std::fs::canonicalize(tmp.path()).unwrap();
        let outside = tempfile::tempdir().unwrap();
        let outside_canonical = std::fs::canonicalize(outside.path()).unwrap();
        let trap = ws.join("trap");
        std::os::unix::fs::symlink(&outside_canonical, &trap).unwrap();

        // Prime the cache by detecting an inside path first.
        let inside = ws.join("inside");
        std::fs::create_dir(&inside).unwrap();
        assert_eq!(
            detect_workspace_escape(
                test_default_workspace_cache(),
                &inside,
                &ws,
                CwdEscapePolicy::WarnAndAllow
            ),
            EscapeKind::Inside
        );

        // The trap is lexically inside but resolves outside via symlink.
        // The cached workspace path must not mask the escape.
        assert_eq!(
            detect_workspace_escape(
                test_default_workspace_cache(),
                &trap,
                &ws,
                CwdEscapePolicy::WarnAndAllow
            ),
            EscapeKind::Escapes
        );
    }

    /// Under `Deny`, a joined path that cannot be canonicalized is not
    /// admitted.
    ///
    /// The symlink loop below is lexically inside the workspace, so the
    /// lexical check passes it, and `canonicalize` fails with `ELOOP`. A
    /// path whose resolution is unanswerable is exactly the case a
    /// fail-closed policy must refuse rather than collapse into "does not
    /// escape".
    #[cfg(unix)]
    #[test]
    fn deny_refuses_a_joined_path_that_cannot_be_canonicalized() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = std::fs::canonicalize(tmp.path()).unwrap();
        // A mutually-referential symlink pair: resolving either yields ELOOP.
        let a = ws.join("loop-a");
        let b = ws.join("loop-b");
        std::os::unix::fs::symlink(&b, &a).unwrap();
        std::os::unix::fs::symlink(&a, &b).unwrap();
        assert!(
            std::fs::canonicalize(&a).is_err(),
            "test fixture must produce an unresolvable path"
        );
        assert!(
            normalize_path(&a).starts_with(&ws),
            "the fixture must be lexically inside, so only the canonical check can catch it"
        );

        let cache = WorkspaceCanonicalCache::new();
        assert_eq!(
            detect_workspace_escape(&cache, &a, &ws, CwdEscapePolicy::Deny),
            EscapeKind::Escapes,
            "Deny must treat an unresolvable cwd as an escape, not as 'does not escape'"
        );
        // WarnAndAllow is advisory by design: it admits the same path.
        assert_eq!(
            detect_workspace_escape(&cache, &a, &ws, CwdEscapePolicy::WarnAndAllow),
            EscapeKind::Inside
        );
    }

    /// A `cwd` that simply does not exist is *not* an unresolvable path
    /// under `Deny`: it cannot be a symlink out of the workspace, and
    /// the spawn reports its own error — refusing it here would break every
    /// plan whose step directory is created by an earlier step.
    #[test]
    fn deny_still_admits_a_missing_but_contained_cwd() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path().to_path_buf();
        let missing = ws.join("not-created-yet");
        let cache = WorkspaceCanonicalCache::new();
        assert_eq!(
            detect_workspace_escape(&cache, &missing, &ws, CwdEscapePolicy::Deny),
            EscapeKind::Inside
        );
    }

    /// Under `Deny` the joined-path canonicalization is not memoised for the
    /// runner's lifetime: a symlink swapped *after* a first spawn of the same
    /// path is re-detected on the next spawn without anyone calling
    /// `invalidate`. A host cannot know a swap happened, so memoising the
    /// verdict would widen the TOCTOU window rather than act as a cache.
    #[cfg(unix)]
    #[test]
    fn deny_re_resolves_the_joined_path_on_every_spawn() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = std::fs::canonicalize(tmp.path()).unwrap();
        let outside = tempfile::tempdir().unwrap();
        let outside_canonical = std::fs::canonicalize(outside.path()).unwrap();

        // First spawn: `sub` is a real directory inside the workspace.
        let sub = ws.join("sub");
        std::fs::create_dir(&sub).unwrap();
        let cache = WorkspaceCanonicalCache::new();
        assert_eq!(
            detect_workspace_escape(&cache, &sub, &ws, CwdEscapePolicy::Deny),
            EscapeKind::Inside
        );

        // Swap it for a symlink pointing outside; no invalidate call.
        std::fs::remove_dir(&sub).unwrap();
        std::os::unix::fs::symlink(&outside_canonical, &sub).unwrap();
        assert_eq!(
            detect_workspace_escape(&cache, &sub, &ws, CwdEscapePolicy::Deny),
            EscapeKind::Escapes,
            "Deny must re-canonicalize per call; a first-spawn result must not decide later spawns"
        );
    }

    /// A fan of distinct `cwd` values sharing
    /// [`WORKSPACE_CANONICAL_CACHE_CAP`] with the workspace entry cannot
    /// change an escape outcome by evicting it: `Deny` does not read the
    /// cache at all, and under `WarnAndAllow` an eviction only costs a
    /// re-canonicalize that returns the same answer.
    #[cfg(unix)]
    #[test]
    fn cache_eviction_by_many_distinct_cwds_does_not_change_escape_outcomes() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = std::fs::canonicalize(tmp.path()).unwrap();
        let outside = tempfile::tempdir().unwrap();
        let outside_canonical = std::fs::canonicalize(outside.path()).unwrap();
        let trap = ws.join("trap");
        std::os::unix::fs::symlink(&outside_canonical, &trap).unwrap();
        let inside = ws.join("inside");
        std::fs::create_dir(&inside).unwrap();

        // A cache far smaller than the fan below, so eviction is certain.
        let cache = WorkspaceCanonicalCache::with_capacity(4);
        for policy in [CwdEscapePolicy::Deny, CwdEscapePolicy::WarnAndAllow] {
            assert_eq!(
                detect_workspace_escape(&cache, &inside, &ws, policy),
                EscapeKind::Inside
            );
            // Fan many distinct joined paths through the cache.
            for i in 0..32 {
                let p = ws.join(format!("fan{i}"));
                let _ = detect_workspace_escape(&cache, &p, &ws, policy);
            }
            assert_eq!(
                detect_workspace_escape(&cache, &trap, &ws, policy),
                EscapeKind::Escapes,
                "eviction must not mask a symlink escape under {policy:?}"
            );
            assert_eq!(
                detect_workspace_escape(&cache, &inside, &ws, policy),
                EscapeKind::Inside,
                "eviction must not invent an escape under {policy:?}"
            );
        }
    }

    /// Best-effort coverage of the symlink-swap window. Layout:
    /// `ws/sub` is initially a real directory inside the workspace, so the
    /// escape check passes. We then swap it for a symlink pointing outside
    /// the workspace and assert that, because `Deny` canonicalizes the
    /// returned path, the chdir target the kernel sees is the original
    /// in-workspace path — not the post-swap escape destination. The
    /// residual window above this (e.g. mount-over) is documented on
    /// `CwdEscapePolicy::Deny` rather than closed in code.
    #[cfg(unix)]
    #[test]
    fn deny_returns_canonical_path_to_shrink_toctou_window() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = std::fs::canonicalize(tmp.path()).unwrap();
        let escape_target = tempfile::tempdir().unwrap();
        let escape_target_canonical = std::fs::canonicalize(escape_target.path()).unwrap();
        let inside = ws.join("sub");
        std::fs::create_dir(&inside).unwrap();

        let vars = Variables::from_env(&ws).expect("UTF-8 path");
        let resolved = resolve_spec_cwd(
            test_default_workspace_cache(),
            Some(std::path::Path::new("sub")),
            &ws,
            &vars,
            CwdEscapePolicy::Deny,
        )
        .expect("sub is inside the workspace");
        assert_eq!(resolved, inside, "Deny should return the canonical path");

        // Simulate the swap that would race a real spawn: replace `sub`
        // with a symlink to a directory outside the workspace.
        std::fs::remove_dir(&inside).unwrap();
        std::os::unix::fs::symlink(&escape_target_canonical, &inside).unwrap();

        // The previously resolved path is the canonical in-workspace one;
        // a chdir to it does not re-traverse the symlink we just planted.
        // This is the protection the canonicalize-under-Deny step provides.
        assert_ne!(
            resolved, escape_target_canonical,
            "resolved path must not be the post-swap escape target"
        );
    }

    /// `WarnAndAllow` gets the same canonicalize-on-success narrowing as
    /// `Deny`. Both policies already pay the canonicalize cost in the escape
    /// check, and gating the symlink-free handoff on `Deny` would leave the
    /// interactive path uniquely exposed to a symlink swap between
    /// `detect_workspace_escape` and the spawn.
    #[cfg(unix)]
    #[test]
    fn warn_and_allow_canonicalizes_inside_workspace() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = std::fs::canonicalize(tmp.path()).unwrap();
        let real = ws.join("real");
        std::fs::create_dir(&real).unwrap();
        let link = ws.join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let vars = Variables::from_env(&ws).expect("UTF-8 path");
        let resolved = resolve_spec_cwd(
            test_default_workspace_cache(),
            Some(std::path::Path::new("link")),
            &ws,
            &vars,
            CwdEscapePolicy::WarnAndAllow,
        )
        .expect("inside-workspace symlink must be allowed under WarnAndAllow");
        assert_eq!(
            resolved, real,
            "WarnAndAllow must hand the kernel the symlink-free canonical path"
        );
    }

    /// A cached workspace canonicalization is refreshed once the entry is
    /// invalidated, so a symlink swap under a cached workspace path cannot
    /// keep serving a stale canonical destination — which would be an escape
    /// window opened by the cache itself.
    ///
    /// This test populates the cache against a symlink workspace path,
    /// swaps the symlink to point at a different real directory, calls
    /// `invalidate(...)` to mark the entry stale (the supported
    /// runner-scoped flow — see [`CommandRunner::invalidate_workspace_cache`]),
    /// and asserts that the next lookup re-runs canonicalize and observes
    /// the new target.
    #[cfg(unix)]
    #[test]
    fn workspace_canonical_cache_re_canonicalizes_after_symlink_swap_and_invalidate() {
        let target_a = tempfile::tempdir().unwrap();
        let target_b = tempfile::tempdir().unwrap();
        let canonical_a = std::fs::canonicalize(target_a.path()).unwrap();
        let canonical_b = std::fs::canonicalize(target_b.path()).unwrap();
        assert_ne!(
            canonical_a, canonical_b,
            "two tempdirs must canonicalize to distinct paths"
        );

        // Create a workspace symlink that initially points at target_a.
        let parent = tempfile::tempdir().unwrap();
        let workspace = parent.path().join("ws-link");
        std::os::unix::fs::symlink(&canonical_a, &workspace).unwrap();

        let cache = WorkspaceCanonicalCache::new();
        let first = cache
            .get_or_compute(&workspace, |p| std::fs::canonicalize(p))
            .expect("first canonicalize must succeed");
        assert_eq!(first, canonical_a, "first call resolves to target_a");

        // Without invalidation, a second call still hits the cache and
        // returns the memoised entry — this pins the dedup behaviour.
        std::fs::remove_file(&workspace).unwrap();
        std::os::unix::fs::symlink(&canonical_b, &workspace).unwrap();
        let stale = cache
            .get_or_compute(&workspace, |p| std::fs::canonicalize(p))
            .expect("cached entry survives without invalidation");
        assert_eq!(
            stale, canonical_a,
            "cache hit must return the original canonicalization until invalidated"
        );

        // After invalidation, the next call re-runs canonicalize and
        // observes the new target. This is the AC #3 contract.
        cache.invalidate(&workspace);
        let refreshed = cache
            .get_or_compute(&workspace, |p| std::fs::canonicalize(p))
            .expect("post-invalidate canonicalize must succeed");
        assert_eq!(
            refreshed, canonical_b,
            "after invalidate, the swapped symlink must be re-canonicalized to target_b"
        );
    }

    /// `CommandRunner::invalidate_workspace_cache` observably affects
    /// subsequent spawn-time canonicalize results, because the spawn path and
    /// the invalidate API share one runner-scoped cache instance rather than
    /// the invalidate API mutating a cache no escape decision reads.
    ///
    /// The test drives the runner cache through the same
    /// `detect_workspace_escape` entry point the spawn path uses and asserts
    /// the second call observes a re-canonicalize after invalidate.
    #[cfg(unix)]
    #[test]
    fn invalidate_workspace_cache_changes_subsequent_spawn_canonicalize() {
        // Two distinct on-disk targets behind a single workspace symlink.
        let target_a = tempfile::tempdir().unwrap();
        let target_b = tempfile::tempdir().unwrap();
        let canonical_a = std::fs::canonicalize(target_a.path()).unwrap();
        let canonical_b = std::fs::canonicalize(target_b.path()).unwrap();
        std::fs::create_dir(canonical_a.join("inside")).unwrap();
        std::fs::create_dir(canonical_b.join("inside")).unwrap();

        let parent = tempfile::tempdir().unwrap();
        let workspace = parent.path().join("ws-link");
        std::os::unix::fs::symlink(&canonical_a, &workspace).unwrap();

        // Spawn-path cache (the type the runner holds).
        let cache = WorkspaceCanonicalCache::new();

        // Prime: an "inside" join under the workspace symlink resolves to
        // target_a/inside, which starts_with(canonical_a) — Inside.
        let inside = workspace.join("inside");
        assert_eq!(
            detect_workspace_escape(&cache, &inside, &workspace, CwdEscapePolicy::WarnAndAllow),
            EscapeKind::Inside
        );

        // PERF-3 / TASK-1172: with joined-path canonicalize results also
        // cached under the same key shape, a fresh `inside2` path issued
        // *after* the symlink swap will canonicalize against the *stale*
        // workspace canonical (target_a), producing an Escapes
        // mis-classification — that's the AC-mandated equivalent of the
        // original "cache stale until invalidate" symptom.
        std::fs::create_dir(canonical_a.join("inside2")).unwrap();
        std::fs::create_dir(canonical_b.join("inside2")).unwrap();
        std::fs::remove_file(&workspace).unwrap();
        std::os::unix::fs::symlink(&canonical_b, &workspace).unwrap();
        let inside2 = workspace.join("inside2");
        assert_eq!(
            detect_workspace_escape(&cache, &inside2, &workspace, CwdEscapePolicy::WarnAndAllow),
            EscapeKind::Escapes,
            "stale workspace cache must mis-classify a freshly-issued path until invalidate"
        );

        // After invalidate, the spawn path observes the new canonical for
        // both the workspace and the joined-path entries underneath it
        // (TASK-1172 invalidate clears descendants too) and classifies as
        // inside again.
        cache.invalidate(&workspace);
        assert_eq!(
            detect_workspace_escape(&cache, &inside2, &workspace, CwdEscapePolicy::WarnAndAllow),
            EscapeKind::Inside,
            "invalidate must let the spawn path re-canonicalize and reclassify"
        );
    }

    /// The cache hard-caps residency, so a long-running embedder injecting
    /// many distinct workspace paths cannot grow the map without bound.
    #[test]
    fn workspace_canonical_cache_evicts_lru_at_cap() {
        let cap = 4;
        let cache = WorkspaceCanonicalCache::with_capacity(cap);
        let keys: Vec<PathBuf> = (0..cap + 2)
            .map(|i| PathBuf::from(format!("/tmp/ops-task-1063-key-{i}")))
            .collect();
        for k in &keys {
            cache.get_or_compute(k, |p| Ok(p.to_path_buf()));
        }
        let len = cache.inner.lock().unwrap().len();
        assert!(
            len <= cap,
            "cache residency {len} must be bounded by cap {cap}"
        );
    }

    /// `expand_err_to_io` does not leak the offending variable name into the
    /// user-facing message body. The full error (including `var_name`) is
    /// logged at debug for operator follow-up, while the rendered
    /// `io::Error` message stays generic so a `${OPS_TOKEN}` reference
    /// dropped into a `.ops.toml` cannot surface in a `StepFailed` message
    /// uploaded to CI.
    #[test]
    fn expand_err_to_io_does_not_leak_variable_name_in_message() {
        let err = ops_core::expand::ExpandError {
            var_name: "OPS_SECRET_TOKEN".to_string(),
            cause: std::env::VarError::NotPresent,
        };
        let io_err = expand_err_to_io(err);
        let msg = io_err.to_string();
        assert!(
            !msg.contains("OPS_SECRET_TOKEN"),
            "variable name leaked to user-facing message: {msg}"
        );
        assert_eq!(io_err.kind(), std::io::ErrorKind::InvalidInput);
        assert!(msg.contains("variable expansion failed"));
    }
}
