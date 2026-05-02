---
id: TASK-0921
title: 'ERR-7: terraform cleanup_artifacts logs through ui::note instead of tracing'
status: Triage
assignee: []
created_date: '2026-05-02 10:12'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:202`

**What**: cleanup_artifacts emits `could not remove ...` via ops_core::ui::note, which prints to stdout/stderr through the user-facing UI helper. This contradicts the project pattern (sibling MetadataIngestor::load uses tracing::warn for the analogous best-effort cleanup) and makes the message non-routable for log capture.

**Why it matters**: Inconsistent diagnostic surface — best-effort cleanup failures should not appear in the user UI stream and should be machine-greppable via tracing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace ui::note with tracing::warn, retaining the path and error context
- [ ] #2 Verify no test relies on stdout containing the message
<!-- AC:END -->
