---
id: TASK-0923
title: 'DUP-1: Maven is_project_open and is_project_open_start duplicate prefix logic'
status: Triage
assignee: []
created_date: '2026-05-02 10:12'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:133`

**What**: is_project_open and is_project_open_start both strip_prefix("<project") and check that the next char is whitespace; they differ only in whether `>` appears on the line. A single helper returning (matches, has_close) eliminates the divergence risk if the Maven opener shape grows another exception (e.g. <project/>).

**Why it matters**: Two near-identical predicates is exactly the DUP-1 pattern the project flags elsewhere; keeping them in sync for future changes is unnecessary cognitive cost.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 One helper returns the (matched, closed) tuple; both predicates derive from it
- [ ] #2 Existing tests for both functions still pass
<!-- AC:END -->
