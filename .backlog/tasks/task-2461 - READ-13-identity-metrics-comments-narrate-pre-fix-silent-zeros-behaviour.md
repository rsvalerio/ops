---
id: TASK-2461
title: 'READ-13: identity metrics comments narrate pre-fix silent-zeros behaviour'
status: To Do
assignee: []
created_date: '2026-10-10 15:27'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - READ
dependencies: []
parent_task_id: 'TASK-2614'
modified_files:
  - extensions-rust/about/src/identity/metrics.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/about/src/identity/metrics.rs:metrics'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/identity/metrics.rs:17-20`, `extensions-rust/about/src/identity/metrics.rs:42-45`

**What**: Two comments narrate past behaviour instead of the current contract:
- `query_identity_metrics` doc (17-20) ends with provenance of another crate's fix: "Same anti-pattern that `about/units::enrich_from_db` got fixed for."
- The module-level `//` comment (42-45): "A schema mismatch or migration bug **used to render as silent zeros** because all four call sites used `.ok()` / `.unwrap_or_default()` without any signal." The current contract is the positive statement: every SQLite query lookup logs at warn before falling back.

**Why it matters**: READ-13: the "used to render as silent zeros" sentence is a process artifact of the ERR-2 / TASK-0376 fix; it goes stale on the next change to these call sites while looking authoritative. `identity/metrics.rs` was outside the TASK-2374 / TASK-2424/2426 cleanup scopes, so the narration survived.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both comments state the current contract (handle resolved once and threaded; every query failure warns before its fallback) in present tense with no 'used to render'/'got fixed for' narration
<!-- AC:END -->
