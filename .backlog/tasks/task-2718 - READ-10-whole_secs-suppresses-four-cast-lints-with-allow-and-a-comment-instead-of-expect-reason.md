---
id: TASK-2718
title: 'READ-10: whole_secs suppresses four cast lints with #[allow] and a comment instead of #[expect(reason)]'
status: To Do
assignee: []
created_date: '2026-11-18 18:32'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2845'
modified_files:
  - crates/theme/src/step_line_theme.rs
priority: low
ordinal: 1000
dedup_key: 'review-2667.06-7'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/step_line_theme.rs:82` (`whole_secs`)

**What**: The guard-bounded `f64` -> `u64` conversion in `whole_secs` is wrapped in `#[allow(clippy::as_conversions, clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]`, with the justification in an 8-line comment above it. This is a suppression documenting a specific known violation, which READ-10 says should be `#[expect(..., reason = "...")]`.

**Why it matters**: With `#[allow]`, if any of the four lints stops firing (e.g. the conversion is rewritten, or `as_conversions` policy changes in `[workspace.lints]`), the suppression silently outlives its cause. `#[expect]` warns via `unfulfilled_lint_expectations`, and the `reason` field carries the rationale in a form tooling surfaces. The lints fire unconditionally (no cfg dependence), so `expect` has no footgun here.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 whole_secs uses #[expect(<only the lints that actually fire>, reason = "...")] in place of #[allow(...)], with the rationale condensed into the reason
- [ ] #2 cargo clippy for ops-theme passes with no unfulfilled_lint_expectations warning
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.06 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
