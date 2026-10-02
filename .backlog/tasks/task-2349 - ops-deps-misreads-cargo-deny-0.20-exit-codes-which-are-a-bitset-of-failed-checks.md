---
id: TASK-2349
title: 'ops deps misreads cargo-deny 0.20 exit codes, which are a bitset of failed checks'
status: Triage
assignee: []
created_date: '2026-10-02 20:45'
labels:
  - code-review-rust
  - deps
  - bug
dependencies: []
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
- [ ] #1 interpret_deny_result decodes and renders findings for every cargo-deny 0.20 failure exit code (bitset of failed checks)
- [ ] #2 A real configuration error is still told apart from a check failure
- [ ] #3 Tests cover the licenses-only and bans-only exit codes
<!-- AC:END -->
