---
id: TASK-0027
title: 'CQ-3: deps/lib.rs is thick — parsing and formatting in one file'
status: Done
assignee: []
created_date: '2026-04-14 19:14'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-quality
  - ARCH-8+FN-1
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions-rust/deps/src/lib.rs (726 lines, ~420 production) — All logic lives in lib.rs: tool detection, cargo invocation, output parsing, diagnostic categorization, and report formatting (5 formatters + color utilities). parse_deny_output is 88 lines with 4 nesting levels, mixing JSON parsing with categorization logic. Formatting functions (lines 373-580) are presentation layer mixed with domain logic. Violates ARCH-8 (thin lib.rs) and FN-1 on parse_deny_output. Affected crate: ops-deps.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract formatting to format.rs and parsing to parse.rs. parse_deny_output should be ≤50 lines with categorization logic in a named helper.
<!-- AC:END -->
