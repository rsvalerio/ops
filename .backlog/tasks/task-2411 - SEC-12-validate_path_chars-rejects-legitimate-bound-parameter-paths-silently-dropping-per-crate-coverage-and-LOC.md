---
id: TASK-2411
title: 'SEC-12: validate_path_chars rejects legitimate bound-parameter paths, silently dropping per-crate coverage and LOC'
status: Triage
assignee: []
created_date: '2026-10-04 14:18'
labels:
  - code-review-rust
  - SEC
dependencies: []
modified_files:
  - extensions/sqlite/src/sql/query/helpers.rs
  - extensions/sqlite/src/sql/query/coverage.rs
  - extensions/sqlite/src/sql/validation.rs
priority: medium
ordinal: 1000
dedup_key: 'SEC-12:extensions/sqlite/src/sql/query/helpers.rs:prepare_per_crate'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/sql/query/helpers.rs:~215` (`prepare_per_crate`), `extensions/sqlite/src/sql/query/coverage.rs:~65` (`query_crate_coverage`), `extensions/sqlite/src/sql/validation.rs:~190` (`validate_path_chars`)

**What**: `member_paths` and `workspace_root` are only ever passed as bound parameters (the code comments say so), yet `validate_path_chars` accepts only ASCII alphanumerics, `- _ / .` and space. A workspace root such as `/home/u/My Project (old)`, `/home/u/proj+x`, `/home/u/@scope/ws` or any non-ASCII directory makes `query_crate_coverage` return `Err`, and every per-crate query fails when a member path contains such a character. Callers degrade through `query_or_warn`, so the user sees missing data and a warn log, with no sign of the real cause.

**Why it matters**: A defense-in-depth check that cannot prevent injection (the values are bound) turns valid input into a silent feature loss. SEC-12 requires validating identifiers that are interpolated, not bound values. The traversal and control-character checks can stay.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Per-crate queries and query_crate_coverage succeed for paths containing parentheses, plus, at-sign and non-ASCII characters; control characters are still rejected
- [ ] #2 A regression test covers a workspace root and member path with such characters
<!-- AC:END -->
