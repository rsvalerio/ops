---
id: TASK-2409
title: 'READ-4: load_with_sidecar docs say the .done rename is not implemented, but cleanup_artifacts does it'
status: To Do
assignee: []
created_date: '2026-10-04 14:18'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2416'
modified_files:
  - extensions/sqlite/src/ingestor.rs
priority: low
ordinal: 1000
dedup_key: 'READ-4:extensions/sqlite/src/ingestor.rs:SidecarIngestorConfig::load_with_sidecar'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/ingestor.rs:127-175` (`SidecarIngestorConfig::load_with_sidecar` docs), `ingestor.rs:359` (`rename_json_to_done`)

**What**: The docs state that option B (renaming the JSON to a `.done` suffix) "is *not* implemented today" and list step 7 as `remove(json_path)`. In fact `cleanup_artifacts` calls `rename_json_to_done` and then unlinks the effective path. The helper docs also number the steps "Step 1..4" while `load_with_sidecar` documents steps 1-8.

**Why it matters**: The documented crash-recovery contract contradicts the code. A reader will wrongly assume a crash leaves a `*.json` file rather than `*.json.done`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 load_with_sidecar docs describe the rename-to-.done then unlink behaviour and the post-crash state it leaves
- [ ] #2 Step numbering in the docs of load_with_sidecar and its helpers is consistent
<!-- AC:END -->
