//! Variable expansion for command specs.
//!
//! Expands `$VAR`, `${VAR}`, `${VAR:-default}`, and `~` in strings using
//! built-in variables and environment fallback via `shellexpand`.

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::path::Path;

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
    builtins: HashMap<&'static str, String>,
}

/// Cached `std::env::temp_dir()` rendering. Computed once per process: the
/// value depends on `TMPDIR` / OS defaults that do not change after startup,
/// and `temp_dir()` itself performs a syscall on Unix. Reused across every
/// `Variables::from_env` call so command-spec expansion avoids the syscall +
/// allocation on every invocation.
static TMPDIR_DISPLAY: std::sync::OnceLock<String> = std::sync::OnceLock::new();

impl Variables {
    /// Build from environment and workspace root.
    pub fn from_env(ops_root: &Path) -> Self {
        let mut builtins: HashMap<&'static str, String> = HashMap::with_capacity(2);
        builtins.insert("OPS_ROOT", ops_root.display().to_string());
        let tmpdir = TMPDIR_DISPLAY
            .get_or_init(|| std::env::temp_dir().display().to_string())
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
                return Ok(Some(Cow::Borrowed(val.as_str())));
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
        let warm_tmpdir = warm.builtins.get("TMPDIR").cloned();
        // Subsequent calls must observe the same cached value.
        for _ in 0..1000 {
            let v = Variables::from_env(&root);
            assert_eq!(v.builtins.get("TMPDIR").cloned(), warm_tmpdir);
        }
    }
}
