---
id: TASK-0833
title: >-
  SEC-31: Action::classify silently drops unknown terraform actions instead of
  surfacing them
status: Triage
assignee: []
created_date: '2026-05-02 09:11'
labels:
  - code-review-rust
  - security
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/model.rs:36-48`

**What**: Pattern matches `[s] if s == "create"` etc., and returns `None` for anything else. `classify_plan` then `filter_map`s `None` away, so a plan containing a `forget`/`import` action (Terraform >=1.5/1.7) renders as if those resources are absent.

**Why it matters**: This is "fail open" for an audit-relevant tool: an operator looking at the rendered table can miss a destructive action they cannot name. Plan diffs are safety-critical UX.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add an Action::Other(String) variant (or a sentinel Unknown) and surface it in render with a distinct color and a warning banner
- [ ] #2 Add fixtures for forget and combined-action arrays not currently enumerated
- [ ] #3 Emit a tracing::warn! with the unknown action string when classification is unrecognized
<!-- AC:END -->
