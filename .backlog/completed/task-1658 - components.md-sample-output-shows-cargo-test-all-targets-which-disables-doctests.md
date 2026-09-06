---
id: TASK-1658
title: >-
  components.md sample output shows 'cargo test --all-targets', which disables
  doctests
status: Done
assignee: []
created_date: '2026-08-06 14:30'
updated_date: '2026-08-15 00:00'
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
- [x] #1 No docs sample shows --all-targets applied to cargo test
- [x] #2 The dot-padding illustration still demonstrates varying label widths
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Shipped in PR #14 (squashed as `81e514d`), released in v0.36.1.

**Scope was larger than reported.** The description named only
`docs/components.md:34`, but AC #1 is about *any* docs sample, and a grep found
five occurrences in that file — lines 34, 156, 193, 242 and 248 (the Error
Detail Box, Summary Separator, and the two annotated composite examples). All
five fixed.

Replacements match what the Rust stack defaults actually run
(`crates/core/src/.default.rust.ops.toml`), so the samples now double as
accurate documentation rather than merely non-misleading filler:

- line 34 → `cargo test --workspace --all-features` (the real `test` command)
- lines 156, 193, 242, 248 → `cargo test --workspace` (shorter label; these
  lines carry timers and trailing `# [n]` annotations whose columns must hold)

Dot padding recomputed per line so every total width and annotation column is
unchanged: 71/63/62/69/60 before and after. AC #2 holds — the primary example
still shows three distinct label widths (25 / 41 / 37 chars).

Left untouched: `docs/command-mappings.md:26`, the prose *explaining* that
`--all-targets` is deliberately absent from `test` — the one place the string
should appear. `cargo build --all-targets` and `cargo clippy --all-targets`
samples are also unchanged; the flag is correct for those two.
<!-- SECTION:NOTES:END -->
