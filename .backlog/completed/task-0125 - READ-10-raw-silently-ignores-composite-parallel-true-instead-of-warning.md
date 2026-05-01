---
id: TASK-0125
title: 'READ-10: --raw silently ignores composite parallel = true instead of warning'
status: Done
assignee: []
created_date: '2026-04-20 19:35'
updated_date: '2026-04-20 20:45'
labels:
  - rust-code-review
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:77-91` (raw-mode branch in `run_commands`)

**What**: When `--raw` is set, the code computes `any_parallel` from the plan but then does `let _ = any_parallel;` and runs sequentially. Users who have `parallel = true` on a composite will see their commands serialized with no feedback. The only mention of this behavior is a single sentence in the `--raw` help text ("Composite commands run sequentially under `--raw` (parallel is ignored)") which most users will not read when diagnosing a slow run.

**Why it matters**: Silent behavioral downgrades based on flag combinations are a reliability footgun — a user adds `--raw` to get a TUI passthrough and their CI time doubles with no signal. A one-line `eprintln!`/tracing warning when `any_parallel && raw` is a cheap mitigation that respects the existing "raw produces no ops output" contract if routed through `tracing::warn!` (visible only with `--verbose`/`RUST_LOG`).

**Scope note**: the `let _ = any_parallel;` binding exists only to silence the unused warning after the refactor — that itself is a smell suggesting the value should either be acted on or not computed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either emit a tracing::warn! when a composite requested parallel execution but --raw forced sequential, or remove the any_parallel computation in the raw branch entirely
- [ ] #2 If a warning is emitted, it goes through tracing (not println/eprintln) so --raw's silent-output contract is preserved by default
- [ ] #3 The  discard is gone
<!-- AC:END -->
