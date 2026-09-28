---
id: TASK-2338
title: 'Emit a JUnit XML report of ops step results for CI test reporters'
status: Triage
assignee: []
created_date: '2026-09-28 15:25'
labels:
  - ci
  - ops-alignment
dependencies: []
modified_files:
  - crates/runner/src/display.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display.rs`

**What**: TASK-2325's survey noted ops emits no JUnit report. TASK-2325 (wave TASK-2332) added GitHub Actions groups, `::error` annotations and a `$GITHUB_STEP_SUMMARY` run summary, but no machine-readable per-step result file (e.g. `--junit <file>`) that CI test-report actions can ingest.

**Why it matters**: CI dashboards and PR check annotations from JUnit consumers cannot see which `ops verify`/`qa` steps failed without scraping logs.

**Origin**: discovered during TASK-2332 while fixing TASK-2325 (JUnit was in the task's description but outside its acceptance criteria).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A run can write a JUnit XML file with one testcase per plan step, failures carrying the message and output tail
<!-- AC:END -->
