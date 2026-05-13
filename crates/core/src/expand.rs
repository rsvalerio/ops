//! Variable expansion for command specs.
//!
//! Expands `$VAR`, `${VAR}`, `${VAR:-default}`, and `~` in strings using
//! built-in variables and environment fallback via `shellexpand`.

use std::borrow::Cow;
use std::collections::hash_map::Entry as HashMapEntry;
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
fn ops_root_cache() -> &'static Mutex<HashMap<PathBuf, Arc<str>>> {
    static CACHE: std::sync::OnceLock<Mutex<HashMap<PathBuf, Arc<str>>>> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cached_ops_root_arc(ops_root: &Path) -> Arc<str> {
    // CONC-1 / TASK-1183: lock contention is negligible because `from_env`
    // is not called from the parallel exec hot loop (that path constructs
    // `Variables` once and clones the Arc from the runner).
    let cache = ops_root_cache();
    let mut guard = cache.lock().unwrap_or_else(|e| {
        // CONC-9 / TASK-1183: lock poisoning here is recoverable — the
        // protected map's every state is valid. Clear and continue.
        cache.clear_poison();
        e.into_inner()
    });
    match guard.entry(ops_root.to_path_buf()) {
        HashMapEntry::Occupied(occ) => Arc::clone(occ.get()),
        HashMapEntry::Vacant(vac) => {
            let arc = Arc::<str>::from(ops_root.display().to_string());
            vac.insert(Arc::clone(&arc));
            arc
        }
    }
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
    let mut seen = cache.lock().unwrap_or_else(|e| {
        cache.clear_poison();
        e.into_inner()
    });
    seen.insert(var_name)
}

#[cfg(test)]
pub(crate) fn expand_warn_seen_count() -> usize {
    let cache = expand_warn_seen();
    cache
        .lock()
        .map(|s| s.set.len())
        .unwrap_or_else(|e| e.into_inner().set.len())
}

#[cfg(test)]
pub(crate) fn reset_expand_warn_seen() {
    let cache = expand_warn_seen();
    let mut seen = cache.lock().unwrap_or_else(|e| {
        cache.clear_poison();
        e.into_inner()
    });
    seen.set.clear();
    seen.order.clear();
}

impl Variables {
    /// Build from environment and workspace root.
    ///
    /// READ-5 / TASK-1068: `TMPDIR` is read from the process environment at
    /// most once per process and cached in [`TMPDIR_DISPLAY`]. Setting
    /// `TMPDIR` via `std::env::set_var` *after* the first call here will
    /// **not** be observed by later callers. If a test needs a specific
    /// `TMPDIR`, set it before any `Variables::from_env` runs in that
    /// process.
    pub fn from_env(ops_root: &Path) -> Self {
        let mut builtins: HashMap<&'static str, Arc<str>> = HashMap::with_capacity(2);
        builtins.insert("OPS_ROOT", cached_ops_root_arc(ops_root));
        let tmpdir = TMPDIR_DISPLAY
            .get_or_init(|| Arc::<str>::from(std::env::temp_dir().display().to_string()))
            .clone();
        builtins.insert("TMPDIR", tmpdir);
        Self { builtins }
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
        assert_eq!(result, format!("{}/config", home));
    }

    #[test]
    fn expands_home_var() {
        let vars = test_vars();
        let result = vars.expand("$HOME/.config");
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap();
        assert_eq!(result, format!("{}/.config", home));
    }

    /// PERF-3 / TASK-1183: two `from_env` calls with the same `ops_root`
    /// must hand out the same `Arc<str>` for `OPS_ROOT` (LRU-of-1 cache),
    /// mirroring the TMPDIR amortisation invariant captured below.
    #[test]
    #[serial_test::serial(ops_root_cache)]
    fn from_env_reuses_cached_ops_root_arc() {
        let root = PathBuf::from("/test/project/lru-pinned");
        let v1 = Variables::from_env(&root);
        let v2 = Variables::from_env(&root);
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
        let prev = std::env::var_os("TMPDIR");
        // SAFETY: test-only guard via #[serial] attribute.
        unsafe {
            std::env::set_var("TMPDIR", "/definitely/not/the/cached/tmpdir/READ-5");
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
        assert_eq!(result, format!("{}/ops-test", tmpdir));
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
        assert_eq!(result, format!("/test/project and {}", tmpdir));
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
        let warm = Variables::from_env(&root);
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
            let v = Variables::from_env(&root);
            let got = v.builtins.get("TMPDIR").expect("TMPDIR populated");
            assert!(
                Arc::ptr_eq(got, &warm_tmpdir),
                "TMPDIR Arc must be reused across from_env calls (OnceLock cache)"
            );
        }
    }
}
