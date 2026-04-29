---
id: TASK-0627
title: >-
  PATTERN-1: detect_package_manager ignores corepack packageManager empty-string
  value
status: Triage
assignee: []
created_date: '2026-04-29 05:22'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_manager.rs:11`

**What**: detect_package_manager treats any non-empty packageManager field as authoritative. An empty `"packageManager": ""` value enters `if let Some(pm) = ...` with pm="" and immediately returns None, bypassing all lockfile probes. PEP-621/npm spec treats empty string as effectively unset; should fall back to lockfile probing.

**Why it matters**: package.json with empty packageManager (real corepack-disable pattern) suppresses lockfile-based detection — About card shows "Node X" instead of "Node X · pnpm".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 detect_package_manager treats Some("") (and pure-whitespace) same as None and falls through to lockfile probe
- [ ] #2 Unit test covers empty-string packageManager field
<!-- AC:END -->
