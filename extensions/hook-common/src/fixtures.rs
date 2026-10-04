//! Shared test fixtures.
//!
//! The one definition of the `commit_config` / `push_config` values the test
//! modules in `lib.rs`, `install.rs`, and `config.rs` share, so a change to
//! `HookConfig`'s fields is made in one place.
//!
//! The `hook_script` values use the `#!/bin/sh` shebang the wrapper crates
//! install, so the fixtures stay a true reference for what an ops hook looks
//! like.

#![cfg(test)]

use crate::HookConfig;

pub fn commit_config() -> HookConfig {
    HookConfig {
        name: "run-before-commit",
        hook_filename: "pre-commit",
        hook_script: "#!/bin/sh\nexec ops run-before-commit\n",
        skip_env_var: "SKIP_OPS_RUN_BEFORE_COMMIT",
        legacy_markers: &[
            "ops run-before-commit",
            "ops before-commit",
            "ops pre-commit",
        ],
        command_help: "Run run-before-commit checks before committing",
    }
}

pub fn push_config() -> HookConfig {
    HookConfig {
        name: "run-before-push",
        hook_filename: "pre-push",
        hook_script: "#!/bin/sh\nexec ops run-before-push\n",
        skip_env_var: "SKIP_OPS_RUN_BEFORE_PUSH",
        legacy_markers: &["ops run-before-push", "ops before-push"],
        command_help: "Run run-before-push checks before pushing",
    }
}

/// A strict, non-empty prefix of `cfg`'s installed script: what a write that
/// died mid-stream leaves on disk, and therefore what
/// `install::classify_existing_hook` must report as `Partial` rather than as
/// a foreign user-authored hook.
///
/// Derived from `cfg.hook_script` rather than spelled out as a literal: a
/// literal prefix is coupled to the fixture's shebang, and stops being a
/// prefix — `Foreign`, not `Partial` — the moment the fixture is edited.
pub fn truncated_hook_script(cfg: &HookConfig) -> &'static str {
    // Everything up to and including the first `ops ` token: still inside the
    // second line, so the result cannot equal the whole script.
    cfg.hook_script
        .split_inclusive("ops ")
        .next()
        .unwrap_or(cfg.hook_script)
}
