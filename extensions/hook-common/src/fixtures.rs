//! Shared test fixtures.
//!
//! Test modules in `lib.rs`, `install.rs`, and `config.rs` previously each
//! declared their own `commit_config` / `push_config` literals; centralising
//! them here removes drift risk when `HookConfig` fields evolve.

#![cfg(test)]

use crate::HookConfig;

pub(crate) fn commit_config() -> HookConfig {
    HookConfig {
        name: "run-before-commit",
        hook_filename: "pre-commit",
        hook_script: "#!/usr/bin/env bash\nexec ops run-before-commit\n",
        skip_env_var: "SKIP_OPS_RUN_BEFORE_COMMIT",
        legacy_markers: &[
            "ops run-before-commit",
            "ops before-commit",
            "ops pre-commit",
        ],
        command_help: "Run run-before-commit checks before committing",
    }
}

pub(crate) fn push_config() -> HookConfig {
    HookConfig {
        name: "run-before-push",
        hook_filename: "pre-push",
        hook_script: "#!/usr/bin/env bash\nexec ops run-before-push\n",
        skip_env_var: "SKIP_OPS_RUN_BEFORE_PUSH",
        legacy_markers: &["ops run-before-push", "ops before-push"],
        command_help: "Run run-before-push checks before pushing",
    }
}
