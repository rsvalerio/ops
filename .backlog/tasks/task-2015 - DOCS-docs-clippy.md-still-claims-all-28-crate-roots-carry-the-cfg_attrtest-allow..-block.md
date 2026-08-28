---
id: TASK-2015
title: >-
  DOCS: docs/clippy.md still claims all 28 crate roots carry the cfg_attr(test,
  allow(..)) block
status: Triage
assignee: []
created_date: '2026-08-28 15:56'
labels:
  - code-review-rust
  - documentation
dependencies: []
modified_files:
  - docs/clippy.md
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `docs/clippy.md:145-155`

**What**: Layer 2 of the lint policy is documented as a four-lint
`#![cfg_attr(test, allow(clippy::unwrap_used, clippy::cast_possible_truncation,
clippy::cast_precision_loss, clippy::cast_sign_loss))]` block, followed by the
claim "All 28 crate roots carry this block". That is no longer true and is
getting less true with each READ-10 task that lands: `extensions/tokei` now
carries **no** crate-root relaxation at all, because `clippy.toml` already sets
`allow-unwrap-in-tests = true` (writing the surviving `unwrap_used` entry as
`expect` reports it as unfulfilled) and the crate has no `as` cast for the three
cast lints to suppress. TASK-1914 already trimmed another crate, and
TASK-1761 / TASK-1801 / TASK-1828 / TASK-1883 / TASK-1917 / TASK-1935 /
TASK-1946 / TASK-1966 will trim the rest.

**Why it matters**: the page is the single source of truth for lint policy, and
its layer-2 section currently instructs the next author to paste a four-lint
block that suppresses nothing in most crates -- which is exactly the defect the
READ-10 findings were filed against. The census line should state the real
policy: the block is per-crate and only lists lints with an actual callsite;
`unwrap_used` in tests is already covered workspace-wide by `clippy.toml`.

**Origin**: discovered during TASK-2012 while fixing TASK-1968.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/clippy.md layer 2 no longer claims every crate root carries the four-lint block, and states that a crate root only relaxes lints it actually triggers
- [ ] #2 The interaction with clippy.toml's allow-unwrap-in-tests is documented, so a crate root that needs no block at all is a recognised outcome
<!-- AC:END -->
