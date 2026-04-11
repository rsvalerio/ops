---
id: TASK-006
title: No clippy.toml — missing custom lint configuration
status: To Do
assignee: []
created_date: '2026-04-07 00:00:00'
updated_date: '2026-04-07 22:48'
labels:
  - rust-code-quality
  - CQ
  - LINT-6
  - LINT-4
  - LINT-5
  - low
  - effort-S
dependencies: []
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: workspace root (missing file)
**Anchor**: `clippy.toml`
**Impact**: No `clippy.toml` exists at the workspace root. Default clippy thresholds apply, with no project-specific tuning. LINT-6 recommends configuring project-wide settings (complexity thresholds, MSRV, allowed identifiers). LINT-4 recommends cherry-picking pedantic lints; LINT-5 recommends cherry-picking restriction lints for production code.

**Notes**:
Recommended starter `clippy.toml`:
```toml
cognitive-complexity-threshold = 25
too-many-arguments-threshold = 5
type-complexity-threshold = 250
```

Additionally, consider adding cherry-picked lints in `Cargo.toml` workspace `[lints.clippy]` or a shared `clippy.toml`:
- From pedantic: `cloned_instead_of_copied`, `manual_string_new`, `needless_pass_by_value`
- From restriction: `unwrap_used` (for library crates), `print_stderr` (enforce `tracing` usage)

The project already enforces `clippy --all-targets -- -D warnings` in CI, which is the recommended LINT-2 baseline. This task adds project-specific tuning on top.
<!-- SECTION:DESCRIPTION:END -->
