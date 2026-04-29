---
id: TASK-0621
title: >-
  PERF-3: provide() clones ctx.working_directory on every call across all four
  about crates
status: Triage
assignee: []
created_date: '2026-04-29 05:21'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/lib.rs:56`

**What**: Every DataProvider::provide impl in the four about crates opens with `let cwd = ctx.working_directory.clone();` and passes &cwd downstream. Sites: extensions-go/about/src/lib.rs:56, modules.rs:23; extensions-python/about/src/lib.rs:63, units.rs:23; extensions-node/about/src/lib.rs:60, units.rs:30; extensions-java/about/src/maven/mod.rs:26, gradle.rs:24. None mutate ctx between clone and consumer — `&ctx.working_directory` (a &Path via Deref) would suffice.

**Why it matters**: PERF-3. PathBuf clone is heap allocation per provider invocation. Removing the clone costs one line and removes a subtle invitation to cwd.push(...) mutations. <!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All eight provide() sites borrow &ctx.working_directory instead of cloning
- [ ] #2 No new lifetime obstacles surface
- [ ] #3 Existing tests pass
<!-- AC:END -->
