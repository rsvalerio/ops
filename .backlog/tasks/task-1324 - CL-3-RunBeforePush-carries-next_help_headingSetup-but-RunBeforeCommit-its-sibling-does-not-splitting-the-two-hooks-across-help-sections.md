---
id: TASK-1324
title: >-
  CL-3: RunBeforePush carries next_help_heading=Setup but RunBeforeCommit (its
  sibling) does not, splitting the two hooks across help sections
status: Done
assignee:
  - TASK-1382
created_date: '2026-05-11 20:56'
updated_date: '2026-05-12 22:59'
labels:
  - code-review-rust
  - cleanup
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/args.rs:101-123`

**What**: The two pre-hook variants are described as a parallel pair throughout the codebase (`pre_hook_cmd::COMMIT_OPS` / `PUSH_OPS`, `run_before_commit` / `run_before_push`, `RunBeforeCommitAction::Install` / `RunBeforePushAction::Install`, DUP-1 TASK-1282 explicitly collapsed their dispatch into one shape). But the clap attributes are asymmetric:

```rust
RunBeforeCommit {                          // no next_help_heading
    #[arg(long)] changed_only: bool,
    #[command(subcommand)] action: Option<RunBeforeCommitAction>,
},
...
#[command(next_help_heading = "Setup")]    // <-- only this one
RunBeforePush {
    #[arg(long)] changed_only: bool,
    #[command(subcommand)] action: Option<RunBeforePushAction>,
},
```

Meanwhile, `help::builtin_category` (help.rs:37-46) already classifies *both* `"run-before-commit"` and `"run-before-push"` into the `"Setup"` category. So:
- Top-level `ops --help` puts both under "Setup" via the categorized helper that splices output before `Options:`.
- Subcommand-level `ops run-before-push --help` shows a `Setup:` heading inside that subcommand's flag table (because of `next_help_heading`); `ops run-before-commit --help` does not.

**Why it matters**:
- The visual treatment of the two hooks diverges by one annotation, which a user comparing the two help screens will read as "push has some Setup mode commit doesn't" — a phantom distinction.
- The categorization is already handled centrally in `help::builtin_category`. Carrying the same intent on a per-variant clap attribute is duplication; carrying it asymmetrically is duplication that drifts.
- If `RunBeforeCommit` should also display the `Setup:` heading, the missing annotation is a regression. If neither should (because the categorized help already groups them), the `RunBeforePush` annotation is dead code that misleads readers. Either way the asymmetry is wrong.

Pick one: either drop `#[command(next_help_heading = "Setup")]` from `RunBeforePush` (the centralized categorizer is sufficient) or add it to `RunBeforeCommit` so the two hook help screens render identically.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either drop next_help_heading from RunBeforePush or add the same annotation to RunBeforeCommit so the two hook variants render symmetric help
- [ ] #2 If the categorization already lives in help::builtin_category, document why the per-variant attribute is also needed (or remove it)
- [ ] #3 Snapshot test or string assertion pins that ops run-before-commit --help and ops run-before-push --help differ only in command-name / direction wording, not section structure
<!-- AC:END -->
