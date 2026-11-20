---
id: TASK-2812
title: 'READ-11: Name the 96000-byte pre-push capture budget and pin its coupling to MAX_REF_UPDATE_LINES'
status: Done
assignee: []
created_date: '2026-11-18 18:46'
updated_date: '2026-11-18 23:00'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2862'
modified_files:
  - extensions/run-before-push/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-11:extensions/run-before-push/src/lib.rs:HOOK_SCRIPT'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-push/src/lib.rs:92-110` (HOOK_SCRIPT tail), `:144-147` (REF_UPDATES_ENV_VAR doc), `:162-171` (MAX_REF_UPDATE_LINES doc); tests at `:574`, `:622`

**What**: The hook's capture budget (96000 bytes, probed as 96001) is an inline literal repeated five times inside the HOOK_SCRIPT string (`head -c 96001`, `-gt 96001`, two comment lines, the stderr message) plus in two doc comments and two test assertions. There is no named Rust `const` for it, and nothing ties it to `MAX_REF_UPDATE_LINES`, whose doc argues "both bounds are live" from the 96000 figure. That argument also only holds for SHA-1 repos: a SHA-256 ref line carries two 64-hex oids (>= ~134 bytes), so 1000 lines need >= 134 KB and the line bound can never be reached under the 96000-byte budget — the doc's claim is silently false there.

**Why it matters**: The budget is pinned to an external limit (execve's 131072-byte per-string cap) and is load-bearing for the fail-closed truncation logic. Changing it in one spot (script vs docs vs tests vs the line bound) leaves the others stale with no test failing, and the line-bound rationale is folklore rather than an enforced invariant.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The capture budget is a named const (e.g. CAPTURE_BUDGET_BYTES) with a doc comment naming the execve per-string cap it is pinned to
- [x] #2 A test asserts HOOK_SCRIPT contains the head -c / -gt probe values derived from that const, so script and const cannot drift
- [x] #3 MAX_REF_UPDATE_LINES doc (or a test) states the relation to the budget correctly for both SHA-1 and SHA-256 oid lengths

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.29 (code-review-rust run, 2026-11-18).

CAPTURE_BUDGET_BYTES now also feeds a compile-time assertion that the forwarded env string stays under EXECVE_MAX_ARG_STRLEN (131072).

<!-- SECTION:NOTES:END -->
