---
id: TASK-0871
title: >-
  ARCH-2: extensions-rust find_workspace_root synthesizes io::Error::NotFound,
  defeating downstream e.kind() checks
status: Triage
assignee: []
created_date: '2026-05-02 09:22'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:286-317`

**What**: fs::canonicalize(start).with_context(...) returns the underlying IO error. After the walk the function fabricates a std::io::Error::new(NotFound, ...). Two different error shapes for two different reasons (canon failure vs no Cargo.toml found) is correct, but the NotFound payload here is a string-wrapped synthetic error, defeating downstream e.kind() checks (e.g. is_manifest_missing in query.rs:30-39).

**Why it matters**: is_manifest_missing walks e.source() looking for std::io::Error::NotFound. The synthetic IoError is wrapped via .into() into anyhow::Error; the kind check works only because the synthetic error sits at the chain root. If a future caller wraps additional context, the heuristic flips silently.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace synthetic IoError with a typed error variant (thiserror enum: NotFound, CanonicalizeFailed)
- [ ] #2 is_manifest_missing matches on the typed variant rather than io::ErrorKind chain-walk
- [ ] #3 Test covers both shapes (no Cargo.toml; canonicalize on missing path)
<!-- AC:END -->
