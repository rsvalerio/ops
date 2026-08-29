---
id: TASK-2053
title: >-
  ARCH-11: ops verify lints only --all-features, so feature-off code can go red
  without failing the gate
status: To Do
assignee:
  - TASK-2062
created_date: '2026-08-29 13:05'
updated_date: '2026-08-29 17:27'
labels:
  - code-review-rust
  - architecture
dependencies: []
modified_files:
  - docs/clippy.md
  - .ops.toml
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `docs/clippy.md:350-353`

**What**: the workspace lint gate is
`cargo clippy --workspace --all-features --all-targets -- -D warnings`
(`ops verify`). With every feature on, `#[cfg(not(feature = ...))]` code is never
compiled, so lints that fire only in the feature-off build are invisible to the
gate. `docs/clippy.md` already warns that a plain `cargo clippy` misses
feature-gated code; the reverse gap — `--all-features` missing the *feature-off*
arms — is undocumented and unenforced.

TASK-2027 is the worked example: two `clippy::missing_const_for_fn` errors sat on
the `duckdb`-off `enrich_from_db` stubs in `ops-about` and only surfaced when
someone ran `cargo clippy -p ops-about`. A default-feature sweep over the
workspace is currently clean, so this is about keeping it that way rather than an
outstanding red build.

**Why it matters**: `-D clippy::nursery` is workspace policy, so any developer or
CI step that lints a subset of crates gets a red build from code they did not
touch, and the failure is attributed to whoever happened to run the narrower
command. Every crate with a `cfg(not(feature = ..))` arm can regress the same way.

**Origin**: discovered during TASK-2049 while fixing TASK-2027.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The lint gate (or a documented companion step) covers the default-feature build as well as --all-features, so a feature-off clippy error fails CI rather than only a per-crate run
- [ ] #2 docs/clippy.md records the --all-features blind spot alongside the existing plain-cargo-clippy warning
<!-- AC:END -->
