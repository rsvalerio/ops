---
id: TASK-1506
title: >-
  API-1: cargo-toml PublishSpec::is_publishable returns None for both
  workspace=true and workspace=false Inherited
status: Done
assignee:
  - TASK-1641
created_date: '2026-05-18 18:04'
updated_date: '2026-05-25 16:13'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/types.rs:241-249`

**What**: `is_publishable` returns `None` for *any* `PublishSpec::Inherited { .. }`, including `workspace: false`, which by TOML semantics is not actually requesting inheritance. The doc-comment frames the `None` case as "unresolved Inherited" but `workspace: false` is never unresolved — it's a no-op declaration cargo rejects.

**Why it matters**: API-1 / PATTERN-1: a single `None` variant carries two distinct meanings ("waiting for resolver" vs "TOML had workspace = false"). Callers gating publish on this method cannot distinguish them. The same shape leaks in `resolve_string_field`'s permissive treatment (documented), so the convention is at least consistent — but `is_publishable` should match.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either explicitly note in is_publishable's doc-comment that None covers both workspace: false and workspace: true cases, or
- [ ] #2 Return Some(true) for workspace: false (cargo's effective semantics — the field is parsed but ignored) so the no-op shape is unambiguous to callers
<!-- AC:END -->
