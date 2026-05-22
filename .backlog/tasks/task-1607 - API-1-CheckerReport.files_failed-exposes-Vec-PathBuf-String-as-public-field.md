---
id: TASK-1607
title: >-
  API-1: CheckerReport.files_failed exposes Vec<(PathBuf, String)> as public
  field
status: Done
assignee:
  - TASK-1636
created_date: '2026-05-22 06:45'
updated_date: '2026-05-22 12:17'
labels:
  - code-review-rust
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/lib.rs:82`

**What**: The public field `CheckerReport.files_failed: Vec<(PathBuf, String)>` uses an unnamed `(path, message)` tuple. Callers have to remember which slot is which (`failed[i].0` vs `failed[i].1`), and the type alone does not communicate intent. The existing tests already reach into `.files_failed[0].0` (`lib.rs:201`, `lib.rs:233`) which illustrates the readability cost.

**Why it matters**: Tuple fields in a public report struct are a stable API. Once external code reads them by index, renaming/extending becomes a breaking change. A named struct (e.g. `pub struct FailedFile { pub path: PathBuf, pub message: String }`) gives callers self-documenting access, leaves room to add fields (line/col, parser kind) without breaking source compatibility, and is the idiomatic Rust choice for a > 1-element record in a public surface.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Introduce a named struct (e.g. FailedFile { path: PathBuf, message: String }) and change CheckerReport.files_failed to Vec<FailedFile>
- [x] #2 Update internal callers and tests to use named fields instead of .0/.1
- [x] #3 cargo build -p ops-config-checkers and cargo test -p ops-config-checkers pass
<!-- AC:END -->
