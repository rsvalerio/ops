//! Shared test fixtures.
//!
//! Test modules in `lib.rs`, `install.rs`, and `config.rs` previously each
//! declared their own `commit_config` / `push_config` literals; centralising
//! them here removes drift risk when `HookConfig` fields evolve.
//!
//! READ-10 / TASK-2036: the `hook_script` values track what the wrapper
//! crates actually install — `#!/bin/sh`, not the `#!/usr/bin/env bash` shape
//! `run-before-commit` and `run-before-push` abandoned in TASK-1910 /
//! TASK-1911. A fixture that has drifted from every real value it stands in
//! for is a false reference for anyone reading these tests to learn what an
//! ops hook looks like.

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
/// READ-10 / TASK-2036: derived from `cfg.hook_script` instead of hardcoded.
/// The partial-classification tests used to spell the prefix out as a literal,
/// which silently coupled them to the fixture's shebang — editing the fixture
/// reclassified the literal from `Partial` to `Foreign` and failed the tests
/// for a reason that had nothing to do with the change.
pub fn truncated_hook_script(cfg: &HookConfig) -> &'static str {
    // Everything up to and including the first `ops ` token: still inside the
    // second line, so the result cannot equal the whole script.
    cfg.hook_script
        .split_inclusive("ops ")
        .next()
        .unwrap_or(cfg.hook_script)
}
