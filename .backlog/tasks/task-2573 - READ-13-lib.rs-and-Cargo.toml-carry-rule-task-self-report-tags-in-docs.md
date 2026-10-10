---
id: TASK-2573
title: 'READ-13: lib.rs and Cargo.toml carry rule/task self-report tags in docs'
status: To Do
assignee: []
created_date: '2026-10-10 15:43'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - read
dependencies: []
parent_task_id: 'TASK-2624'
modified_files:
  - extensions/hook-common/src/lib.rs
  - extensions/hook-common/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/hook-common/src/lib.rs:lib module docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/lib.rs:260,273,284`; candidates in inline comments and `extensions/hook-common/Cargo.toml:11,22,31`

**What**: Three test docs in lib.rs open with review-rule self-report prefixes ('/// TEST-6: the accepted-token set...', 260, 273, 284), plus an inline '// -- hook_script! prologue (DUP-1) --' marker (298). Cargo.toml comments carry the same artifact with task IDs: 'SEC-25 / TASK-1210:' (11), 'TEST-19:' (22), 'DRY-1 / TASK-2034:' (31). The repo stripped exactly these prefixes (tag dropped, prose kept) from crates/theme in e5b5443f and from module docs in 1d9bc644.

<!-- scan confidence: candidates to inspect -->
**Why it matters**: READ-13 — self-reports of which guidelines or tasks a change followed are process artifacts: meaningless to a reader using the crate and stale on the next change while looking authoritative. The behavioral prose (why the bypass precedes the probe, what the test pins) is end-state content and stays.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Doc comments in lib.rs no longer open with rule-ID prefixes
- [ ] #2 Cargo.toml comments state the dependency rationale without SEC-/TEST-/DRY-/TASK- tags
- [ ] #3 cargo test -p ops-hook-common passes
<!-- AC:END -->
