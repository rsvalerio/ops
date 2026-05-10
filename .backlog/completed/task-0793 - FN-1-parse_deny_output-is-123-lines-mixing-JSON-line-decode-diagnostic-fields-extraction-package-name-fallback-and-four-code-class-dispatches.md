---
id: TASK-0793
title: >-
  FN-1: parse_deny_output is 123 lines mixing JSON line decode,
  diagnostic-fields extraction, package-name fallback, and four code-class
  dispatches
status: Done
assignee:
  - TASK-0821
created_date: '2026-05-01 05:59'
updated_date: '2026-05-01 06:45'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:338-461`

**What**: Single function does (a) per-line JSON decode + tracing log on failure, (b) DiagnosticFields reshape + log on failure, (c) code lookup, (d) severity/message extraction, (e) package-name precedence with sentinel logging, (f) four cascading if ADVISORY_CODES.contains / LICENSE_CODES.contains / BAN_CODES.contains / SOURCE_CODES.contains blocks each with their own entry construction.

**Why it matters**: Operates at three abstraction levels in one function; adding a new code class or evolving severity policy means editing in five places. Size obscures that each branch builds slightly different entry shapes — which is where TASK-0597 / TASK-0436 found bugs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract per-line decode (DenyLine + DiagnosticFields) into a helper returning a typed Diagnostic struct
- [ ] #2 Extract package-name resolution into its own helper (already has its own tracing concern)
- [ ] #3 Replace the four-branch if-chain with a single dispatch table mapping code → entry-class enum so adding a new class is one line
- [ ] #4 Function ends up <=50 lines and each helper is at one abstraction level
<!-- AC:END -->
