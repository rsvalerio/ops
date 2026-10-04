---
id: TASK-2380
title: 'TEST-6: foundation error and drift branches (unparseable file, unparseable member, unresolvable members) have no tests'
status: To Do
assignee: []
created_date: '2026-10-04 14:13'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2419'
modified_files:
  - extensions-rust/foundation/src/tests.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-6:extensions-rust/foundation/src/tests.rs:error-branches'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/foundation/src/tests.rs`

**What**: No test in `tests.rs` or `compare.rs` covers these branches (no test string contains "does not parse" or malformed input):
- `check_file` when the repo file is not valid TOML (`lib.rs:~366`).
- `check_member_opt_in` and `scaffold_member_opt_in` with a malformed member manifest.
- `workspace_members` when `CargoToml::parse` fails (`lib.rs:~148`, returns empty and warns).
- `check_file` returning an error for a non-NotFound read failure, such as a directory at `deny.toml`.
- `scaffold_lints` and `check` for `Shape::Package` with `--force`.

**Why it matters**: Each of these is a behavior branch of a check that gates CI, and none has a regression guard. The unparseable-file and unparseable-member branches are the ones with the open findings above.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Tests cover a malformed foundation file, a malformed member manifest, and a root manifest whose members cannot be resolved, asserting the specific drift or error text
- [ ] #2 A test covers a non-NotFound read error and the Package-shape force replace
<!-- AC:END -->
