---
id: TASK-0617
title: >-
  ARCH-11: extensions-java/about dev-dependencies missing ops-extension
  test-support feature flag
status: Triage
assignee: []
created_date: '2026-04-29 05:21'
labels:
  - code-review-rust
  - ARCH
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/Cargo.toml:18`

**What**: extensions-java/about/Cargo.toml lists only `tempfile = { workspace = true }` under [dev-dependencies], while sister crates (extensions-go/about, extensions-python/about, extensions-node/about) declare `ops-extension = { workspace = true, features = ["test-support"] }` to enable Context::test_context(...). Java tests instead reach for ops_core::config::Config and Context::new (maven/mod.rs:67-70).

**Why it matters**: Divergent test-context construction across four parallel crates makes test refactors more expensive.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extensions-java/about/Cargo.toml adds ops-extension with test-support feature to [dev-dependencies]
- [ ] #2 Java tests migrated to Context::test_context
- [ ] #3 All four about crates use the same test-context construction
<!-- AC:END -->
