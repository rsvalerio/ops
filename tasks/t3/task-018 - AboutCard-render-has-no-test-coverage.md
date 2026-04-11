---
id: TASK-018
title: 'AboutCard::render has no test coverage'
status: To Do
assignee: []
created_date: '2026-04-07 00:00:00'
updated_date: '2026-04-07 22:48'
labels:
  - rust-test-quality
  - TQ
  - TEST-5
  - TEST-6
  - medium
  - effort-S
  - crate-core
dependencies:
  - TASK-007
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/core/src/project_identity.rs:118-152`
**Anchor**: `fn render`
**Impact**: `AboutCard::render` is a public API with multiple branches (TTY styling, optional description, optional fields list with padding) but has zero test coverage. `from_identity` is well-tested, but the output-facing `render` method is not verified at all. The only indirect coverage is `cli_about_shows_header` in integration tests, which is feature-gated (`#[cfg_attr(not(feature = "stack-rust"), ignore)]`) and only asserts `stdout.contains("ops")` — it doesn't verify layout, field rendering, or the unstyled path.

**Notes**:
Blocked by TASK-007: `render` currently hardcodes `io::stdout().is_terminal()`, making it untestable without controlling terminal state. Once TASK-007 lands (accept `is_tty: bool` parameter), add tests for:
- Full card with all optional fields populated (description, multiple fields, repository)
- Minimal card (no description, no optional fields)
- TTY vs non-TTY output paths (styled vs plain)
- Field alignment (max_key_len padding logic)
- Empty fields list (no field section emitted)
<!-- SECTION:DESCRIPTION:END -->
