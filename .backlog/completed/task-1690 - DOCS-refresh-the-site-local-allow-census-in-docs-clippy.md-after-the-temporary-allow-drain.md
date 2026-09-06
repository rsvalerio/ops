---
id: TASK-1690
title: >-
  DOCS: refresh the site-local allow census in docs/clippy.md after the
  temporary-allow drain
status: Done
assignee:
  - TASK-1988
created_date: '2026-08-26 21:54'
updated_date: '2026-08-28 14:54'
labels:
  - code-review-rust
  - documentation
dependencies: []
modified_files:
  - docs/clippy.md
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `docs/clippy.md`

**What**: The "Current site-local allows" census table still describes the
pre-drain state (`too_many_arguments` 5, `trivially_copy_pass_by_ref` 3,
`unnecessary_wraps` 2, …). Clearing the `# --- Temporary allows ---` block moves
a lint's exception from layer 1 to layer 3 wherever the code genuinely needs it,
so every wave in the TASK-1683..TASK-1689 batch adds site-local `#[allow]`s that
the census does not list. Wave 143 alone added roughly fourteen: `expect_used`
(poisoned locks, static templates/aliases/directives, caches populated by
construction, plus a file-level allow on `crates/cli/tests/integration.rs`),
`future_not_send` (two runner futures), `needless_collect` (three
borrow/concurrency-load-bearing collects), `panic_in_result_fn` (two duckdb test
mocks), and `literal_string_with_formatting_args` (four indicatif templates).

**Why it matters**: The census exists to spot when one category grows enough to
deserve a policy decision instead of N scattered exceptions. A stale table
cannot do that job, and the drain is exactly the event that makes the question
worth asking again.

**Origin**: discovered during TASK-1685 (wave143) while fixing TASK-1675 and
TASK-1682. Deliberately not fixed in-wave: six sibling waves are adding
site-local allows concurrently, so any count written now is wrong before it
lands. Refresh once, after the whole `code-review/run-20260826` batch merges.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The census table in docs/clippy.md reflects the site-local allows actually present after the temporary-allow block is empty
- [x] #2 Any category that grew past a handful of sites is either justified as policy or called out as needing one
<!-- AC:END -->
