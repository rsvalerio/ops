---
id: TASK-0565
title: >-
  PATTERN-1: parse_pnpm_workspace_yaml silently ignores inline-list packages
  flow form
status: Triage
assignee: []
created_date: '2026-04-29 05:03'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/units.rs:148-179`

**What**: The naive YAML parser only recognises the block-list shape packages: with newline and indented dash-prefixed entries. The flow form packages: with bracketed inline values is explicitly skipped via the !line.contains opening-bracket guard, so workspaces declared inline yield zero members and the units card silently shows no packages.

**Why it matters**: pnpm accepts both shapes; users hitting the inline form get no diagnostic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Inline-list form is parsed, or surfaced via tracing::warn as unsupported
- [ ] #2 A unit test covers the inline list form returning the expected globs
<!-- AC:END -->
