---
id: TASK-1083
title: >-
  PATTERN-1: extensions-node detect_package_manager returns None for unknown
  packageManager values, skipping lockfile probe
status: Done
assignee: []
created_date: '2026-05-07 21:21'
updated_date: '2026-05-08 06:16'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_manager.rs:13-22`

**What**: When `packageManager` is set to a non-empty value the parser doesn't recognise (e.g., `"deno"`, `"yarn-pnp@4"`, a typo like `"pnmp"`), the function returns `None` immediately — skipping the lockfile-probe fallback that the docstring's empty/whitespace branch explicitly delegates to.

**Why it matters**: An unknown label is informationally equivalent to "no useful hint" — the same condition the empty-string branch already handles by falling through. Treating "unknown" differently from "empty" is a silent failure mode that hides a real `pnpm-lock.yaml` from the About card whenever a workspace pins a tool we don't enumerate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Unknown packageManager values fall through to lockfile probing
- [ ] #2 Unit test pins Some("deno@2.0.0") + pnpm-lock.yaml present → Some("pnpm")
- [ ] #3 Docstring updated to state 'unknown values are treated as unset, mirroring whitespace-only'
<!-- AC:END -->
