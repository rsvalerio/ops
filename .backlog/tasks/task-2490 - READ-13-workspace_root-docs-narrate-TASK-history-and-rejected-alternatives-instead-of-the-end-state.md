---
id: TASK-2490
title: 'READ-13: workspace_root docs narrate TASK history and rejected alternatives instead of the end state'
status: Done
assignee: []
created_date: '2026-10-10 15:30'
updated_date: '2026-10-10 21:40'
labels:
  - code-review
  - read
dependencies: []
parent_task_id: 'TASK-2615'
modified_files:
  - extensions-rust/cargo-toml/src/workspace_root.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/cargo-toml/src/workspace_root.rs:workspace_root'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/workspace_root.rs:10`

**What**: workspace_root.rs docs are saturated with rule/task provenance tags and decision-history narration. Locations:
- workspace_root.rs:10 — FindWorkspaceRootError doc opens "ARCH-2 / TASK-0871: typed errors ... Replaces the previously synthesised io::Error::new(NotFound, ...)".
- workspace_root.rs:53-60 — MAX_ANCESTOR_DEPTH doc narrates "TASK-0963: exposed as pub so callers and tests can ...".
- workspace_root.rs:78-104 — find_workspace_root doc tags its threat model "SEC-25 / TASK-0604 / TASK-1036" (the symlink threat model itself is enduring; the tags and journey framing are not).
- workspace_root.rs:155-164 — find_workspace_root_strict doc narrates "SEC-25 / TASK-1204: addresses the symlink-retarget gap documented on find_workspace_root".
- workspace_root.rs:166-209 — the "Scope of the guarantee (TASK-1785 / TASK-2026)" section narrates "TASK-2026 recorded the decision behind check 2", "The alternative — re-anchoring discovery to the caller's pre-canonical start — was rejected", and cites the test that drove it.
- workspace_root.rs:250-262 — strict_candidate_action doc narrates "TASK-1785: the directory rejection arms ... were previously untested ... Taking canonicalize as a parameter lets tests drive them".
- workspace_root.rs:301-309 — manifest_is_contained doc tags "SEC-25 / TASK-2026".
- workspace_root.rs:425-430 and 460-467 — manifest_declares_workspace / content_declares_workspace docs tag "SEC-11 / TASK-1781" and narrate the earlier false-negative behaviour.

**Why it matters**: READ-13 — documentation describes the end state, not the journey. This file's docs mix genuinely enduring security documentation (the symlink threat model, the two per-candidate checks and their scope) with process narration that goes stale while looking authoritative. The rewrite must keep the threat model and guarantee scope as present-tense statements and strip only the tags and decision history. The project has been stripping exactly this shape from sibling crates.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Docs in workspace_root.rs carry no RULE-ID/TASK-XXXX references and no narration of rejected alternatives or previously-untested arms
- [x] #2 The enduring security semantics survive the rewrite: symlink threat model, the two per-candidate checks and what each rejects, canonicalize-injection test seam, depth-cap rationale

<!-- AC:END -->
