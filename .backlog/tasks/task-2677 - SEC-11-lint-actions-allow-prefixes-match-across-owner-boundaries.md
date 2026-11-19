---
id: TASK-2677
title: 'SEC-11: lint-actions allow prefixes match across owner boundaries'
status: To Do
assignee: []
created_date: '2026-11-18 18:26'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2843'
modified_files:
  - crates/cli/src/lint_actions_cmd.rs
priority: low
ordinal: 1000
dedup_key: 'SEC-11:crates/cli/src/lint_actions_cmd.rs:uses_problem'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/lint_actions_cmd.rs:205` (`uses_problem`)

**What**: An allow-list entry exempts any `uses:` value for which `value.starts_with(prefix)` holds. The `--allow` help suggests `owner/repo/`, but nothing enforces a boundary: an entry written as `my-org` (or `my-org/act`) also exempts `my-org-attacker/evil@main` (or `my-org/action-typosquat@main`), which then skips the SHA-pin check entirely.

**Why it matters**: `ops lint-actions` is a supply-chain gate. A natural-looking allow entry without the trailing `/` silently widens the exemption to look-alike owners and repos, so an unpinned third-party action passes the gate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An allow prefix matches only at a path boundary (prefix ends with '/', or the next char of the value is '/' or '@')
- [ ] #2 Or prefixes without a trailing '/' are rejected or normalised at load time with a clear message
- [ ] #3 A test shows allow 'my-org' does not exempt 'my-org-other/x@main'
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.02 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
