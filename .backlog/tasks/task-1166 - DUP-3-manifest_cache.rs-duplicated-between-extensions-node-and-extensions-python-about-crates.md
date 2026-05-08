---
id: TASK-1166
title: >-
  DUP-3: manifest_cache.rs duplicated between extensions-node and
  extensions-python about crates
status: To Do
assignee:
  - TASK-1265
created_date: '2026-05-08 07:45'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/manifest_cache.rs:1-24` and `extensions-python/about/src/manifest_cache.rs:1-23`

**What**: The two files are byte-for-byte equivalent except for `\"package.json\"` vs `\"pyproject.toml\"` and doc strings. Each is a thin wrapper around `ops_about::manifest_cache::ArcTextCache`. Doc comments explicitly acknowledge the duplication.

**Why it matters**: Three-stack drift surface for a known-shared pattern. Collapsing to one helper in `ops_about` exposing `for_filename(&'static str)` removes the shim entirely. A future go.mod cache wrapper would be a third copy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Single ops_about::manifest_cache entry point lets a downstream crate name a filename and obtain the cached-text accessor without per-crate boilerplate
- [ ] #2 Both extensions-node/about/src/manifest_cache.rs and extensions-python/about/src/manifest_cache.rs are deleted
- [ ] #3 No behavioural change: same per-process Arc dedup, missing-file None, poison-recovery semantics
<!-- AC:END -->
