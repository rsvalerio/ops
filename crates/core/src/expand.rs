//! Variable expansion for command specs.
//!
//! Expands `$VAR`, `${VAR}`, `${VAR:-default}`, and `~` in strings using
//! built-in variables and environment fallback via `shellexpand`.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// ERR-1 / TASK-0450: a non-recoverable variable expansion failure.
///
/// Returned from [`Variables::try_expand`] when `shellexpand` reports an
/// error such as `VarError::NotUnicode` — the underlying env var exists but
/// cannot be read as UTF-8, so the literal `${VAR}` would otherwise flow
/// through unchanged into argv / cwd / env values. Strict callers (the
/// command-build path) propagate this so the failure is visible instead of
/// materialising a literal `${VAR}` path on disk.
#[derive(Debug, Clone)]
pub struct ExpandError {
    pub var_name: String,
    pub cause: std::env::VarError,
}

impl fmt::Display for ExpandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "variable expansion failed for `${}`: {}",
            self.var_name, self.cause
        )
    }
}

impl std::error::Error for ExpandError {
    // ERR-7 / TASK-0835: expose the underlying VarError so callers and
    // tracing formatters can walk the chain via `{:#}` / `Error::source`,
    // instead of getting a flattened string snapshot.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.cause)
    }
}

/// Built-in variables available for expansion in command specs.
///
/// Lookup order: built-in variables first, then `std::env::var()` fallback.
///
/// # Behaviour when a variable is missing
///
/// The closure returns `Ok(None)` for a variable that is neither in the
/// builtins map nor in the process environment. `shellexpand` handles
/// `Ok(None)` itself by leaving the reference (e.g. `$UNDEFINED`) literal
/// in the output rather than emitting an empty string, and the
/// `${VAR:-default}` syntax still resolves to the default.
///
/// A genuine `Err(VarError)` from the lookup (e.g. `VarError::NotUnicode`
/// for a non-UTF-8 env value) is *not* the same as a missing variable.
/// We log such errors at `tracing::warn!` with the offending variable
/// name (ERR-1) before falling back to `Cow::Borrowed(input)`, so config
/// bugs are visible instead of silently passing through unchanged.
#[derive(Debug, Clone)]
pub struct Variables {
    builtins: HashMap<&'static str, Arc<str>>,
}

/// Cached `std::env::temp_dir()` rendering. Computed once per process: the
/// value depends on `TMPDIR` / OS defaults that do not change after startup,
/// and `temp_dir()` itself performs a syscall on Unix. Reused across every
/// `Variables::from_env` call so command-spec expansion avoids the syscall +
/// allocation on every invocation.
///
/// PERF-3 / TASK-0967: stored as `Arc<str>` so `from_env` can hand out a
/// reference-counted clone in O(1) instead of allocating a fresh `String`
/// on every call. CLI entry / hooks / RunBeforeCommit invoke `from_env`
/// outside the parallel-runtime Arc-cloning boundary, so amortised
/// per-call allocation matters here.
///
/// READ-5 / TASK-1068: **process-lifetime contract.** This `OnceLock` is
/// populated on the first `Variables::from_env` call and never refreshed.
/// Any later `std::env::set_var("TMPDIR", ...)` is **invisible** to
/// subsequent `Variables::from_env` callers — they will keep observing the
/// value captured at first init. Tests (or any code) that need to swap
/// `TMPDIR` and have `Variables` see the new value MUST do so *before* the
/// first `from_env` call in the process; otherwise the swap is silently
/// shadowed. The cache is intentional (see `from_env_reuses_cached_tmpdir_arc`
/// and the `tmpdir_swap_after_from_env_is_not_observed` regression test).
static TMPDIR_DISPLAY: std::sync::OnceLock<Arc<str>> = std::sync::OnceLock::new();

/// PERF-3 / TASK-1183: per-`ops_root` cache for the rendered `OPS_ROOT`
/// value so repeat `from_env` calls hand out an `Arc::clone` instead of
/// re-allocating the rendered `String` and a fresh `Arc<str>` inner.
/// `from_env` is invoked from hooks (`run-before-commit`,
/// `run-before-push`), about-card refreshes, and dry-run.
///
/// Stored as a `HashMap` rather than an LRU-of-1 so concurrent callers
/// using different `ops_root` paths (notably tests running in parallel) do
/// not evict each other and force allocation churn. In production the
/// dominant pattern is a single stable root per process, so the map stays
/// at one entry; embedders that legitimately switch roots get their
/// rendered values memoised independently.
///
/// CONC-1 / TASK-1418: bounded by [`OPS_ROOT_CACHE_CAP`] with FIFO
/// eviction so a long-lived embedder (or test binary running thousands of
/// distinct workspace roots) cannot grow the map without bound. The cap
/// is comfortably above any realistic workload — a single process working
/// across >64 distinct workspace roots is pathological — so eviction only
/// fires under adversarial input.
struct OpsRootCache {
    map: HashMap<PathBuf, Arc<str>>,
    order: VecDeque<PathBuf>,
}

/// CONC-1 / TASK-1418: maximum number of distinct workspace roots cached
/// before evicting the oldest. See [`OpsRootCache`] for the rationale.
pub(crate) const OPS_ROOT_CACHE_CAP: usize = 64;

fn ops_root_cache() -> &'static Mutex<OpsRootCache> {
    static CACHE: std::sync::OnceLock<Mutex<OpsRootCache>> = std::sync::OnceLock::new();
    CACHE.get_or_init(|| {
        Mutex::new(OpsRootCache {
            map: HashMap::new(),
            order: VecDeque::new(),
        })
    })
}

/// Resolve the `OPS_ROOT` `Arc<str>` for a workspace root, caching the
/// rendered value and canonicalising the key so equivalent paths collapse.
///
/// ERR-1 / TASK-1424: the cache key is the canonicalised path when
/// `std::fs::canonicalize` succeeds, so two semantically equal roots
/// (`./project` vs `/abs/project`, symlinked vs canonical) yield the same
/// `Arc<str>` and the same `OPS_ROOT` substitution. `canonicalize` requires
/// the path to exist; when it fails (synthetic test paths, freshly-staged
/// repos that have not yet been written to disk) we fall back to the raw
/// `Path` so callers still get a stable Arc within the process.
fn cached_ops_root_arc(ops_root: &Path) -> Result<Arc<str>, ExpandError> {
    // CONC-1 / TASK-1183: lock contention is negligible because `from_env`
    // is not called from the parallel exec hot loop (that path constructs
    // `Variables` once and clones the Arc from the runner).
    //
    // PERF-3 / TASK-1465: probe the cache by raw `&Path` first; the
    // canonicalize syscall now runs **only on miss** to install the alias
    // entry. The hit path therefore costs one lock + hashmap lookup with no
    // `realpath(2)` traversal — relevant for hook callers
    // (`run-before-commit`/`run-before-push`) and dry-run that invoke
    // `from_env` repeatedly on the same workspace root.
    let cache = ops_root_cache();
    // CONC-9 / TASK-1183 + DUP-3 / TASK-1477: lock poisoning here is
    // recoverable — every state of the protected map is valid. Centralised
    // in `sync::lock_recover` so the policy stays uniform across the four
    // expand/detect sites that share it.
    let guard = crate::sync::lock_recover(cache);
    if let Some(arc) = guard.map.get(ops_root) {
        return Ok(Arc::clone(arc));
    }
    // Miss: now pay the canonicalize syscall (so symlink + raw paths still
    // collapse onto a single cached value) and install both keys.
    drop(guard);
    // TEST-15 / TASK-1465: increment immediately before the syscall so the
    // counter measures exactly what the hit path must avoid.
    #[cfg(test)]
    record_canonicalize_call(ops_root);
    let canonical = std::fs::canonicalize(ops_root).ok();
    let key: &Path = canonical.as_deref().unwrap_or(ops_root);
    // ERR-1 / TASK-1462: refuse to lossy-render a non-UTF-8 workspace
    // root through `Path::display()` — that path silently substitutes
    // U+FFFD and the resulting `Arc<str>` flows into argv/cwd/env through
    // `try_expand`, defeating the strict-expand contract (TASK-0450).
    // Surface as `ExpandError::NotUnicode` so callers see the failure
    // instead of materialising a corrupt OPS_ROOT.
    let rendered = key.to_str().ok_or_else(|| ExpandError {
        var_name: "OPS_ROOT".to_string(),
        cause: std::env::VarError::NotUnicode(key.as_os_str().to_owned()),
    })?;
    let mut guard = crate::sync::lock_recover(cache);
    // Re-check after the syscall window — another thread may have
    // populated the entry under the same raw key.
    if let Some(existing) = guard.map.get(ops_root) {
        return Ok(Arc::clone(existing));
    }
    // ERR-1 / TASK-1424: if the canonical form already lives in the cache
    // (the alias-by-canonical scenario — e.g. a previous call resolved
    // the real path directly), reuse that `Arc<str>` so a real + symlink
    // pair collapses to one rendering. Without this branch the new entry
    // would shadow the existing one and break `Arc::ptr_eq`.
    let arc = if let Some(existing) = guard.map.get(key) {
        Arc::clone(existing)
    } else {
        Arc::<str>::from(rendered)
    };
    // CONC-1 / TASK-1418: evict the oldest entry when at cap so the new
    // distinct root still fits.
    if guard.map.len() >= OPS_ROOT_CACHE_CAP {
        if let Some(oldest) = guard.order.pop_front() {
            guard.map.remove(&oldest);
        }
    }
    let raw_owned = ops_root.to_path_buf();
    guard.map.insert(raw_owned.clone(), Arc::clone(&arc));
    guard.order.push_back(raw_owned);
    // Also install the canonical key (when distinct) so the next caller
    // that hands us the canonical form hits without re-canonicalising.
    let canon_owned = key.to_path_buf();
    if canon_owned != ops_root && !guard.map.contains_key(&canon_owned) {
        if guard.map.len() >= OPS_ROOT_CACHE_CAP {
            if let Some(oldest) = guard.order.pop_front() {
                guard.map.remove(&oldest);
            }
        }
        guard.map.insert(canon_owned.clone(), Arc::clone(&arc));
        guard.order.push_back(canon_owned);
    }
    Ok(arc)
}

#[cfg(test)]
pub(crate) fn ops_root_cache_len() -> usize {
    // ERR-1 / TASK-1474: test seams must surface mutex poison rather than
    // silently returning a "successful" count. `lock_recover_warn` clears
    // the poison and emits a `tracing::warn!` so a flake caused by a
    // sibling panic shows up at the right level instead of looking like a
    // later value-mismatch.
    let cache = ops_root_cache();
    crate::sync::lock_recover_warn(cache, "ops_root_cache_len")
        .map
        .len()
}

#[cfg(test)]
pub(crate) fn reset_ops_root_cache() {
    let cache = ops_root_cache();
    // DUP-3 / TASK-1477: shared poison-recover policy.
    let mut guard = crate::sync::lock_recover(cache);
    guard.map.clear();
    guard.order.clear();
}

/// TEST-15 / TASK-1465: counts `std::fs::canonicalize` calls issued by
/// [`cached_ops_root_arc`].
///
/// The hit-path contract ("a warm `from_env` must not reach `realpath(2)`")
/// used to be asserted by timing 200k calls against a one-second budget. That
/// is a wall-clock assertion in a debug build: it passes on an idle machine and
/// fails under CPU contention, which is exactly the condition a CI runner is
/// in. Counting the syscall proves the same property deterministically, in a
/// thousandth of the iterations, and is immune to how loaded the host is.
///
/// Mirrors the existing seam style (`ops_root_cache_len`,
/// `stack::detect::canonicalize_cache_contains`).
///
/// Keyed **per path** rather than a single global counter. Test binaries run
/// their tests in parallel threads and many of them call `from_env`, so a
/// process-wide counter is incremented by unrelated tests between a reader's
/// two observations — measured as a spurious delta of 3 under load. Keying by
/// the root being canonicalised makes each test observe only its own synthetic
/// path, which no other test uses.
#[cfg(test)]
fn canonicalize_calls() -> &'static Mutex<std::collections::HashMap<PathBuf, usize>> {
    static CALLS: OnceLock<Mutex<std::collections::HashMap<PathBuf, usize>>> = OnceLock::new();
    CALLS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

#[cfg(test)]
fn record_canonicalize_call(path: &Path) {
    *crate::sync::lock_recover(canonicalize_calls())
        .entry(path.to_path_buf())
        .or_insert(0) += 1;
}

#[cfg(test)]
pub(crate) fn canonicalize_call_count(path: &Path) -> usize {
    crate::sync::lock_recover(canonicalize_calls())
        .get(path)
        .copied()
        .unwrap_or(0)
}

/// PERF-3 / TASK-1411 + CONC-1 / TASK-1444: maximum number of distinct
/// `var_name` entries [`expand_warn_seen`] holds before evicting the
/// oldest. The dedup contract is "warn-once-per-distinct-name", so a stream
/// of adversarial distinct names (e.g. a CI matrix that emits
/// `OPS__FOO_<n>` across thousands of values) cannot grow the set without
/// bound. The cap is comfortably above any realistic command-spec workload
/// (>= 64 distinct variable names is unrealistic) so eviction only fires
/// under pathological input. Eviction is FIFO over insertion order — when
/// at cap a fresh `var_name` displaces the oldest entry so the new distinct
/// name still surfaces a single user-facing warn.
pub(crate) const EXPAND_WARN_SEEN_CAP: usize = 256;

/// ERR-1 / TASK-1224 + TASK-1411 / TASK-1444: track the set of variable
/// names for which [`Variables::expand`] has already surfaced a user-facing
/// warning, so repeated failures on the same variable do not flood stderr.
/// The underlying `tracing::warn!` continues to fire for every failure, so
/// structured-log consumers still see the full sequence.
///
/// Bounded by [`EXPAND_WARN_SEEN_CAP`]. The `VecDeque<String>` holds
/// insertion order so the oldest entry can be evicted in O(1) when the set
/// is at capacity; the `HashSet` mirrors the same names for O(1) membership.
/// Drop-on-full eviction is acceptable because the protected state is a
/// deduplication hint, not a correctness invariant.
struct ExpandWarnSeen {
    set: HashSet<String>,
    order: VecDeque<String>,
}

impl ExpandWarnSeen {
    fn new() -> Self {
        Self {
            set: HashSet::new(),
            order: VecDeque::new(),
        }
    }

    /// Insert `var_name`; evict the oldest entry first if at capacity.
    /// Returns `true` when the name was newly inserted (caller should emit
    /// the one-shot user-facing warn).
    fn insert(&mut self, var_name: &str) -> bool {
        if self.set.contains(var_name) {
            return false;
        }
        if self.set.len() >= EXPAND_WARN_SEEN_CAP {
            if let Some(oldest) = self.order.pop_front() {
                self.set.remove(&oldest);
            }
        }
        self.set.insert(var_name.to_string());
        self.order.push_back(var_name.to_string());
        true
    }
}

fn expand_warn_seen() -> &'static Mutex<ExpandWarnSeen> {
    static SET: OnceLock<Mutex<ExpandWarnSeen>> = OnceLock::new();
    SET.get_or_init(|| Mutex::new(ExpandWarnSeen::new()))
}

/// Returns `true` when this `var_name` had not yet emitted a user-facing
/// warning in this process — the caller should emit `ui::warn` once. A
/// poisoned mutex is recoverable: the protected state is a deduplication
/// hint, not a correctness invariant.
fn mark_expand_warn_emitted(var_name: &str) -> bool {
    let cache = expand_warn_seen();
    // DUP-3 / TASK-1477: shared poison-recover policy.
    let mut seen = crate::sync::lock_recover(cache);
    seen.insert(var_name)
}

#[cfg(test)]
pub(crate) fn expand_warn_seen_count() -> usize {
    // ERR-1 / TASK-1474: surface mutex poison via `lock_recover_warn` so a
    // sibling-panic flake is visible at the recovery site rather than
    // masquerading as a later count mismatch.
    let cache = expand_warn_seen();
    crate::sync::lock_recover_warn(cache, "expand_warn_seen_count")
        .set
        .len()
}

#[cfg(test)]
pub(crate) fn reset_expand_warn_seen() {
    let cache = expand_warn_seen();
    // DUP-3 / TASK-1477: shared poison-recover policy.
    let mut seen = crate::sync::lock_recover(cache);
    seen.set.clear();
    seen.order.clear();
}

impl Variables {
    /// Construct a `Variables` with no builtins populated. Used by
    /// fallback paths (e.g. `CommandRunner::from_arc_config` when
    /// `from_env` surfaces a non-UTF-8 workspace root via
    /// `ExpandError::NotUnicode`) so downstream `try_expand` calls fail
    /// loud on the missing variable rather than panicking at runner
    /// construction time. ERR-1 / TASK-1462.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            builtins: HashMap::new(),
        }
    }

    /// Build from environment and workspace root.
    ///
    /// READ-5 / TASK-1068: `TMPDIR` is read from the process environment at
    /// most once per process and cached in [`TMPDIR_DISPLAY`]. Setting
    /// `TMPDIR` via `std::env::set_var` *after* the first call here will
    /// **not** be observed by later callers. If a test needs a specific
    /// `TMPDIR`, set it before any `Variables::from_env` runs in that
    /// process.
    pub fn from_env(ops_root: &Path) -> Result<Self, ExpandError> {
        let mut builtins: HashMap<&'static str, Arc<str>> = HashMap::with_capacity(2);
        // ERR-1 / TASK-1462: surface a non-UTF-8 workspace root as a
        // typed `ExpandError::NotUnicode` instead of lossy-rendering
        // through `Path::display()`. Strict callers (the command-build
        // path) propagate this so a corrupt OPS_ROOT cannot silently
        // flow into spawned subprocess argv / cwd / env.
        builtins.insert("OPS_ROOT", cached_ops_root_arc(ops_root)?);
        let tmpdir = TMPDIR_DISPLAY
            .get_or_init(|| Arc::<str>::from(std::env::temp_dir().display().to_string()))
            .clone();
        builtins.insert("TMPDIR", tmpdir);
        Ok(Self { builtins })
    }

    /// Expand `$VAR`, `${VAR}`, `${VAR:-default}`, and `~` in the input string.
    ///
    /// Lossy variant: on a `shellexpand` error this logs a warning and
    /// returns the input unchanged. Suitable for display / dry-run paths
    /// where rendering "${VAR}" is acceptable. **Strict callers (the path
    /// that materialises arguments into argv, cwd, or env) MUST use
    /// [`Self::try_expand`]** so a non-UTF-8 env var fails loudly instead
    /// of being passed through literally (ERR-1 / TASK-0450).
    pub fn expand<'a>(&'a self, input: &'a str) -> Cow<'a, str> {
        match self.try_expand(input) {
            Ok(out) => out,
            Err(err) => {
                tracing::warn!(
                    var = %err.var_name,
                    cause = %err.cause,
                    "variable expansion failed; passing input through unchanged"
                );
                // ERR-1 / TASK-1224: a default `OPS_LOG_LEVEL=info` filters
                // the tracing line above, so users debugging "why does my
                // dry-run show ${HOME} literally" never see a diagnostic.
                // Surface the first failure per distinct variable name via
                // the always-on user-facing channel; subsequent failures of
                // the same variable stay structured-only so a single bad
                // env value in a hot loop does not flood stderr.
                if mark_expand_warn_emitted(&err.var_name) {
                    crate::ui::warn(format!(
                        "variable expansion failed for `${}`: {} — passing input \
                         through unchanged",
                        err.var_name, err.cause
                    ));
                }
                Cow::Borrowed(input)
            }
        }
    }

    /// Strict variant of [`Self::expand`]: returns `Err` on `shellexpand`
    /// errors (e.g. `VarError::NotUnicode`) instead of falling back to the
    /// literal input. Use this on any path that turns the result into an
    /// argv element, cwd, or env value — see ERR-1 / TASK-0450.
    pub fn try_expand<'a>(&'a self, input: &'a str) -> Result<Cow<'a, str>, ExpandError> {
        // CL-3: delegate to the shared helper so `~` expansion stays in sync
        // with platform path conventions used by the config loader.
        let home_dir = || -> Option<String> {
            crate::paths::home_dir()
                .as_deref()
                .and_then(std::path::Path::to_str)
                .map(String::from)
        };

        // OWN-8: builtins are borrowed from `self`; `Cow::Borrowed` avoids
        // one heap allocation per expanded var. Env vars are inherently
        // owned (std::env::var returns String) so they stay `Cow::Owned`.
        let lookup = |var: &str| -> Result<Option<Cow<'_, str>>, std::env::VarError> {
            if let Some(val) = self.builtins.get(var) {
                return Ok(Some(Cow::Borrowed(val.as_ref())));
            }
            match std::env::var(var) {
                Ok(val) => Ok(Some(Cow::Owned(val))),
                Err(std::env::VarError::NotPresent) => Ok(None),
                Err(e) => Err(e),
            }
        };

        shellexpand::full_with_context(input, home_dir, lookup).map_err(|err| ExpandError {
            var_name: err.var_name,
            cause: err.cause,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_vars() -> Variables {
        Variables::from_env(&PathBuf::from("/test/project"))
            .expect("UTF-8 test path; from_env must succeed")
    }

    #[test]
    fn expands_ops_root() {
        let vars = test_vars();
        assert_eq!(vars.expand("$OPS_ROOT/src"), "/test/project/src");
    }

    #[test]
    fn expands_ops_root_braced() {
        let vars = test_vars();
        assert_eq!(vars.expand("${OPS_ROOT}/src"), "/test/project/src");
    }

    #[test]
    fn expands_tilde() {
        let vars = test_vars();
        let result = vars.expand("~/config");
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap();
        assert_eq!(result, format!("{home}/config"));
    }

    #[test]
    fn expands_home_var() {
        let vars = test_vars();
        let result = vars.expand("$HOME/.config");
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap();
        assert_eq!(result, format!("{home}/.config"));
    }

    /// PERF-3 / TASK-1183: two `from_env` calls with the same `ops_root`
    /// must hand out the same `Arc<str>` for `OPS_ROOT` (LRU-of-1 cache),
    /// mirroring the TMPDIR amortisation invariant captured below.
    #[test]
    #[serial_test::serial(ops_root_cache)]
    fn from_env_reuses_cached_ops_root_arc() {
        let root = PathBuf::from("/test/project/lru-pinned");
        let v1 = Variables::from_env(&root).expect("UTF-8 path");
        let v2 = Variables::from_env(&root).expect("UTF-8 path");
        let a = v1.builtins.get("OPS_ROOT").expect("OPS_ROOT populated");
        let b = v2.builtins.get("OPS_ROOT").expect("OPS_ROOT populated");
        assert!(
            Arc::ptr_eq(a, b),
            "OPS_ROOT must be the same Arc across from_env calls with the same root"
        );
    }

    /// PERF-3 / TASK-0967: every `Variables::from_env` call must reuse the
    /// same `Arc<str>` for the cached `TMPDIR` rather than allocating a
    /// fresh `String`. Verified via `Arc::ptr_eq` on the values stored in
    /// the builtins map; the API surface (`expand` / `try_expand`) is
    /// unchanged and the existing `expands_tmpdir` test pins behaviour.
    #[test]
    fn from_env_reuses_cached_tmpdir_arc() {
        let v1 = test_vars();
        let v2 = test_vars();
        let a = v1.builtins.get("TMPDIR").expect("TMPDIR populated");
        let b = v2.builtins.get("TMPDIR").expect("TMPDIR populated");
        assert!(
            Arc::ptr_eq(a, b),
            "TMPDIR must be the same Arc across from_env calls"
        );
    }

    /// READ-5 / TASK-1068: lock in the documented process-lifetime contract
    /// of [`TMPDIR_DISPLAY`]. Once `Variables::from_env` has been called in
    /// the process, swapping `TMPDIR` via `set_var` is invisible — the cache
    /// is deliberate (PERF-3 / TASK-0967) and tests that need a different
    /// `TMPDIR` must set it before any `from_env` call. This test asserts
    /// that the post-init swap is *not* observed.
    #[test]
    #[serial_test::serial]
    fn tmpdir_swap_after_from_env_is_not_observed() {
        // Force the cache to populate (idempotent if already initialised in
        // this process — that is exactly the contract under test).
        let v_before = test_vars();
        let cached = v_before
            .builtins
            .get("TMPDIR")
            .expect("TMPDIR populated")
            .clone();

        // Swap TMPDIR to a value the cache could not possibly have captured.
        //
        // TEST-15 / TASK-1664: the swap target must be a **real** directory,
        // not a synthetic non-existent path. `set_var` is process-global while
        // `#[serial]` only serialises against other `#[serial]` tests, so for
        // the duration of this window every concurrently-running test that
        // calls `tempfile::tempdir()` resolves TMPDIR — ~85 of them in this
        // crate alone. Pointing it at nothing made them fail with
        // `NotFound`; `stack::tests::detect_finds_cargo_toml` was observed
        // dying exactly that way once the full suite started running in CI.
        // A valid directory proves the same contract (the cached Arc must not
        // change) without breaking unrelated tests that lose the race.
        //
        // Created before the swap, so it lands under the original TMPDIR.
        let swap_dir = tempfile::tempdir().expect("tempdir for the TMPDIR swap target");
        let prev = std::env::var_os("TMPDIR");
        // SAFETY: test-only guard via #[serial] attribute.
        unsafe {
            std::env::set_var("TMPDIR", swap_dir.path());
        }

        let v_after = test_vars();
        let observed = v_after.builtins.get("TMPDIR").expect("TMPDIR populated");

        // Restore TMPDIR before asserting so a panic does not leak state.
        // SAFETY: test-only guard via #[serial] attribute.
        unsafe {
            match prev {
                Some(val) => std::env::set_var("TMPDIR", val),
                None => std::env::remove_var("TMPDIR"),
            }
        }

        assert!(
            Arc::ptr_eq(&cached, observed),
            "post-init TMPDIR swap must not be observed: cache contract is process-lifetime"
        );
    }

    #[test]
    fn expands_tmpdir() {
        let vars = test_vars();
        let result = vars.expand("$TMPDIR/ops-test");
        let tmpdir = std::env::temp_dir().display().to_string();
        assert_eq!(result, format!("{tmpdir}/ops-test"));
    }

    #[test]
    fn expands_user() {
        let vars = test_vars();
        let result = vars.expand("$USER");
        // USER should come from env fallback
        if let Ok(user) = std::env::var("USER") {
            assert_eq!(result, user);
        }
        // On systems without USER, it passes through
    }

    #[test]
    fn expands_pwd() {
        let vars = test_vars();
        let result = vars.expand("$PWD");
        // PWD comes from env fallback (real env var)
        if let Ok(pwd) = std::env::var("PWD") {
            assert_eq!(result, pwd);
        }
    }

    #[test]
    fn no_expansion_for_plain_string() {
        let vars = test_vars();
        let input = "just a plain string";
        let result = vars.expand(input);
        assert_eq!(result, input);
        // Should be borrowed (zero alloc)
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn unknown_var_passes_through() {
        let vars = test_vars();
        // Use a var name extremely unlikely to exist in env
        let input = "$__OPS_NONEXISTENT_TEST_VAR_12345__";
        let result = vars.expand(input);
        assert_eq!(result, input);
    }

    /// READ-4 regression: pinning pass-through for a *deterministically*
    /// unset env var (removed via `remove_var`) rather than relying on a
    /// long unlikely-to-exist name. If shellexpand ever changes `Ok(None)`
    /// behaviour to substitute empty strings, this test breaks loudly.
    #[test]
    #[serial_test::serial]
    fn missing_env_var_passes_through_unchanged() {
        let key = "OPS_TEST_DEFINITELY_UNSET_VAR";
        // SAFETY: test-only guard via #[serial] attribute.
        unsafe { std::env::remove_var(key) };
        let vars = test_vars();
        let input = format!("${key}");
        let result = vars.expand(&input);
        assert_eq!(result.as_ref(), input, "missing env var must pass through");
    }

    /// ERR-1 regression: a `VarError::NotUnicode` from the lookup must not
    /// be conflated with "missing variable". The current contract is to log
    /// at warn and pass the input through unchanged; this test pins that
    /// pass-through for a deliberately-corrupt env value.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn non_utf8_env_var_passes_through_after_logging() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let key = "OPS_TEST_NON_UTF8_VAR";
        let bad: OsString = OsString::from_vec(vec![0xff, 0xfe, 0xfd]);
        // SAFETY: test-only guard via #[serial] attribute.
        unsafe {
            std::env::set_var(key, &bad);
        }
        let vars = test_vars();
        let input = format!("${key}");
        let result = vars.expand(&input);
        // SAFETY: test-only guard via #[serial] attribute.
        unsafe {
            std::env::remove_var(key);
        }
        assert_eq!(
            result.as_ref(),
            input,
            "non-UTF-8 env value must fall back to original input"
        );
    }

    /// ERR-1 / TASK-1224: when `expand` falls back on a `VarError`, the
    /// first failure per distinct variable name surfaces via the always-on
    /// `crate::ui::warn` channel; subsequent failures of the same variable
    /// must stay structured-only so a single bad env value in a hot loop
    /// does not flood stderr. A new distinct variable must trip the
    /// user-facing channel exactly once again.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn expand_warn_emits_once_per_distinct_var() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        super::reset_expand_warn_seen();
        assert_eq!(super::expand_warn_seen_count(), 0, "precondition");

        let key_a = "OPS_TEST_TASK1224_VAR_A";
        let key_b = "OPS_TEST_TASK1224_VAR_B";
        let bad: OsString = OsString::from_vec(vec![0xff, 0xfe, 0xfd]);
        // SAFETY: test-only guard via #[serial] attribute.
        unsafe {
            std::env::set_var(key_a, &bad);
            std::env::set_var(key_b, &bad);
        }

        let vars = test_vars();
        // Two failures on the same var: ui::warn must fire only once.
        let _ = vars.expand(&format!("${key_a}"));
        let _ = vars.expand(&format!("${key_a}/extra"));
        assert_eq!(
            super::expand_warn_seen_count(),
            1,
            "repeated failures on the same var must dedupe"
        );

        // A new distinct var: ui::warn fires again, set grows.
        let _ = vars.expand(&format!("${key_b}"));
        assert_eq!(
            super::expand_warn_seen_count(),
            2,
            "distinct var must surface a new ui::warn"
        );

        // SAFETY: test-only guard via #[serial] attribute.
        unsafe {
            std::env::remove_var(key_a);
            std::env::remove_var(key_b);
        }
        super::reset_expand_warn_seen();
    }

    /// CONC-1 / TASK-1418: the `ops_root_cache` must stay bounded under a
    /// stream of distinct workspace roots (e.g. an embedder rotating
    /// projects). Insert past the cap and assert the map size clamps to
    /// [`super::OPS_ROOT_CACHE_CAP`] with FIFO eviction so a new distinct
    /// root still gets memoised.
    #[test]
    #[serial_test::serial(ops_root_cache)]
    fn ops_root_cache_is_bounded_with_fifo_eviction() {
        super::reset_ops_root_cache();
        let cap = super::OPS_ROOT_CACHE_CAP;
        // Use synthetic non-existent paths so canonicalize() fails and the
        // raw path becomes the cache key — the test pins the eviction
        // policy, not filesystem behaviour.
        for i in 0..(cap * 2) {
            let root = PathBuf::from(format!("/synthetic/cache/bound/{i}"));
            let _ = Variables::from_env(&root).expect("UTF-8 path");
        }
        assert!(
            super::ops_root_cache_len() <= cap,
            "cache must stay clamped to OPS_ROOT_CACHE_CAP under churn (got {})",
            super::ops_root_cache_len()
        );
        super::reset_ops_root_cache();
    }

    /// PERF-3 / TASK-1423: the cache hit path must reuse the same
    /// `Arc<str>` across many lookups on the same root, going through the
    /// borrow-by-`&Path` fast path rather than allocating a fresh
    /// `PathBuf` per call. We can't observe the absence of allocation
    /// directly, but the `Arc::ptr_eq` invariant pins the documented
    /// hit-path contract: a regression to `entry(to_path_buf())` would
    /// still match `Arc::ptr_eq` but the per-call PathBuf allocation
    /// would surface in the microbench-style test below
    /// (`from_env_amortises_tmpdir`'s OPS_ROOT sibling).
    #[test]
    #[serial_test::serial(ops_root_cache)]
    fn ops_root_cache_hit_path_reuses_arc() {
        super::reset_ops_root_cache();
        let root = PathBuf::from("/synthetic/cache/hit/pin");
        let warm = Variables::from_env(&root).expect("UTF-8 path");
        let warm_arc = warm
            .builtins
            .get("OPS_ROOT")
            .expect("OPS_ROOT populated")
            .clone();
        for _ in 0..1024 {
            let v = Variables::from_env(&root).expect("UTF-8 path");
            let got = v.builtins.get("OPS_ROOT").expect("OPS_ROOT populated");
            assert!(
                Arc::ptr_eq(got, &warm_arc),
                "OPS_ROOT Arc must be reused on cache hits"
            );
        }
        super::reset_ops_root_cache();
    }

    /// ERR-1 / TASK-1462: a non-UTF-8 workspace root must fail
    /// `Variables::from_env` with a typed `ExpandError::NotUnicode`
    /// instead of being lossy-rendered through `Path::display()` and
    /// silently flowing into argv / cwd / env via `try_expand`. The
    /// canonicalize syscall fails on a synthetic non-existent path so the
    /// raw `&Path` branch is what feeds `to_str()`; that mirrors the
    /// production failure mode (the workspace root may legitimately be
    /// non-existent on freshly-staged repos — the cache still installs
    /// the raw path).
    #[cfg(unix)]
    #[test]
    fn from_env_rejects_non_utf8_workspace_root() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        // Build a `PathBuf` from raw bytes that is not valid UTF-8.
        // 0xff is reserved as a continuation byte in UTF-8 and never
        // appears in a valid encoding.
        let bytes: &[u8] = b"/tmp/non-utf8-task-1462-\xff/root";
        let bad: PathBuf = PathBuf::from(OsStr::from_bytes(bytes));

        let err = Variables::from_env(&bad).expect_err("non-UTF-8 root must error");
        assert_eq!(err.var_name, "OPS_ROOT");
        assert!(
            matches!(err.cause, std::env::VarError::NotUnicode(_)),
            "expected VarError::NotUnicode, got {:?}",
            err.cause
        );
    }

    /// PERF-3 / TASK-1465: once the OPS_ROOT cache has been warmed for a
    /// given workspace root, subsequent `from_env` calls must skip the
    /// `std::fs::canonicalize` syscall entirely — the hit path probes the
    /// cache by raw `&Path` before paying any IO.
    ///
    /// TEST-15 / TASK-1664: asserted by **counting the syscall**, not by
    /// timing. The previous form ran 200k calls against a one-second
    /// wall-clock budget, which is load-dependent in a debug build: it passes
    /// on an idle machine and fails under CPU contention (measured here at
    /// ~2-4.7s with parallel builds running). That is the state a shared CI
    /// runner is normally in, so enabling the full suite in CI would have made
    /// it a recurring flake. The counter proves the same property exactly, in
    /// 1/200th of the iterations, with no timing dependency at all.
    ///
    /// Same treatment `secret_patterns.rs` already applied to its
    /// `SECRET_SCAN_LIMIT` test for the same reason.
    #[test]
    #[serial_test::serial(ops_root_cache)]
    fn from_env_hit_path_avoids_canonicalize_syscall() {
        super::reset_ops_root_cache();
        // Synthetic path that does not exist; `std::fs::canonicalize`
        // on this path would error on every call, but the cache hit path
        // must short-circuit before reaching that syscall.
        let root = PathBuf::from("/synthetic/task-1465/no-canonicalize-on-hit");

        let _ = Variables::from_env(&root).expect("UTF-8 path; first call installs cache");

        // Count only the warm calls: the miss above legitimately canonicalises.
        let before = super::canonicalize_call_count(&root);
        for _ in 0..1_000 {
            let _ = Variables::from_env(&root).expect("UTF-8 path");
        }
        let after = super::canonicalize_call_count(&root);
        assert_eq!(
            after,
            before,
            "1000 warm `from_env` calls must not reach canonicalize; \
             saw {} syscall(s) — the hit path regressed to canonicalising \
             before the cache probe",
            after - before
        );

        super::reset_ops_root_cache();
    }

    /// ERR-1 / TASK-1424: two semantically equal workspace roots — here, a
    /// real tempdir and a symlink that points at it — must yield the same
    /// `OPS_ROOT` `Arc<str>` so a single process that derives the root
    /// twice (relative for dry-run, absolute for exec) cannot leak two
    /// divergent renderings into argv / env expansion.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial(ops_root_cache)]
    fn ops_root_cache_collapses_equivalent_paths() {
        super::reset_ops_root_cache();
        let dir = tempfile::tempdir().expect("tempdir");
        let real = dir.path().to_path_buf();
        let link = dir.path().with_extension("link");
        std::os::unix::fs::symlink(&real, &link).expect("symlink");

        let v_real = Variables::from_env(&real).expect("UTF-8 path");
        let v_link = Variables::from_env(&link).expect("UTF-8 path");
        let a = v_real.builtins.get("OPS_ROOT").expect("OPS_ROOT populated");
        let b = v_link.builtins.get("OPS_ROOT").expect("OPS_ROOT populated");
        assert!(
            Arc::ptr_eq(a, b),
            "equivalent paths (real + symlink) must collapse to one Arc<str>"
        );

        // Cleanup.
        let _ = std::fs::remove_file(&link);
        super::reset_ops_root_cache();
    }

    /// PERF-3 / TASK-1411 + CONC-1 / TASK-1444: an adversarial stream of
    /// distinct variable names must not grow `expand_warn_seen` without
    /// bound. The cap is documented at [`super::EXPAND_WARN_SEEN_CAP`];
    /// eviction is FIFO so a new distinct name still surfaces a one-shot
    /// user-facing warn after the set reaches capacity.
    #[test]
    #[serial_test::serial]
    fn expand_warn_seen_is_bounded_with_fifo_eviction() {
        super::reset_expand_warn_seen();
        let cap = super::EXPAND_WARN_SEEN_CAP;
        // Fill past the cap by 2x distinct names.
        for i in 0..(cap * 2) {
            let name = format!("ADVERSARIAL_VAR_{i}");
            let first = super::mark_expand_warn_emitted(&name);
            assert!(first, "first sighting of `{name}` must mark as new");
        }
        assert_eq!(
            super::expand_warn_seen_count(),
            cap,
            "set size must be clamped to the documented cap"
        );

        // The very first inserted name should have been evicted; re-inserting
        // it must again return `true` (the user-facing warn fires once more).
        let first_name = "ADVERSARIAL_VAR_0";
        let re_marked = super::mark_expand_warn_emitted(first_name);
        assert!(
            re_marked,
            "evicted name must re-mark as new on next sighting (FIFO eviction)"
        );

        // A name still inside the window must dedupe (warn-once contract).
        let recent_name = format!("ADVERSARIAL_VAR_{}", cap * 2 - 1);
        let recent_dedupe = super::mark_expand_warn_emitted(&recent_name);
        assert!(
            !recent_dedupe,
            "recent name must still dedupe within the capacity window"
        );

        super::reset_expand_warn_seen();
    }

    /// TASK-1444 AC#3: the bound must be large enough that realistic
    /// command-spec workloads never evict (>= 64). Pin the documented floor
    /// at compile time so a future tightening surfaces in build output.
    const _: () = assert!(
        super::EXPAND_WARN_SEEN_CAP >= 64,
        "documented floor (TASK-1444): cap must accommodate realistic workloads"
    );

    /// TASK-0450: strict `try_expand` must surface the underlying
    /// `VarError::NotUnicode` so the caller can fail the spawn instead of
    /// materialising a literal `${VAR}` into argv / cwd.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn try_expand_fails_loudly_on_non_utf8_env_var() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let key = "OPS_TEST_NON_UTF8_TRY_EXPAND";
        let bad: OsString = OsString::from_vec(vec![0xff, 0xfe, 0xfd]);
        // SAFETY: test-only guard via #[serial] attribute.
        unsafe { std::env::set_var(key, &bad) };
        let vars = test_vars();
        let input = format!("${key}");
        let outcome = vars.try_expand(&input);
        // SAFETY: test-only guard via #[serial] attribute.
        unsafe { std::env::remove_var(key) };
        let err = outcome.expect_err("non-UTF-8 env var must fail strict expansion");
        assert_eq!(err.var_name, key);
        assert!(
            matches!(err.cause, std::env::VarError::NotUnicode(_)),
            "ExpandError must carry the typed VarError cause: {:?}",
            err.cause
        );
        // ERR-7 / TASK-0835: source() must walk back to the underlying VarError.
        let src = std::error::Error::source(&err).expect("source chain present");
        assert!(src.is::<std::env::VarError>(), "source should be VarError");
    }

    #[test]
    fn try_expand_propagates_value_for_known_var() {
        let vars = test_vars();
        let result = vars
            .try_expand("$OPS_ROOT/src")
            .expect("known var must succeed");
        assert_eq!(result, "/test/project/src");
    }

    #[test]
    fn default_value_syntax() {
        let vars = test_vars();
        let result = vars.expand("${__OPS_NONEXISTENT_TEST_VAR__:-fallback}");
        assert_eq!(result, "fallback");
    }

    #[test]
    fn multiple_vars_in_one_string() {
        let vars = test_vars();
        let result = vars.expand("$OPS_ROOT and $TMPDIR");
        let tmpdir = std::env::temp_dir().display().to_string();
        assert_eq!(result, format!("/test/project and {tmpdir}"));
    }

    /// Microbench-style regression: constructing `Variables::from_env` many
    /// times must amortise to the cached `TMPDIR` lookup rather than re-running
    /// the `std::env::temp_dir()` syscall on every call. Pins the OnceLock
    /// optimisation; if it regresses (TMPDIR resolved per call) the syscall
    /// cost becomes visible at scale.
    #[test]
    fn from_env_amortises_tmpdir() {
        let root = PathBuf::from("/bench/root");
        // Warm the OnceLock once.
        let warm = Variables::from_env(&root).expect("UTF-8 path");
        let warm_tmpdir = warm
            .builtins
            .get("TMPDIR")
            .expect("TMPDIR populated")
            .clone();
        // Subsequent calls must observe the *same* cached `Arc<str>` — not just
        // an equal string. TEST-11 / TASK-1037: a value-equality assertion here
        // would still pass even if `from_env` re-rendered the TMPDIR path on
        // every call, defeating the OnceLock optimisation this test exists to
        // pin. `Arc::ptr_eq` is the only check that breaks on regression.
        for _ in 0..1000 {
            let v = Variables::from_env(&root).expect("UTF-8 path");
            let got = v.builtins.get("TMPDIR").expect("TMPDIR populated");
            assert!(
                Arc::ptr_eq(got, &warm_tmpdir),
                "TMPDIR Arc must be reused across from_env calls (OnceLock cache)"
            );
        }
    }
}
