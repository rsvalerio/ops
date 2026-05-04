---
id: TASK-0950
title: >-
  READ-9: core::style ANSI helpers emit escape codes unconditionally (no
  TTY/NO_COLOR gate)
status: Triage
assignee: []
created_date: '2026-05-04 21:45'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/style.rs:3-19`

**What**: The `ansi_style!` macro defines `cyan`, `dim`, `green`, `red`, `yellow`, `bold`, etc. that always wrap input in `"\x1b[{code}m{}\x1b[0m"` regardless of `NO_COLOR` or whether output goes to a TTY. The sibling `theme::apply_style` correctly gates on TTY + `NO_COLOR`, so two color subsystems behave inconsistently.

**Why it matters**: Callers in `cli/src/tools_cmd.rs`, `theme_cmd.rs`, `extension_cmd.rs` write into a `&mut dyn Write` that may be a redirected file or pipe; unconditional escape codes violate the `NO_COLOR` convention and pollute non-terminal sinks (CI logs, captured output, piped grep).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 core::style helpers consult an is_terminal() && !NO_COLOR predicate before emitting SGR codes (mirror theme::style::sgr::color_enabled)
- [ ] #2 When stdout/stderr is not a terminal or NO_COLOR is set, helpers like cyan("x") return "x" verbatim with no escapes
- [ ] #3 Existing call sites in tools_cmd.rs etc. emit plain text in non-TTY tests
<!-- AC:END -->
