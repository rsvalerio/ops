---
id: TASK-2307
title: 'Adopt ops backlog wave create in the code-review-triage skill'
status: Triage
assignee: []
created_date: '2026-09-26 20:25'
labels:
  - feature
  - backlog
  - waves
dependencies: []
modified_files: []
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `dev-skills` plugin, `skills/code-review-triage/SKILL.md` (outside this repo)

**What**: `ops backlog wave create <title> --members <ids>` now creates the wave parent (marker label, dependencies) and links every member (parent_task_id, status To Do) in one validated step. The code-review-triage skill still issues one `task create` plus one `task edit` per member.

**Why it matters**: until the skill switches, the N+1-write failure mode TASK-2296 removed (a member missing its parent link or status flip) is still live in practice. The skill lives in rsvalerio/ai, so this likely moves there like TASK-2295.

**Origin**: discovered during TASK-2305 while fixing TASK-2296.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 code-review-triage creates each wave with ops backlog wave create --members (falling back to task create/edit only for an ops binary without it)
<!-- AC:END -->
