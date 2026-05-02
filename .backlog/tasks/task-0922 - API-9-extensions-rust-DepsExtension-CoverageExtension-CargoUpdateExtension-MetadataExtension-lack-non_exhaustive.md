---
id: TASK-0922
title: >-
  API-9: extensions-rust
  DepsExtension/CoverageExtension/CargoUpdateExtension/MetadataExtension lack
  #[non_exhaustive]
status: Done
assignee: []
created_date: '2026-05-02 10:12'
updated_date: '2026-05-02 11:27'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:291`

**What**: DepsExtension (deps/src/lib.rs:291), CoverageExtension (test-coverage/src/lib.rs:28), CargoUpdateExtension (cargo-update/src/lib.rs:263), and MetadataExtension (metadata/src/lib.rs:47) are public unit-like structs constructed by callers via positional `Self`. Adding a state field later is a breaking change with no #[non_exhaustive] guard.

**Why it matters**: Mirrors TASK-0884 / TASK-0858 for sibling crates; the extension crates top-level structs are part of the public registration surface and need the same compatibility guard the data-model structs already carry.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add #[non_exhaustive] to the four extension structs
- [ ] #2 Document construction via the existing factory only
<!-- AC:END -->
