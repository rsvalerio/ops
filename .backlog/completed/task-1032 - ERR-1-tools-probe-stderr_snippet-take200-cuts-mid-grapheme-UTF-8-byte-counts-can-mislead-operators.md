---
id: TASK-1032
title: >-
  ERR-1: tools::probe stderr_snippet take(200) cuts mid-grapheme/UTF-8 byte
  counts can mislead operators
status: Done
assignee: []
created_date: '2026-05-07 20:24'
updated_date: '2026-05-07 23:15'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe.rs:118-119` and `267-268`

**What**: Both `check_cargo_tool_installed` and `check_rustup_component_installed` truncate stderr for the warn-log via `stderr_tail.chars().take(200).collect::<String>()`. `chars()` is char-based not byte-based, so the cap is in *Unicode scalar values*, not in display columns or bytes. This is fine for ASCII cargo output, but rustup component-list output and localised cargo error messages can include CJK or RTL characters where 200 scalar values can be 600+ bytes — exceeding the implicit "small breadcrumb" intent and bloating logs in non-en_US locales.

More importantly, the truncation can land mid-grapheme cluster (e.g. an emoji + variation selector pair), producing a malformed string fragment in logs.

**Why it matters**: Low-severity log hygiene. The unit test at line 426-430 only verifies ASCII-byte length so it can't catch this regression.

**Suggested fix**: use `format_error_tail` (the same helper sibling crates already use, e.g. cargo-update lib.rs) which is byte-bounded and handles char boundaries. Already pulled in across the workspace per TASK-0559.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 stderr truncation in check_cargo_tool_installed and check_rustup_component_installed routes through a byte-bounded helper that respects char boundaries
- [ ] #2 Test added with a non-ASCII stderr fixture asserts the resulting snippet is well-formed UTF-8 and bounded by display width or a fixed byte cap
<!-- AC:END -->
