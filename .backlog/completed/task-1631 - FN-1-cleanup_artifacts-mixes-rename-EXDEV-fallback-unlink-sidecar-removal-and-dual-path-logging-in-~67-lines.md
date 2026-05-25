---
id: TASK-1631
title: >-
  FN-1: cleanup_artifacts mixes rename, EXDEV fallback, unlink, sidecar removal,
  and dual-path logging in ~67 lines
status: Done
assignee:
  - TASK-1640
created_date: '2026-05-22 07:17'
updated_date: '2026-05-22 13:43'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: \`extensions/duckdb/src/ingestor.rs:295\`

**What**: \`SidecarIngestorConfig::cleanup_artifacts\` spans roughly lines 295-361 (~67 lines) and mixes five concerns at different abstraction levels:

1. Rename JSON staging file to \`.json.done\` (with cross-device fallback)
2. Emit dual-path tracing debug breadcrumb on rename failure
3. Unlink the effective post-rename path
4. NotFound recovery branch with its own debug breadcrumb
5. Permission/IO error branch with a warn breadcrumb plus sidecar policy decision

The function exceeds the FN-1 50-line guideline and operates at two abstraction levels (low-level fs syscalls + breadcrumb formatting + recovery policy). Extracting the rename-or-fallback into \`rename_to_done()\` and the unlink-with-recovery into \`unlink_and_remove_sidecar()\` would let each helper stay under 30 lines with a single concern.

**Why it matters**: Mixed abstraction levels make the recovery policy (which the doc-comment goes to great lengths to explain) harder to audit. Today it takes ~70 lines of file context to confirm that a particular crash window still calls \`remove_workspace_sidecar\`; with the helpers extracted the policy reduces to three named calls.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 cleanup_artifacts is ≤ 50 lines of body or each helper it calls is
- [ ] #2 rename-or-fallback path is reachable via a single named helper (e.g. rename_to_done)
- [ ] #3 post-rename unlink-with-recovery is reachable via a single named helper
- [ ] #4 Existing tests (cleanup_breadcrumb_*, cleanup_keeps_sidecar_when_json_removal_fails, cleanup_artifacts_clears_done_residue_left_by_prior_crash, cleanup_is_best_effort_when_json_missing) still pass
<!-- AC:END -->
