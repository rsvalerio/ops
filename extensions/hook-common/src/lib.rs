//! Shared logic for git hook extensions (run-before-commit, run-before-push).
//!
//! Both hook crates share identical control flow differing only in constants
//! (hook filename, env var name, legacy markers, help text). This crate
//! extracts those common functions behind a [`HookConfig`] descriptor.
//!
//! Submodules:
//! - [`git`]: `.git` directory discovery (plain repos, worktrees, submodules).
//! - [`install`]: hook file installation with symlink/out-of-tree defenses.
//! - [`config`]: `.ops.toml` mutation to register the hook's composite command.

pub mod config;
pub mod git;
pub mod install;
pub(crate) mod paths;

#[cfg(test)]
mod fixtures;

pub use config::ensure_config_command;
pub use git::find_git_dir;
pub use install::install_hook;

/// Describes one git-hook extension so the shared helpers know which file to
/// create, which env var to check, etc.
///
/// Marked `#[non_exhaustive]`: out-of-crate code must use [`HookConfig::new`]
/// (or the [`crate::impl_hook_wrappers!`] macro that wraps it) so adding new
/// fields stays a non-breaking change.
#[non_exhaustive]
pub struct HookConfig {
    /// Command name, e.g. `"run-before-commit"`.
    pub name: &'static str,
    /// Git hook filename inside `.git/hooks/`, e.g. `"pre-commit"`.
    pub hook_filename: &'static str,
    /// The full hook script to install.
    pub hook_script: &'static str,
    /// Environment variable that, when set to `"1"`, skips execution.
    pub skip_env_var: &'static str,
    /// Substrings in an existing hook that mark it as a legacy ops hook
    /// (will be overwritten).
    pub legacy_markers: &'static [&'static str],
    /// Help text written into the TOML command entry.
    pub command_help: &'static str,
}

impl HookConfig {
    /// Build a `HookConfig` from its parts. Use this instead of struct-literal
    /// construction so adding fields here stays backwards compatible.
    ///
    /// One arg per field: the constructor mirrors the struct's shape so the
    /// macro that drives it stays a flat declarative description.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        name: &'static str,
        hook_filename: &'static str,
        hook_script: &'static str,
        skip_env_var: &'static str,
        legacy_markers: &'static [&'static str],
        command_help: &'static str,
    ) -> Self {
        Self {
            name,
            hook_filename,
            hook_script,
            skip_env_var,
            legacy_markers,
            command_help,
        }
    }
}

/// Returns `true` if the skip env var is set to a recognized truthy value.
///
/// Accepts (case-insensitive): `"1"`, `"true"`, `"yes"`, `"on"`. Anything else
/// — including the empty string, `"0"`, `"false"`, or arbitrary text — is
/// treated as "don't skip". This matches how most CLI env-var opt-outs are
/// commonly typed; documenting only `"1"` previously surprised users who set
/// `SKIP_OPS_RUN_BEFORE_COMMIT=true`.
pub fn should_skip(config: &HookConfig) -> bool {
    std::env::var(config.skip_env_var)
        .ok()
        .is_some_and(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
}

/// Generate the per-extension hook wrappers (`HOOK_CONFIG`, `should_skip`,
/// `find_git_dir`, `install_hook`, `ensure_config_command`) from a single
/// declarative description. Keeps the two hook extension crates in lockstep.
#[macro_export]
macro_rules! impl_hook_wrappers {
    (
        name: $name:expr,
        hook_filename: $hook_filename:expr,
        hook_script: $hook_script:expr,
        skip_env_var: $skip_env_var:expr,
        legacy_markers: $legacy_markers:expr,
        command_help: $command_help:expr $(,)?
    ) => {
        pub const HOOK_CONFIG: $crate::HookConfig = $crate::HookConfig::new(
            $name,
            $hook_filename,
            $hook_script,
            $skip_env_var,
            $legacy_markers,
            $command_help,
        );

        pub fn hook_config() -> $crate::HookConfig {
            HOOK_CONFIG
        }

        pub fn should_skip() -> bool {
            $crate::should_skip(&HOOK_CONFIG)
        }

        pub fn find_git_dir(from: &::std::path::Path) -> Option<::std::path::PathBuf> {
            $crate::find_git_dir(from)
        }

        pub fn install_hook(
            git_dir: &::std::path::Path,
            w: &mut dyn ::std::io::Write,
        ) -> ::anyhow::Result<::std::path::PathBuf> {
            $crate::install_hook(&HOOK_CONFIG, git_dir, w)
        }

        pub fn ensure_config_command(
            config_dir: &::std::path::Path,
            selected_commands: &[String],
            w: &mut dyn ::std::io::Write,
        ) -> ::anyhow::Result<()> {
            $crate::ensure_config_command(&HOOK_CONFIG, config_dir, selected_commands, w)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::commit_config;

    #[test]
    #[serial_test::serial]
    fn should_skip_returns_false_by_default() {
        let cfg = commit_config();
        let _guard = EnvGuard::remove(cfg.skip_env_var);
        assert!(!should_skip(&cfg));
    }

    /// RAII guard that restores an env var to its previous value on drop.
    /// Pair with `#[serial_test::serial]` to prevent races with other env-mutating tests.
    struct EnvGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvGuard {
        fn remove(key: &'static str) -> Self {
            let original = std::env::var(key).ok();
            std::env::remove_var(key);
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }
}
