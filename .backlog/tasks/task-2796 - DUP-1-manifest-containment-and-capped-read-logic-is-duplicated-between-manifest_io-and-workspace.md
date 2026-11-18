---
id: TASK-2796
title: 'DUP-1: manifest containment-and-capped-read logic is duplicated between manifest_io and workspace'
status: To Do
assignee: []
created_date: '2026-11-18 18:45'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - DUP
dependencies: []
parent_task_id: 'TASK-2858'
modified_files:
  - extensions/about/src/manifest_io.rs
  - extensions/about/src/workspace.rs
priority: medium
ordinal: 1000
dedup_key: 'DUP-1:extensions/about/src/manifest_io.rs:read_optional_text'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/manifest_io.rs:47-89` (`read_optional_text`, `warn_failed_to_read`) and `extensions/about/src/workspace.rs:246-305` (`Resolver::read_member_manifest`, `warn_manifest_unreadable`)

**What**: Both functions perform the same ~25-line security-sensitive sequence: `canonicalize(path)` with `NotFound` → silent `None` and other errors → warn; fail-closed containment `canonical.starts_with(canonical_root)` with a warn; `ops_core::text::read_capped_to_string(&canonical)` with `NotFound` → `None` and other errors → warn. The two warn helpers are also identical apart from the message text. The only real difference is that `Resolver` memoises the canonical root; manifest_io's docs even say it is "mirroring the workspace-member read".

**Why it matters**: This is the crate's path-containment boundary for untrusted checkouts. Two copies mean a hardening fix (e.g. a new refusal case or a changed log field) must be made twice and can silently drift, which DUP-1 exists to prevent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A single helper (e.g. manifest_io::read_contained(path, canonical_root, kind)) implements canonicalize + containment + capped read, and both call sites use it
- [ ] #2 Existing containment, FIFO and NotFound tests in both modules still pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.22 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
