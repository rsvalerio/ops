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

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]

pub mod config;
pub mod git;
pub mod git_state;
pub mod install;
pub(crate) mod paths;

#[cfg(any(test, feature = "test-helpers"))]
pub mod test_helpers;

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
    /// Environment variable that skips execution when set to a truthy value.
    /// [`should_skip`] is the source of truth for which values count
    /// (READ-5 / TASK-1916).
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

/// Generate a complete POSIX sh hook script from a crate-specific tail.
///
/// DUP-1 / TASK-2108: the bypass-then-probe prologue — the skip-var `case`
/// guard followed by the `command -v ops` preflight with its diagnostic —
/// used to be hand-copied into every hook crate, and the copies diverged:
/// the pre-push script advertised `SKIP_OPS_RUN_BEFORE_PUSH` as the escape
/// hatch yet never evaluated it, so the advertised bypass did not work in
/// exactly the situation its own diagnostic described. The prologue now
/// lives here, once, parameterised by the same fields [`HookConfig`] carries.
///
/// Every argument must be a string literal, so the result is a `&'static
/// str` usable in `const` contexts (`const HOOK_SCRIPT: &str = ...`):
///
/// - `name` — the extension name (`"run-before-commit"`), spelled into the
///   "Installed by" comment and the reinstall advice.
/// - `hook_filename` — the git hook filename (`"pre-commit"`), used as the
///   diagnostic prefix and the hook path it names.
/// - `skip_env_var` — the bypass env var name. Keep it identical to the
///   calling crate's `SKIP_ENV_VAR` const; pin the two together with a test,
///   since a macro cannot reference the const and stay `const`-evaluable.
/// - `tail` — the hook-specific lines that follow the guard.
#[macro_export]
macro_rules! hook_script {
    (
        name: $name:literal,
        hook_filename: $hook_filename:literal,
        skip_env_var: $skip_env_var:literal,
        tail: $( $tail:literal ),* $(,)?
    ) => {
        concat!(
            "#!/bin/sh\n",
            "# Installed by `ops ", $name, " install`.\n",
            "# The bypass is honoured before the probe below: that probe's own\n",
            "# diagnostic advertises this variable, so it has to work in exactly\n",
            "# the situation the diagnostic describes -- ops missing from PATH.\n",
            "# Matched with shell builtins only, for the same reason. Value list\n",
            "# mirrors `ops_hook_common::should_skip`.\n",
            "case \"${", $skip_env_var, ":-}\" in\n",
            "    1 | [Tt][Rr][Uu][Ee] | [Yy][Ee][Ss] | [Oo][Nn]) exit 0 ;;\n",
            "esac\n",
            "if ! command -v ops >/dev/null 2>&1; then\n",
            "    echo \"", $hook_filename, ": cannot find the 'ops' binary on PATH ",
            "(hook: .git/hooks/", $hook_filename, ").\" >&2\n",
            "    echo \"", $hook_filename, ": add ops to PATH (e.g. ~/.cargo/bin) and rerun \\`ops ",
            $name, " install\\`, or bypass with ", $skip_env_var, "=1.\" >&2\n",
            "    exit 1\n",
            "fi\n",
            $( $tail, )*
        )
    };
}

/// Returns `true` if the skip env var is set to a recognized truthy value.
///
/// Accepts (case-insensitive): `"1"`, `"true"`, `"yes"`, `"on"`. Anything else
/// — including the empty string, `"0"`, `"false"`, or arbitrary text — is
/// treated as "don't skip". This matches how most CLI env-var opt-outs are
/// commonly typed; documenting only `"1"` previously surprised users who set
/// `SKIP_OPS_RUN_BEFORE_COMMIT=true`.
#[must_use]
pub fn should_skip(config: &HookConfig) -> bool {
    std::env::var(config.skip_env_var)
        .is_ok_and(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
}

/// Generate the per-extension hook wrappers (`HOOK_CONFIG`, `should_skip`,
/// `find_git_dir`, `install_hook`, `ensure_config_command`) from a single
/// declarative description.
///
/// Keeps the two hook extension crates in lockstep.
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

        /// Per-extension wrapper for [`ops_hook_common::ensure_config_command`].
        ///
        /// The synthesized `[commands.<name>]` entry hardcodes
        /// `fail_fast = true`. See the wrapped function's doc for the
        /// rationale and the operator override path
        /// (hand-edit `.ops.toml` post-install; the early-exit guard
        /// preserves the edit on subsequent reinstalls).
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
    use crate::test_helpers::EnvGuard;

    #[test]
    #[serial_test::serial]
    fn should_skip_returns_false_by_default() {
        let cfg = commit_config();
        let _guard = EnvGuard::remove(cfg.skip_env_var);
        assert!(!should_skip(&cfg));
    }

    /// TEST-6 (TASK-1884): the accepted-token set is the operator's opt-out
    /// contract. Pin every documented spelling so a refactor to `v == "1"`
    /// cannot silently remove the escape hatch.
    #[test]
    #[serial_test::serial]
    fn should_skip_accepts_documented_truthy_tokens() {
        let cfg = commit_config();
        for value in ["1", "true", "yes", "on"] {
            let _guard = EnvGuard::set(cfg.skip_env_var, value);
            assert!(should_skip(&cfg), "{value:?} must skip");
        }
    }

    /// TEST-6 (TASK-1884): the doc promises case-insensitivity.
    #[test]
    #[serial_test::serial]
    fn should_skip_is_case_insensitive() {
        let cfg = commit_config();
        for value in ["TRUE", "Yes", "ON", "On", "TrUe"] {
            let _guard = EnvGuard::set(cfg.skip_env_var, value);
            assert!(should_skip(&cfg), "{value:?} must skip");
        }
    }

    /// TEST-6 (TASK-1884): the rejection half of the contract, which nothing
    /// covered. A refactor to "set means true" would make `SKIP_...=false` —
    /// the spelling an operator reaches for to *re-enable* the hook —
    /// silently disable every pre-commit and pre-push check.
    #[test]
    #[serial_test::serial]
    fn should_skip_rejects_set_but_falsy_values() {
        let cfg = commit_config();
        for value in ["0", "false", "FALSE", "no", "off", "", "maybe", " 1"] {
            let _guard = EnvGuard::set(cfg.skip_env_var, value);
            assert!(!should_skip(&cfg), "{value:?} must not skip");
        }
    }

    // -- hook_script! prologue (DUP-1 / TASK-2108) --

    /// A stand-in script with no risk of colliding with a real crate's
    /// identifiers, so these tests pin the *prologue* rather than any one
    /// crate's tail.
    macro_rules! probe_script {
        () => {
            crate::hook_script! {
                name: "run-before-probe",
                hook_filename: "pre-probe",
                skip_env_var: "SKIP_OPS_RUN_BEFORE_PROBE",
                tail: "exec ops run-before-probe\n",
            }
        };
    }

    /// TASK-2108 AC#1+#2: the generated prologue evaluates the bypass
    /// *before* the missing-ops probe, and names the hook path, the skip
    /// var, and the reinstall command in its diagnostic.
    #[test]
    fn hook_script_prologue_honours_the_bypass_before_the_probe() {
        let script = probe_script!();
        let bypass_at = script
            .find("case \"${SKIP_OPS_RUN_BEFORE_PROBE:-}\" in")
            .expect("prologue must open the bypass case");
        let probe_at = script
            .find("if ! command -v ops")
            .expect("prologue must probe for ops");
        assert!(
            bypass_at < probe_at,
            "the bypass must be honoured before the probe, got: {script}"
        );
        assert!(script.contains(".git/hooks/pre-probe"));
        // Backticks are shell-escaped in the diagnostic, so the script text
        // carries them as \` — the shell prints bare backticks at runtime.
        assert!(script.contains("rerun \\`ops run-before-probe install\\`"));
    }

    /// TASK-2108 AC#4: the generated script parses under `sh -n`, fails
    /// closed with `ops` off PATH, and exits 0 for every documented truthy
    /// bypass token in that same situation.
    #[cfg(unix)]
    #[test]
    fn hook_script_prologue_passes_sh_n_bypass_and_fails_closed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let script_path = dir.path().join("pre-probe");
        std::fs::write(&script_path, probe_script!()).unwrap();

        let parse = std::process::Command::new("/bin/sh")
            .arg("-n")
            .arg(&script_path)
            .status()
            .unwrap();
        assert!(parse.success(), "prologue must parse under `sh -n`");

        // PATH deliberately excludes the ambient one so a developer's own
        // installed `ops` cannot satisfy the probe.
        let run = |envs: &[(&str, &str)]| {
            let mut cmd = std::process::Command::new("/bin/sh");
            cmd.arg(&script_path).env("PATH", "/usr/bin:/bin");
            for (k, v) in envs {
                cmd.env(k, v);
            }
            cmd.output().unwrap()
        };

        for value in ["1", "true", "TRUE", "Yes", "on"] {
            let out = run(&[("SKIP_OPS_RUN_BEFORE_PROBE", value)]);
            assert_eq!(
                out.status.code(),
                Some(0),
                "{value:?} must skip cleanly, stderr was: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }

        let out = run(&[]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "stderr was: {stderr}");
        assert!(stderr.contains("ops"), "must name ops, got: {stderr}");
        assert!(stderr.contains(".git/hooks/pre-probe"), "got: {stderr}");
        assert!(
            stderr.contains("SKIP_OPS_RUN_BEFORE_PROBE"),
            "got: {stderr}"
        );

        // A value `should_skip` rejects must still reach the probe and fail.
        let out = run(&[("SKIP_OPS_RUN_BEFORE_PROBE", "maybe")]);
        assert_eq!(out.status.code(), Some(1));
    }
}
