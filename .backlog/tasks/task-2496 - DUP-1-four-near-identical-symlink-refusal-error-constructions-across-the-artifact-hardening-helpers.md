---
id: TASK-2496
title: 'DUP-1: four near-identical symlink-refusal error constructions across the artifact hardening helpers'
status: To Do
assignee: []
created_date: '2026-10-10 15:31'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - duplication
dependencies: []
parent_task_id: 'TASK-2622'
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: medium
ordinal: 1000
dedup_key: 'DUP-1:extensions-terraform/plan/src/lib.rs:lib'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:510` (plus 541, 628, 643)

**What**: The same io::Error construction — ErrorKind::InvalidInput plus a format! message of the shape "<what> {} is a symlink; refusing to <verb> ..." naming path.display() — is written out four times:

- extensions-terraform/plan/src/lib.rs:510-520 — create_artifact_dir, non-unix arm
- extensions-terraform/plan/src/lib.rs:541-549 — verify_artifact_dir, unix symlink branch
- extensions-terraform/plan/src/lib.rs:628-638 — write_plan_json, non-unix pre-open probe
- extensions-terraform/plan/src/lib.rs:643-651 — write_plan_json, unix ELOOP arm

Each block is 9-10 near-identical lines differing only in the noun ("artifact directory" / "plan JSON") and the verb phrase ("refusing to stage plan artifacts through it" / "refusing to write through it").

**Why it matters**: DUP-1 (identical blocks of 5+ lines). Four hand-maintained copies of a security-relevant rejection error is where wording and ErrorKind drift creeps in silently: a future edit to one copy (a different ErrorKind, a clearer message) leaves the other three behind, and the symlink defences are exactly the code whose messages a triager must be able to grep uniformly. A single helper — fn symlink_error(path: &Path, what: &str, refusing: &str) -> io::Error (or an Option<io::Error> probe helper for the two pre-check arms) — states the invariant once.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A single helper constructs the symlink-refusal io::Error, and all four sites (create_artifact_dir non-unix, verify_artifact_dir unix, write_plan_json non-unix probe, write_plan_json unix ELOOP arm) route through it
- [ ] #2 The user-visible message still names the path, whether it is the artifact directory or the plan JSON, and the refusal verb, per the existing tests
<!-- AC:END -->
