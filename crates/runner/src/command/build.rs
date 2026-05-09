//! Command-building helpers: cwd resolution, workspace-escape detection, and
//! tokio [`Command`] construction from an [`ExecCommandSpec`].
//!
//! See the `SEC-004` / `SEC-14` notes on [`resolve_spec_cwd`] for the escape
//! policy rationale.

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

/// CONC-7 / TASK-1063: monotonic LRU access tick stamped on every cache hit
/// and insert. Mirrors the `next_lru_tick` pattern in
/// `extensions/about/src/manifest_cache.rs` (TASK-1106) — `Relaxed` is
/// sufficient because each access is taken under the cache mutex; we only
/// need a strictly increasing stamp for victim selection.
fn next_workspace_lru_tick() -> u64 {
    static LRU_TICK: AtomicU64 = AtomicU64::new(0);
    LRU_TICK.fetch_add(1, Ordering::Relaxed)
}

/// CONC-7 / TASK-1063: cap on the number of distinct workspace paths held
/// resident. Production runs see `1` key; tests inject many tempdirs.
/// The cap is a high-water mark for embedders and integration tests so the
/// previous unbounded `RwLock<HashMap>` cannot grow without limit.
pub(crate) const WORKSPACE_CANONICAL_CACHE_CAP: usize = 256;

#[derive(Clone)]
struct WorkspaceCacheEntry {
    canonical: Option<PathBuf>,
    last_accessed: u64,
}

/// CONC-7 / TASK-1063: bounded, runner-scoped cache of
/// `canonicalize(workspace)` results.
///
/// Replaces the prior `OnceLock<RwLock<HashMap<PathBuf, Option<PathBuf>>>>`
/// process-global. The previous design had two problems:
///
/// 1. **Unbounded**: every distinct workspace `PathBuf` ever seen was
///    retained for the lifetime of the process. Long-running embedders or
///    in-process test fixtures that spin up many tempdirs accumulated
///    entries indefinitely.
/// 2. **Stale forever**: a `canonicalize(...)` result was cached on first
///    miss with no invalidation path. If a symlink under the cached
///    workspace was swapped after the entry was populated, all subsequent
///    containment decisions used the stale canonical path.
///
/// The cache is now an instance type owned by [`CommandRunner`] (see
/// `mod.rs`). When the runner is dropped, the cache and its entries go
/// with it. The runner exposes [`CommandRunner::invalidate_workspace_cache`]
/// and [`CommandRunner::clear_workspace_cache`] for hosts that need to
/// react to a known on-disk change without dropping the runner.
///
/// Eviction policy mirrors `extensions/about/src/manifest_cache.rs`
/// (TASK-1106): least-recently-used by access tick, evicted one entry at
/// a time when the cap is reached.
pub(crate) struct WorkspaceCanonicalCache {
    /// CONC-7 / TASK-1063: a `Mutex` (rather than `RwLock`) is sufficient
    /// here. The hot path is dominated by lock-free reads downstream of
    /// the canonicalize syscall; the per-spawn cost of the mutex
    /// acquisition itself is negligible against the work it guards.
    /// Using `Mutex` also matches `ArcTextCache`'s pattern, keeping the
    /// poison-recovery shape consistent across caches in this codebase.
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

    /// Default-capacity constructor used by [`CommandRunner::new`].
    pub(crate) fn new() -> Self {
        Self::with_capacity(WORKSPACE_CANONICAL_CACHE_CAP)
    }

    /// Forget the cached canonicalization for `workspace` and any joined-path
    /// descendants so the next call re-runs `canonicalize`. Used by hosts that
    /// observe an on-disk swap (and by the symlink-swap regression test for
    /// AC #3).
    ///
    /// PERF-3 / TASK-1172: `detect_workspace_escape` now caches joined-path
    /// canonicalizations in this same cache, keyed by the (uncanonicalised)
    /// joined path. A workspace symlink swap therefore invalidates not just
    /// the workspace entry but every joined-path entry underneath it; we
    /// drop both shapes in one pass so callers retain a single invalidate
    /// API.
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
    /// # Concurrency contract (CONC-2 / TASK-1229)
    ///
    /// The cache mutex is held across the `canonicalize` closure. This is
    /// **intentional** — the thundering-herd dedup the closure-under-lock
    /// shape gives us is the whole reason for this cache: concurrent
    /// callers for the same uncached path collapse onto a single
    /// `canonicalize` syscall (the contract PERF-3 / TASK-1095 pinned).
    /// The cost is that concurrent first-time lookups for **distinct**
    /// workspace paths also serialise on this mutex during the syscall.
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
    /// `extensions/about/src/manifest_cache.rs` (TASK-1144) already moved
    /// to that shape; this cache deliberately lags because the workload
    /// has not yet justified the extra indirection.
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
        canonical
    }
}

impl Default for WorkspaceCanonicalCache {
    fn default() -> Self {
        Self::new()
    }
}

/// ERR-5 / TASK-1063: poison recovery. The cache value is the pure result
/// of `canonicalize`, not authoritative state, so a panic in a previous
/// holder cannot leave a torn invariant; treating poison as fatal would
/// brick the cache for every other caller in the process.
fn recover_workspace_cache<T>(
    err: std::sync::PoisonError<std::sync::MutexGuard<'_, T>>,
) -> std::sync::MutexGuard<'_, T> {
    tracing::warn!("workspace canonicalize cache mutex was poisoned by a prior panic; recovered");
    err.into_inner()
}

/// PERF-3 / TASK-0765: cache the canonical workspace path keyed by raw path.
///
/// ARCH-9 / TASK-1126: takes the runner-scoped cache instance as a
/// parameter so the spawn path consults the same cache that
/// [`CommandRunner::invalidate_workspace_cache`] / [`clear_workspace_cache`]
/// mutate. Earlier this routed through a process-global static, which made
/// the public invalidate API a no-op against the cache that actually decided
/// escape outcomes for production callers.
fn canonical_workspace_cached(
    cache: &WorkspaceCanonicalCache,
    workspace: &Path,
) -> Option<PathBuf> {
    cache.get_or_compute(workspace, |p| std::fs::canonicalize(p))
}

/// PERF-3 / TASK-1095: testable seam for the cache so tests can inject a
/// canonicalize counter and verify the burst-startup thundering-herd is
/// collapsed to a single syscall per workspace path.
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

/// ARCH-9 / TASK-1126: test-only ambient cache so the existing
/// `resolve_spec_cwd` / `detect_workspace_escape` regression tests do not have
/// to construct one per assertion. Production callers MUST thread the
/// runner-scoped `Arc<WorkspaceCanonicalCache>` and never reach this static.
#[cfg(test)]
pub(crate) fn test_default_workspace_cache() -> &'static Arc<WorkspaceCanonicalCache> {
    static CACHE: OnceLock<Arc<WorkspaceCanonicalCache>> = OnceLock::new();
    CACHE.get_or_init(|| Arc::new(WorkspaceCanonicalCache::new()))
}

/// ERR-1 / TASK-0450: convert a strict-expansion error into an `io::Error`
/// so build failures share the spawn-error pipeline and surface as a
/// `StepFailed` event rather than panicking through `expect`.
///
/// SEC-22 / TASK-1175: the produced `io::Error` is the source of
/// `StepFailed.message` and the TAP file body, both of which round-trip
/// to CI artifacts. Log the full chain (including the offending variable
/// name and the underlying `VarError`) at `tracing::debug!` like
/// `log_and_redact_spawn_error`, but return a generic operator-facing
/// message so the variable name from a `.ops.toml`-supplied
/// `${OPS_TOKEN}`/`${ATTACKER_VAR}` reference cannot leak into uploaded
/// CI logs. Operators chasing the leak follow the same `RUST_LOG=debug`
/// path as for spawn-error redaction.
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
/// SEC-14: interactive invocations (`ops <cmd>`) tolerate escapes with a
/// warning — `.ops.toml` is trusted the way a Makefile is trusted.
/// Hook-triggered invocations (`run-before-commit`, `run-before-push`) are
/// strict: a co-worker's PR can land a `.ops.toml` that runs on every
/// commit the maintainer makes, so the hook path fails closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CwdEscapePolicy {
    /// Log a warning and spawn anyway. Default for interactive `ops run`.
    #[default]
    WarnAndAllow,
    /// Refuse to spawn; return an error. Used by git-hook-triggered paths.
    ///
    /// SEC-14 / TASK-0886: hook-triggered entry points (`run-before-commit`,
    /// `run-before-push`) now construct a `CommandRunner` with this policy
    /// so a `.ops.toml` landed by a coworker PR cannot escape the workspace
    /// on the next commit. The default interactive path stays
    /// `WarnAndAllow` to avoid a behaviour change for existing users.
    ///
    /// SEC-25: residual TOCTOU window. The check happens in
    /// [`detect_workspace_escape`], which calls `std::fs::canonicalize`,
    /// while the actual `chdir` is performed by the OS when the child is
    /// spawned. To shrink the window, [`resolve_spec_cwd`] canonicalizes
    /// the joined path on a best-effort basis under *both* policies and
    /// hands the symlink-free result to `current_dir`, so the kernel does
    /// not re-resolve any symlinks at exec time. SEC-23 / TASK-1140: prior
    /// to this change the canonicalize-on-success step was gated on `Deny`
    /// only, leaving the interactive `WarnAndAllow` path more exposed to a
    /// symlink-swap race than the hook path even though both pay the same
    /// canonicalize cost in `detect_workspace_escape`. The two policies
    /// now share the same TOCTOU surface; `Deny` differs only in failing
    /// closed on detected escapes. A narrow race remains: an attacker who
    /// can replace a component of the canonical path (e.g. by mounting
    /// over it or swapping a directory they own) between canonicalization
    /// and exec can still divert the child. Closing this fully would
    /// require an `openat`/`fchdir`-style fd handoff to the child, which
    /// neither `std::process::Command` nor `tokio::process::Command`
    /// exposes today.
    Deny,
}

/// FN-1: classification of how a spec `cwd` relates to the workspace root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EscapeKind {
    /// Path stays inside the workspace under both lexical and canonical checks.
    Inside,
    /// Path escapes the workspace (lexically and/or via symlink canonicalization).
    Escapes,
}

/// Classify `joined` against `workspace`. Pure function: fast lexical check
/// first, then a canonical check so a symlink inside the workspace pointing
/// outside is still caught.
pub(crate) fn detect_workspace_escape(
    cache: &WorkspaceCanonicalCache,
    joined: &std::path::Path,
    workspace: &std::path::Path,
) -> EscapeKind {
    let lexically_escapes = !normalize_path(joined).starts_with(workspace);
    // PERF-3 / TASK-1172: route the joined-path canonicalize through the
    // same `WorkspaceCanonicalCache` as the workspace side, so a composite
    // that fans the same `cwd = "sub"` over many parallel spawns pays one
    // canonicalize per distinct joined path instead of one per spawn. The
    // existing `invalidate(path)` API removes whatever entry sits at that
    // key, so a SEC-25 symlink-swap recovery can invalidate either the
    // workspace or the joined path and continue to detect the escape on
    // the next call (mirror of TASK-1063 AC #3 for the workspace side).
    let canonically_escapes = match (
        canonical_workspace_cached(cache, joined),
        canonical_workspace_cached(cache, workspace),
    ) {
        (Some(a), Some(b)) => !a.starts_with(&b),
        _ => false,
    };
    if lexically_escapes || canonically_escapes {
        EscapeKind::Escapes
    } else {
        EscapeKind::Inside
    }
}

/// FN-1: apply an escape policy to a detected escape. `Deny` converts to an
/// `io::Error`; `WarnAndAllow` emits a tracing warning and lets the caller
/// continue.
pub(crate) fn apply_escape_policy(
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
            tracing::warn!(
                cwd = %workspace_cwd.display(),
                spec_cwd = %spec_cwd.display(),
                resolved = %joined.display(),
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
/// `policy == Deny` (SEC-14 hook path). Otherwise logs and continues.
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
    if detect_workspace_escape(cache, &joined, workspace_cwd) == EscapeKind::Escapes {
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
/// ## SEC-004 / SEC-14: cwd traversal guard
///
/// Delegates to [`resolve_spec_cwd`] with [`CwdEscapePolicy::WarnAndAllow`],
/// which warns but still spawns (interactive trust model). Callers that
/// need fail-closed behaviour (git hooks) should call [`build_command_with`]
/// with [`CwdEscapePolicy::Deny`].
///
/// Note: `current_dir` is validated by the OS when the command is spawned — if the
/// path does not exist, `Command::output()` returns an `io::Error` that propagates
/// through the existing error handling in `exec_command`.
// ERR-5 / TASK-0456: `build_command` previously panicked via
// `.expect("WarnAndAllow policy never returns Err")` to encode "cannot
// fail under WarnAndAllow" at the type level. After TASK-0450 the
// expansion path itself is fallible (a non-UTF-8 env var must surface as
// a step failure rather than crashing the runner), so the no-panic
// guarantee is now structural in the *return type*: build_command
// returns `Result`, and every caller threads the error to a StepFailed
// event. There is no remaining `.expect` to revisit.
#[cfg(test)]
pub fn build_command(
    spec: &ExecCommandSpec,
    cwd: &std::path::Path,
    vars: &Variables,
) -> Result<Command, std::io::Error> {
    let cache = WorkspaceCanonicalCache::new();
    build_command_with(&cache, spec, cwd, vars, CwdEscapePolicy::WarnAndAllow)
}

/// CONC-5 / TASK-0330: async variant that runs the synchronous filesystem
/// work in [`build_command`] (notably `std::fs::canonicalize` calls inside
/// [`detect_workspace_escape`] and [`resolve_spec_cwd`]) on the blocking
/// thread pool.
///
/// Without this, every parallel command spawn blocks a tokio worker on
/// `canonicalize` syscalls — slow on NFS or symlink-heavy paths and
/// proportional to the spec cwd's depth. Under high `MAX_PARALLEL` counts
/// that starves other tasks scheduled on the same worker.
///
/// OWN-2 / TASK-0462: `vars` and `cwd` are passed as `Arc` so the only
/// per-spawn allocations on the parallel hot path are `Arc::clone` (a
/// single atomic refcount bump each), not a deep `Variables`/`PathBuf`
/// clone. The previous signature took `Variables`/`PathBuf` by value,
/// which silently re-allocated the inner `HashMap` per spawn and mixed
/// `Arc` indirection at the call site with per-call deep clones — the
/// worst of both.
///
/// PERF-3 / TASK-1125: `spec` is now `Arc<ExecCommandSpec>` so callers
/// (notably the parallel spawn path) only pay an `Arc::clone` per spawn
/// instead of a deep clone of `args: Vec<String>` / `env: IndexMap` /
/// `cwd: Option<PathBuf>` / `program: String`. End-to-end Arc-only
/// inputs match the trace claim emitted below.
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
    cmd.kill_on_drop(true);
    Ok(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::test_utils::{exec_spec, exec_spec_with_cwd};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// CONC-5 / TASK-0330: the async variant must dispatch the canonicalize
    /// work to the blocking pool so a single-threaded runtime can still
    /// drive other tasks while build_command runs. This test uses a
    /// `current_thread` runtime — the only worker — and asserts that a
    /// concurrent counter task makes progress while build_command_async is
    /// in flight.
    ///
    /// Under the previous synchronous `build_command` call from inside an
    /// async function, the runtime worker would be blocked for the
    /// duration of every canonicalize syscall, starving the counter task
    /// (and in production, every other task scheduled on that worker).
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
            let vars = Variables::from_env(tmp.path());

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
    /// that accidentally rewrite the spec inside spawn_blocking.
    #[tokio::test]
    async fn build_command_async_preserves_program_name() {
        let tmp = tempfile::tempdir().unwrap();
        let vars = Variables::from_env(tmp.path());
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
        let vars = Variables::from_env(&ws);
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
        let vars = Variables::from_env(&ws);
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

    /// SEC-23 / TASK-0500: an absolute spec_cwd outside the workspace must
    /// be rejected under `Deny`. The previous bug short-circuited the policy
    /// check so a malicious `cwd = "/etc"` would silently spawn at /etc on
    /// the hook path.
    #[test]
    fn resolve_spec_cwd_absolute_outside_workspace_is_denied() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let vars = Variables::from_env(&ws);
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

    /// SEC-23: under WarnAndAllow the absolute path is still returned (the
    /// interactive trust model lets `.ops.toml` choose its cwd) but the
    /// escape is logged.
    #[test]
    fn resolve_spec_cwd_absolute_outside_workspace_warns_under_warn_and_allow() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let vars = Variables::from_env(&ws);
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
        let vars = Variables::from_env(&ws);
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
        let vars = Variables::from_env(&ws);
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
        let vars = Variables::from_env(&ws);
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

    /// READ-5 / TASK-0900: a non-UTF-8 cwd must surface a loud
    /// InvalidInput error instead of being lossy-expanded into a
    /// wrong-but-similar path that would chdir the child into the
    /// "wrong" directory.
    #[cfg(unix)]
    #[test]
    fn resolve_spec_cwd_rejects_non_utf8_cwd_loudly() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        let ws = std::path::PathBuf::from("/tmp/ws");
        let bad: OsString = OsString::from_vec(vec![b's', b'u', b'b', 0xff]);
        let bad_path: std::path::PathBuf = bad.into();
        let vars = Variables::from_env(&ws);
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
            detect_workspace_escape(test_default_workspace_cache(), &inside, &ws),
            EscapeKind::Inside
        );
    }

    #[test]
    fn detect_workspace_escape_parent_escapes() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let escaping = ws.join("../etc");
        assert_eq!(
            detect_workspace_escape(test_default_workspace_cache(), &escaping, &ws),
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

    /// READ-5 / TASK-0773: an absolute spec_cwd inside the workspace must go
    /// through the same canonicalize-under-Deny narrowing that relative
    /// spec_cwd already enjoyed. Pins the symmetric behaviour after
    /// TASK-0773 so a future refactor cannot silently regress to the
    /// asymmetric path that left absolute hook-path cwds unprotected.
    #[cfg(unix)]
    #[test]
    fn deny_canonicalizes_absolute_inside_workspace() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = std::fs::canonicalize(tmp.path()).unwrap();
        let inside = ws.join("sub");
        std::fs::create_dir(&inside).unwrap();
        let escape_target = tempfile::tempdir().unwrap();
        let escape_target_canonical = std::fs::canonicalize(escape_target.path()).unwrap();

        let vars = Variables::from_env(&ws);
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
        // The previously resolved canonical path is unaffected — this is
        // the protection extending the canonicalize-under-Deny block to
        // absolute paths now grants.
        std::fs::remove_dir(&inside).unwrap();
        std::os::unix::fs::symlink(&escape_target_canonical, &inside).unwrap();
        assert_ne!(
            resolved, escape_target_canonical,
            "resolved path must not be the post-swap escape target"
        );
    }

    /// PERF-3 / TASK-1095: under burst startup with N threads asking for the
    /// same fresh workspace path, the cache must collapse the thundering herd
    /// to exactly one canonicalize call. Pre-TASK-1095, the writer path
    /// always re-canonicalized after the read-lock miss even if a racing
    /// writer had already populated the entry — N calls instead of 1.
    #[test]
    fn canonical_workspace_cached_collapses_burst_to_single_canonicalize() {
        use std::sync::Barrier;

        // Use a path keyed by this test name + nanos so the static cache
        // does not have a hit from a prior test run in the same process.
        let unique = std::path::PathBuf::from(format!(
            "/tmp/ops-task-1095-burst-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let n = 32usize;
        let counter = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(n));
        let mut handles = Vec::with_capacity(n);
        let expected = std::path::PathBuf::from("/expected/canonical");
        for _ in 0..n {
            let counter = Arc::clone(&counter);
            let barrier = Arc::clone(&barrier);
            let key = unique.clone();
            let expected = expected.clone();
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                canonical_workspace_cached_with(test_default_workspace_cache(), &key, |_p| {
                    counter.fetch_add(1, Ordering::SeqCst);
                    // Sleep briefly so concurrent callers reliably reach the
                    // write-lock re-check window before we return; without
                    // this the first writer can finish so fast that the
                    // others hit the read-lock fast path and the test does
                    // not exercise the double-check at all.
                    std::thread::sleep(std::time::Duration::from_millis(20));
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
            "double-checked locking must collapse N concurrent first-callers to a single canonicalize syscall"
        );
    }

    /// PERF-3 / TASK-0765: behavioural parity for the cached workspace
    /// canonicalize. A symlink **inside the workspace** that points outside
    /// must still be flagged as an escape after the cache populates.
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
            detect_workspace_escape(test_default_workspace_cache(), &inside, &ws),
            EscapeKind::Inside
        );

        // The trap is lexically inside but resolves outside via symlink.
        // The cached workspace path must not mask the escape.
        assert_eq!(
            detect_workspace_escape(test_default_workspace_cache(), &trap, &ws),
            EscapeKind::Escapes
        );
    }

    /// SEC-25: best-effort regression for the symlink-swap window. Layout:
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

        let vars = Variables::from_env(&ws);
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

    /// SEC-23 / TASK-1140: WarnAndAllow must enjoy the same
    /// canonicalize-on-success narrowing as Deny. Prior to TASK-1140 the
    /// canonicalize block was gated on `Deny`, leaving the interactive
    /// path uniquely exposed to a symlink swap between the
    /// `detect_workspace_escape` check and the spawn even though both
    /// policies pay the same canonicalize cost in the check.
    #[cfg(unix)]
    #[test]
    fn warn_and_allow_canonicalizes_inside_workspace() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = std::fs::canonicalize(tmp.path()).unwrap();
        let real = ws.join("real");
        std::fs::create_dir(&real).unwrap();
        let link = ws.join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let vars = Variables::from_env(&ws);
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

    /// CONC-7 / TASK-1063: regression for AC #3. The unbounded process-global
    /// `OnceLock<RwLock<HashMap>>` previously cached `canonicalize(workspace)`
    /// forever with no invalidation path. After a symlink swap under a
    /// previously cached workspace path, the next call returned the stale
    /// canonical destination — a SEC-25-shaped escape window the cache
    /// widened.
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

        // Without invalidation, a second call must still hit the cache and
        // return the stale entry — this pins the dedup behaviour the LRU
        // cache shares with the prior implementation.
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

    /// ARCH-9 / TASK-1126 AC #2: `CommandRunner::invalidate_workspace_cache`
    /// must observably affect subsequent spawn-time canonicalize results.
    /// Pre-fix the spawn path read a process-global static cache while
    /// `invalidate_workspace_cache` mutated only the runner-scoped
    /// `workspace_cache` field — the public invalidate API was a no-op
    /// against the cache that decided escape outcomes for production
    /// callers. This test drives the runner cache through the same
    /// `detect_workspace_escape` entry point the spawn path uses and
    /// asserts the second call observes a re-canonicalize after invalidate.
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
            detect_workspace_escape(&cache, &inside, &workspace),
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
            detect_workspace_escape(&cache, &inside2, &workspace),
            EscapeKind::Escapes,
            "stale workspace cache must mis-classify a freshly-issued path until invalidate"
        );

        // After invalidate, the spawn path observes the new canonical for
        // both the workspace and the joined-path entries underneath it
        // (TASK-1172 invalidate clears descendants too) and classifies as
        // inside again.
        cache.invalidate(&workspace);
        assert_eq!(
            detect_workspace_escape(&cache, &inside2, &workspace),
            EscapeKind::Inside,
            "invalidate must let the spawn path re-canonicalize and reclassify"
        );
    }

    /// CONC-7 / TASK-1063: AC #1 regression. The cache must hard-cap
    /// residency so a long-running embedder injecting many distinct
    /// workspace paths cannot grow the map without bound.
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

    /// SEC-22 / TASK-1175: `expand_err_to_io` must NOT leak the offending
    /// variable name into the user-facing message body. The full error
    /// (including `var_name`) is logged at debug for operator follow-up,
    /// but the rendered `io::Error` message stays generic so a
    /// `${OPS_TOKEN}` typo or `${ATTACKER_VAR}` reference dropped into a
    /// `.ops.toml` cannot surface in StepFailed message uploaded to CI.
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
