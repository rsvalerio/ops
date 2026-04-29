---
id: TASK-0611
title: >-
  API-9: UpgradeEntry, AdvisoryEntry, DenyEntry pub structs lack
  #[non_exhaustive]
status: Triage
assignee: []
created_date: '2026-04-29 05:20'
labels:
  - code-review-rust
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:33`

**What**: TASK-0468/0435 already filed non_exhaustive gaps in sibling crates and TASK-0557 raised it for InheritanceError. Same gap exists for public deps types: UpgradeEntry, AdvisoryEntry, DenyEntry, UpgradeResult, DenyResult, DepsReport. Adding a field for new cargo-deny output (e.g. CVE id alongside advisory id) is a breaking change.

**Why it matters**: Same justification as TASK-0435 — these are the data-provider contract for `ops deps`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 #[non_exhaustive] added to public structs and constructors updated where needed
<!-- AC:END -->
