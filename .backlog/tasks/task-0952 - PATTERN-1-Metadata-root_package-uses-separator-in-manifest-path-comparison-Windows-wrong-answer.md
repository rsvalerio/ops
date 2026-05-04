---
id: TASK-0952
title: >-
  PATTERN-1: Metadata::root_package uses '/' separator in manifest path
  comparison (Windows wrong-answer)
status: Triage
assignee: []
created_date: '2026-05-04 21:45'
labels:
  - code-review-rust
  - correctness
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/types.rs:254-256`

**What**: `root_package` formats the expected manifest path as `format!("{}/Cargo.toml", ws_root)` with a hardcoded `/`, then compares against `manifest_path()` from cargo metadata. On Windows, cargo emits backslash-separated `manifest_path` values, so the string compare never matches and `root_package()` always returns `None`.

**Why it matters**: Silent wrong answer on Windows: any consumer of `root_package()` gets `None` for a perfectly valid workspace, masquerading workspaces as virtual.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Compare via PathBuf join (Path::new(p.manifest_path()) == Path::new(&ws_root).join("Cargo.toml")) so platform separators line up
- [ ] #2 Regression test exercises comparison via paths with backslash separator, or uses Path-based equivalence assertion
<!-- AC:END -->
