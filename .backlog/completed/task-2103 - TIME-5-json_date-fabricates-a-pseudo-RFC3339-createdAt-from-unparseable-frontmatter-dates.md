---
id: TASK-2103
title: 'TIME-5: json_date fabricates a pseudo-RFC3339 createdAt from unparseable frontmatter dates'
status: Done
assignee: []
created_date: '2026-09-08 06:42'
updated_date: '2026-09-09 18:36'
labels:
  - code-review-rust
  - time
dependencies: []
parent_task_id: 'TASK-2243'
modified_files:
  - crates/backlog/src/render.rs
priority: low
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/render.rs:44`

**What**: `json_date(raw)` re-shapes the raw frontmatter string by blind string surgery (`replace(' ', "T")` + appending `:00Z`/`Z`) with no validation. For a task whose `created_date`/`updated_date` does not carry the expected shape (the parser accepts any scalar), it emits a value that looks like a timestamp but is garbage: `json_date("back then")` produces `"backTthenZ"`, which lands in the `createdAt`/`updatedAt` fields of the `task view --json` and `task list --json` envelopes.

**Why it matters**: TIME-5 — a timestamp crossing a boundary must be unambiguous. Consumers of the JSON contract parse `createdAt` as an RFC-3339-ish instant; a fabricated pseudo-timestamp is worse than a null because it fails downstream parsing at a distance from its cause. The crate already has the correct validator one module away (`cleanup::parse_frontmatter_date`, which understands both the HH:MM and HH:MM:SS forms); routing json_date through a real parse (falling back to null or the raw string when unparseable) reuses it and keeps the emitted field honest.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 createdAt/updatedAt in the JSON envelopes are emitted only from a frontmatter date that actually parses, with a defined fallback (null or raw string) otherwise
- [x] #2 A test pins the unparseable-date behaviour (today it would emit backTthenZ)

<!-- AC:END -->
