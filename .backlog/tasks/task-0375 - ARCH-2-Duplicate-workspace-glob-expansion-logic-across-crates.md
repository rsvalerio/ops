---
id: TASK-0375
title: 'ARCH-2: Duplicate workspace-glob expansion logic across crates'
status: To Do
assignee:
  - TASK-0417
created_date: '2026-04-26 09:38'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:7`

**What**: resolve_member_globs is a hand-rolled glob expander supporting only a leading * with no recursion or escape handling. The metadata crate already returns the canonical fully-resolved member list — but the about provider re-parses Cargo.toml itself.

**Why it matters**: Two sources of truth for "what are the workspace members" (cargo metadata vs hand-rolled glob). Member sets diverge for advanced patterns like crates/**/foo, [workspace] exclude, env-driven configs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either replace resolve_member_globs with calls into MetadataProvider, or document why the Rust about extension intentionally avoids the metadata data provider
- [ ] #2 Add a regression test where [workspace] exclude is set and verify units provider honors it
<!-- AC:END -->
