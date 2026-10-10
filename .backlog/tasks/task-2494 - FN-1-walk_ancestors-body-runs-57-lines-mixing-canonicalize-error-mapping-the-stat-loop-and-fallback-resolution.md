---
id: TASK-2494
title: 'FN-1: walk_ancestors body runs ~57 lines, mixing canonicalize error mapping, the stat loop, and fallback resolution'
status: To Do
assignee: []
created_date: '2026-10-10 15:30'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - fn
dependencies: []
parent_task_id: 'TASK-2615'
modified_files:
  - extensions-rust/cargo-toml/src/workspace_root.rs
priority: medium
ordinal: 1000
dedup_key: 'FN-1:extensions-rust/cargo-toml/src/workspace_root.rs:walk_ancestors'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/workspace_root.rs:358`

**What**: `fn walk_ancestors` (workspace_root.rs:358-420) has a ~57-line body over the 50-line FN-1 threshold, and it mixes three responsibilities at different abstraction levels:
1. Canonicalizing `start` and mapping both failure modes to FindWorkspaceRootError variants (lines 363-381).
2. The ancestor loop: building each candidate path, `try_exists` with non-NotFound warn-and-skip handling, dispatching to the injected `check` closure over CandidateAction, and advancing via `Path::parent` (lines 384-411).
3. First-seen fallback resolution vs the NotFound error (lines 413-419).

**Why it matters**: FN-1 — functions should operate at a single abstraction level. Extracting the start-canonicalization block into a `canonicalize_start(start, max_depth) -> Result<PathBuf, FindWorkspaceRootError>` helper (it is a self-contained error-mapping concern with its own tracing event) leaves the loop itself under the threshold and makes the two failure paths independently testable. Consistent with prior FN-1 filings at medium (e.g. TASK-2451, TASK-2469, TASK-2471).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 walk_ancestors body is at or under 50 lines, or each extracted helper carries a single responsibility (canonicalization/error mapping vs walk loop)
- [ ] #2 Behaviour is unchanged: existing find_root tests pass without modification
<!-- AC:END -->
