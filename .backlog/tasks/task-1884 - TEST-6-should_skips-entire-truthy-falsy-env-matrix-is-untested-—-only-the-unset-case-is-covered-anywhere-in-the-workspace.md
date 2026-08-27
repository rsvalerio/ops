---
id: TASK-1884
title: >-
  TEST-6: should_skip's entire truthy/falsy env matrix is untested — only the
  unset case is covered anywhere in the workspace
status: Triage
assignee: []
created_date: '2026-08-27 15:33'
labels:
  - code-review-rust
  - tests
dependencies: []
modified_files:
  - extensions/hook-common/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/lib.rs:88-99` (`should_skip`), test at `extensions/hook-common/src/lib.rs:167-173`

**What**: `should_skip` is the operator's opt-out for both git hooks:

```rust
pub fn should_skip(config: &HookConfig) -> bool {
    std::env::var(config.skip_env_var)
        .is_ok_and(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
}
```

Its doc comment states a precise contract: four accepted tokens, case-insensitive, and *everything else* — including the empty string, `"0"`, `"false"`, and arbitrary text — means "don't skip". The only test that exists is `should_skip_returns_false_by_default`, which removes the variable and asserts `false`. Grepping the whole workspace for `SKIP_OPS` confirms nothing else covers it: `extensions/run-before-commit/src/lib.rs:120` has the same unset-only test, and `crates/cli/src/subcommands.rs:532` only *unsets* the variable to keep an unrelated test hermetic.

So none of the following is pinned anywhere:

- `SKIP_OPS_RUN_BEFORE_COMMIT=1` / `true` / `yes` / `on` -> `true` (the documented happy path — the feature itself)
- `TRUE`, `Yes`, `On` -> `true` (the case-insensitivity the doc promises)
- `0`, `false`, `no`, `off`, `""`, `maybe` -> `false`

**Why it matters**: this is a two-sided silent failure on a gate. If the accepted set ever narrows (a refactor to `v == "1"`), `SKIP=true` stops working and the operator's escape hatch disappears with no error. If it ever widens (a refactor to `!v.is_empty()`, or to a generic truthiness helper that treats `"0"`/`"false"` as set-therefore-true), then `SKIP_OPS_RUN_BEFORE_COMMIT=false` — the spelling an operator would naturally reach for to *re-enable* the hook — silently disables every pre-commit and pre-push check, and nothing in the suite notices. The rejection half of the contract is exactly the half no current test touches. `EnvGuard` + `#[serial_test::serial]` already exist in this crate (`test_helpers.rs`), so the table test costs a few lines.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test asserts should_skip returns true for each documented token: 1, true, yes, on
- [ ] #2 A test asserts case-insensitivity for at least TRUE / Yes / ON
- [ ] #3 A test asserts should_skip returns false for set-but-falsy values: 0, false, the empty string, and an arbitrary string such as maybe
- [ ] #4 The new tests use EnvGuard and #[serial_test::serial] so they cannot race other env-mutating tests
<!-- AC:END -->
