---
id: TASK-2540
title: 'FN-1: decode_diagnostic in parse/deny.rs runs to 68 code lines across three abstraction levels'
status: Done
assignee: []
created_date: '2026-10-10 15:37'
updated_date: '2026-10-10 21:45'
labels:
  - code-review
  - fn
dependencies: []
parent_task_id: 'TASK-2618'
modified_files:
  - extensions-rust/deps/src/parse/deny.rs
priority: medium
ordinal: 1000
dedup_key: 'FN-1:extensions-rust/deps/src/parse/deny.rs:decode_diagnostic'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/deny.rs:343`

**What**: `decode_diagnostic` is 68 code lines (excluding comments and blanks), well past the FN-1 threshold of 50. It mixes three abstraction levels in one body: envelope JSON decoding and the `envelopes_seen`/`error_log` bookkeeping, `fields` decoding with its drift breadcrumbs, and the missing-`code` / missing-`severity` fallback ladders with their `tracing` events.

**Why it matters**: The function is the crate's main cargo-deny drift-detection surface; its length makes the counting protocol (which line increments which counter, and in which order) hard to verify against the guards in `check_partial_decode_loss` that depend on it. Extracting the severity-substitution block and the envelope-decode step into named helpers would put each at one abstraction level (FN-1) without changing behaviour.

<!-- scan confidence: measured 68 non-comment lines; single enclosing item -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 decode_diagnostic body is at most 50 code lines, or the excess is justified by an explicit comment naming the state-machine/exception rationale
- [x] #2 Counter-increment sites (envelopes_seen, candidate_diagnostics) remain in the same relative order, verified by the existing deny tests passing unchanged

<!-- AC:END -->
