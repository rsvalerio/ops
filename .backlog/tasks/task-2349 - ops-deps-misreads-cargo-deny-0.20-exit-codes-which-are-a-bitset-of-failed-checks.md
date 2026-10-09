---
id: TASK-2349
title: 'ops deps misreads cargo-deny 0.20 exit codes, which are a bitset of failed checks'
status: Done
assignee: []
created_date: '2026-10-02 20:45'
updated_date: '2026-10-04 15:27'
labels:
  - code-review-rust
  - deps
  - bug
dependencies: []
parent_task_id: 'TASK-2417'
modified_files:
  - extensions-rust/deps/src/parse/deny.rs
  - extensions-rust/deps/src/parse/deny/tests.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/deny.rs` (`interpret_deny_result`)

**What**: `interpret_deny_result` assumes cargo-deny exits 0 (clean), 1 (findings) or 2 (config error). cargo-deny 0.20.2 exits with a bitset of the checks that failed: a deny.toml with `unused-allowed-license = "deny"` and unused allowances makes `cargo deny check` exit 4 (licenses), which ops reports as "cargo deny exited with unexpected status code 4; refusing to treat partial diagnostics as authoritative" instead of decoding and rendering the license findings. A bans-only failure would presumably exit 2 and be misread as a configuration error.

**Why it matters**: the gate still fails closed, but with a misleading error and no rendered findings, so an operator sees "unexpected status" rather than which licenses or crates failed. Confirm the bitset values against cargo-deny's source before changing the mapping.

**Origin**: discovered during TASK-2348 while fixing TASK-2346 (end-to-end check with `unused-allowed-license = "deny"`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 interpret_deny_result decodes and renders findings for every cargo-deny 0.20 failure exit code (bitset of failed checks)
- [x] #2 A real configuration error is still told apart from a check failure
- [x] #3 Tests cover the licenses-only and bans-only exit codes

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed in wave TASK-2417. Exit codes probed against cargo-deny 0.20.2 in a scratch crate: bans-only exits 2, licenses-only 4, bans+licenses 6; a broken deny.toml exits 1 after a JSON log envelope at ERROR level; a clap usage error exits 2 with plain-text stderr. interpret_deny_result now treats any non-empty subset of the four check bits (1..=15) as a check failure and decodes its findings; a failing run with no classifiable finding reports a configuration error when an ERROR log envelope is present, otherwise the zero-diagnostics error. Not probed directly: the advisories (1) and sources (8) bit values, taken from the bitset layout.
<!-- SECTION:NOTES:END -->
