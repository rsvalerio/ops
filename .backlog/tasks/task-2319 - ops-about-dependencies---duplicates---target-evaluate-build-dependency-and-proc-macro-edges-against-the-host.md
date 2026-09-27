---
id: TASK-2319
title: 'ops about dependencies --duplicates --target: evaluate build-dependency and proc-macro edges against the host'
status: Triage
assignee: []
created_date: '2026-09-27 15:56'
labels:
  - code-review-rust
  - feature
  - about
  - deps
dependencies: []
modified_files:
  - extensions-rust/about/src/deps_provider.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/deps_provider.rs:edge_counts`

**What**: The platform filter evaluates every edge, including `build` dep_kinds and anything reached below a build-dependency or proc-macro, against the `--target` triples. Cargo compiles those for the host, so `--target <non-host>` can drop host-only build tooling (or keep target-gated edges beneath build scripts) that `cargo tree --target` would treat differently.

**Why it matters**: With the default (host) filter the result is exact; only an explicit non-host `--target` can misreport build-time duplicates.

**Origin**: discovered during TASK-2314 while fixing TASK-2310.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 With --target <non-host>, edges below a build-dependency or proc-macro are evaluated against the host triple, pinned by a test
<!-- AC:END -->
