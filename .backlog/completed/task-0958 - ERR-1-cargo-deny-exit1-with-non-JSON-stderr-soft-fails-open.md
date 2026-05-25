---
id: TASK-0958
title: 'ERR-1: cargo deny exit==1 with non-JSON stderr soft-fails-open'
status: Done
assignee:
  - TASK-1013
created_date: '2026-05-04 21:46'
updated_date: '2026-05-07 19:11'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:342-358` (interpret_deny_result)

**What**: `interpret_deny_result` for exit 1 only treats whitespace-only stderr as failure. If cargo-deny emits text-mode error banners (e.g. "error[A001]: …") on stderr at exit 1 (forgotten user-format flag, or future cargo-deny defaults), every line falls through `decode_diagnostic`'s "malformed JSON" debug log and `parse_deny_output` returns an empty `DenyResult` — the supply-chain gate scores green.

**Why it matters**: Soft-fail-open hidden inside otherwise strict exit handling. The gate's safety property "fail closed on schema drift" leaks for the most likely drift mode (text-mode default).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 After parse_deny_output, if exit==1 and zero diagnostics decoded, surface a hard error citing 'cargo deny exit 1 with non-JSON stderr'
- [x] #2 Test covers a stderr blob with no JSON lines at exit 1 and asserts the gate fails closed
<!-- AC:END -->
