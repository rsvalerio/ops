---
id: TASK-0680
title: >-
  ERR-1: parse_active_toolchain accepts pathological rustup output as toolchain
  identifier
status: Done
assignee:
  - TASK-0737
created_date: '2026-04-30 05:15'
updated_date: '2026-04-30 17:59'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe.rs:36-39`

**What**: Returns the first non-empty line's first whitespace token; rustup ≥1.28 prints multiple sections (e.g. when there is no active toolchain, it prints an explanatory paragraph). A pathological "no active toolchain configured" message could be parsed as a toolchain like `error:` and used as `--toolchain error:` in `install_rustup_component_with_timeout`. The validate_cargo_tool_arg for the toolchain side rescues this (rejects :), but the parse contract is fragile.

**Why it matters**: Parsing contract should reject obvious non-toolchain shapes rather than relying on downstream validators.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Reject lines beginning with error: / info: / containing : outside the standard <channel>-<triple> shape
- [ ] #2 Add a regression test for the rustup ≥1.28 'no active toolchain' output
<!-- AC:END -->
