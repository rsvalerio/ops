---
id: TASK-2176
title: 'ERR-1: cargo-deny exit 0 skips the partial-decode-loss guard, so the dominant ops deps path has no schema-drift protection'
status: Done
assignee: []
created_date: '2026-09-08 07:12'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - error-handling
dependencies: []
parent_task_id: 'TASK-2234'
modified_files:
  - extensions-rust/deps/src/parse/deny.rs
priority: high
ordinal: 89000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/deny.rs:77`

**What**: `interpret_deny_result` applies the crate's drift guards only on the `Some(1)` arm:

```rust
Some(0) => Ok(parse_deny_output(stderr)),
Some(1) => { ...zero-diagnostics guard...; check_partial_decode_loss(&diag, stderr)?; Ok(parsed) }
```

`parse_deny_output` is documented as the unguarded wrapper (`parse_deny_output_inner(stderr).0`) — it discards `DenyParseDiagnostics` entirely. So the `candidate_diagnostics` vs `entries_emitted` drop-rate check built by TASK-1840 never runs on an exit-0 run.

Exit 0 is not the rare case. cargo-deny exits 0 whenever every finding is at `warning` level — which is the default for `[bans] multiple-versions` and for `unmaintained`/`yanked` when configured as `warn`. This repo's own `ops deps` output renders a `Duplicate Crates: N warnings` row, i.e. it routinely takes the exit-0 path with a non-empty diagnostic stream. On that path a per-code cargo-deny schema change that stops an entire diagnostic class from decoding produces silently missing rows and a green report, which is precisely the failure mode TASK-1840 was filed to close.

**Why it matters**: the guard protecting the supply-chain gate is absent from the code path the gate normally runs on. `check_partial_decode_loss` is a no-op when `candidate_diagnostics == 0`, so running it on the exit-0 arm is free on genuinely clean runs and costs nothing in behaviour except closing the hole.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 interpret_deny_result routes the Some(0) arm through parse_deny_output_inner and applies check_partial_decode_loss, the same guard the Some(1) arm applies
- [ ] #2 a genuinely clean exit-0 run (no diagnostic envelopes on stderr) still returns Ok with an empty DenyResult
- [ ] #3 a test drives exit 0 with a stderr carrying several type==diagnostic lines of which most have an unrecognised code, and asserts interpret_deny_result errs instead of returning the surviving subset
<!-- AC:END -->
