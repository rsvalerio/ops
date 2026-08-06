---
id: TASK-1658
title: >-
  components.md sample output shows 'cargo test --all-targets', which disables
  doctests
status: Triage
assignee: []
created_date: '2026-08-06 14:30'
labels:
  - docs
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `docs/components.md:34`

**What**: the pending-step visual example renders

```text
  ○  cargo test --all-targets .........................................
```

For `cargo test`, `--all-targets` *disables* doctests — cargo's own help reads "Test all targets (does not include doctests)". The docs therefore model, as a representative ops invocation, the exact anti-pattern the Rust defaults now carry a regression test against (`rust_test_commands_omit_all_targets_to_keep_doctests`, PR #5).

**Why it matters**: low severity — this is illustrative rendering output, chosen to show dot padding, not a command ops runs. But it is the sample a reader copies when writing their own `.ops.toml`, and silently losing doctest coverage is hard to notice: the suite still passes, just with fewer tests.

**Fix**: change the third line to something that does not misinform, e.g. `cargo test --workspace --all-features`, keeping a similar width so the padding illustration still reads correctly. Check the neighbouring lines (`cargo build --all-targets`, `cargo clippy --all-targets -- -D warnings`) too — those two are fine, since `--all-targets` is correct for build and clippy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No docs sample shows --all-targets applied to cargo test
- [ ] #2 The dot-padding illustration still demonstrates varying label widths
<!-- AC:END -->
