//! Variable expansion for command specs.
//!
//! Expands `$VAR`, `${VAR}`, `${VAR:-default}`, and `~` in strings using
//! built-in variables and environment fallback via `shellexpand`.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;

/// Built-in variables available for expansion in command specs.
///
/// Lookup order: built-in variables first, then `std::env::var()` fallback.
/// Unknown variables are left as-is.
#[derive(Debug, Clone)]
pub struct Variables {
    builtins: HashMap<String, String>,
}

impl Variables {
    /// Build from environment and workspace root.
    pub fn from_env(ops_root: &Path) -> Self {
        let mut builtins = HashMap::new();
        builtins.insert("OPS_ROOT".into(), ops_root.display().to_string());
        builtins.insert("TMPDIR".into(), std::env::temp_dir().display().to_string());
        Self { builtins }
    }

    /// Expand `$VAR`, `${VAR}`, `${VAR:-default}`, and `~` in the input string.
    pub fn expand<'a>(&'a self, input: &'a str) -> Cow<'a, str> {
        let home_dir = || -> Option<String> {
            std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .ok()
        };

        let lookup = |var: &str| -> Result<Option<Cow<'_, str>>, std::env::VarError> {
            if let Some(val) = self.builtins.get(var) {
                return Ok(Some(Cow::Owned(val.clone())));
            }
            match std::env::var(var) {
                Ok(val) => Ok(Some(Cow::Owned(val))),
                Err(std::env::VarError::NotPresent) => Ok(None),
                Err(e) => Err(e),
            }
        };

        shellexpand::full_with_context(input, home_dir, lookup).unwrap_or(Cow::Borrowed(input))
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
}
