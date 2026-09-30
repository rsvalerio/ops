---
id: TASK-2346
title: 'ops deps --check fails when cargo-deny emits license-not-encountered warnings'
status: Triage
assignee: []
created_date: '2026-09-30 20:45'
labels:
  - deps
  - bug
dependencies: []
modified_files:
  - crates/deps/src/parse/deny.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/deps/src/parse/deny.rs` (the TASK-1840 guard: undecodable diagnostics fail the run)

**What**: with cargo-deny 0.20.2, a deny.toml whose `[licenses] allow` has entries no dependency uses makes cargo-deny emit one `license-not-encountered` warning per unused entry, e.g. `{"fields":{"code":"license-not-encountered","graphs":[],"labels":[{"column":6,"line":24,"message":"unmatched license allowance","span":"0BSD"}],"message":"license was not encountered","severity":"warning",...}}`. ops 0.77.0 decodes none of them ("cargo deny emitted 12 diagnostic line(s) but only 0 could be decoded and classified (12 dropped)") and fails `ops deps --check`. The fail-closed guard works as designed, but the diagnostic has no `graphs` and is not about a crate, and the classifier has no case for it.

**Repro**: forge-testbed at b1b401d plus a mise.toml pinning ops 0.77.0 and cargo-deny 0.20.2, then `ops deps --check`. Its deny.toml is a shared baseline with 14 allowed licenses.

**Workaround in use**: `unused-allowed-license = "allow"` under `[licenses]` (forge-testbed PR #6).

**Why it matters**: forge rust-ci runs `ops deps --check` in every caller, and a shared baseline allow-list is the normal case, so every such repo fails until it adds the workaround.

**Origin**: forge-testbed migration to forge rust-ci on mise, 2026-09-30.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A cargo-deny license-not-encountered warning (cargo-deny 0.20.2) is decoded and classified as a warning, not dropped
- [ ] #2 ops deps --check passes on a repo whose deny.toml allow-list has unused entries (unless configured to deny them)
- [ ] #3 A test covers the license-not-encountered diagnostic shape
<!-- AC:END -->
