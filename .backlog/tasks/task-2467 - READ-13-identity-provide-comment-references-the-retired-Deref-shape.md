---
id: TASK-2467
title: 'READ-13: identity provide comment references the retired Deref shape'
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
  - extensions-rust/about/src/identity/mod.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/about/src/identity/mod.rs:RustIdentityProvider::provide'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/identity/mod.rs:51-54`

**What**: The comment in `RustIdentityProvider::provide` mixes the current contract with a comparative to the old design: "OWN-12 / TASK-1767: named accessors, not `Deref` into the raw `CargoToml`. The manifest's `[workspace].members` still holds the unexpanded glob spec, and the type **no longer puts it one field access away** from the resolved list." "No longer" forces the reader to reconstruct the retired `Deref` impl to parse the sentence.

**Why it matters**: READ-13: the clause a newcomer would delete verbatim ("no longer ...") is process narration; the end-state fact is that `LoadedManifest` exposes the spec and the resolved list through separate named accessors. `identity/mod.rs` was outside the TASK-2374 / TASK-2426 cleanup scopes.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Comment states the accessor separation contract without the 'no longer puts it one field access away' comparative to the removed Deref shape
<!-- AC:END -->
