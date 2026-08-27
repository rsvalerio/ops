---
id: TASK-1916
title: >-
  READ-5: SKIP_ENV_VAR's doc says the hook skips 'when set to 1', re-stating the
  exact wrong contract TASK-0224 fixed in should_skip
status: Triage
assignee: []
created_date: '2026-08-27 15:41'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions/run-before-commit/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:46-47`

**What**:

```rust
/// Environment variable that skips the run-before-commit check when set to "1".
pub const SKIP_ENV_VAR: &str = "SKIP_OPS_RUN_BEFORE_COMMIT";
```

The actual predicate this constant feeds (`extensions/hook-common/src/lib.rs:88-99`) accepts four tokens, case-insensitively:

```rust
pub fn should_skip(config: &HookConfig) -> bool {
    std::env::var(config.skip_env_var)
        .is_ok_and(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
}
```

and its own doc says, explicitly: *"documenting only `\"1\"` previously surprised users who set `SKIP_OPS_RUN_BEFORE_COMMIT=true`"* — the widening was TASK-0224 (`READ-5: should_skip only treats literal 1 as skip — undocumented`, Done). The constant that names the variable in the crate a user actually greps for still carries the pre-fix wording, so the fix is only half-landed: the behaviour widened, this doc did not.

The operator-facing message compounds it. `crates/cli/src/subcommands.rs:213-217` prints:

```rust
ops_core::ui::note(format!("[{}] {}=1 — skipping", hook.hook_name, hook.skip_env_var));
```

so a user who set `SKIP_OPS_RUN_BEFORE_COMMIT=true` is told `SKIP_OPS_RUN_BEFORE_COMMIT=1 — skipping`, reporting a value they did not set. On the hook path that note is the only evidence that a commit bypassed its checks; misreporting the trigger makes it useless for answering "why did this commit skip the gate?"

The sibling crate carries the identical stale line (`extensions/run-before-push/src/lib.rs:36`); filed here per crate scope.

**Why it matters**: READ-5 — invariants must be stated accurately where a reader will look for them. This is the documented escape hatch for a gate that blocks commits; a user who reads the constant's doc concludes `=true` does not work and reaches for `--no-verify` instead, which disables *every* hook rather than this one. Meanwhile a reader who trusts the doc could "simplify" `should_skip` back to `v == "1"` and believe they were matching the specification.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 SKIP_ENV_VAR's doc comment lists the accepted values (1, true, yes, on) and states that matching is case-insensitive, or defers to should_skip's doc by link
- [ ] #2 The CLI skip note reports the value the operator actually set instead of hardcoding =1
<!-- AC:END -->
