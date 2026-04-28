---
id: TASK-0436
title: 'ERR-1: parse_deny_output silently drops diagnostics with unknown codes'
status: To Do
assignee:
  - TASK-0533
created_date: '2026-04-28 04:43'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:343-399`

**What**: After successfully parsing a DenyLine and DiagnosticFields, the function checks the code against four hard-coded sets (ADVISORY_CODES, LICENSE_CODES, BAN_CODES, SOURCE_CODES). A diagnostic whose code does not appear in any set is silently discarded — there is no else branch and no tracing::debug!/warn!. cargo-deny is on a stable but evolving schema (e.g. unmaintained, notice, workspace-duplicate were added over time). The equivalent diagnostic for malformed JSON (lines 318, 336) is logged via tracing::debug!, so the asymmetry is local: schema additions are exactly the case operators need to learn about.

**Why it matters**: A future cargo-deny release that introduces a new diagnostic code (e.g. a new ban category) would silently disappear from DenyResult, masking real findings. Sibling TASK-0317 closed the parse-error variant of the same hazard.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Log unknown codes at tracing::debug! with code, severity, and a truncated message so operators can detect cargo-deny schema drift
- [ ] #2 Include a unit test that asserts the trace fires for a diagnostic whose code is not in any of the four sets, while the DenyResult remains unchanged
<!-- AC:END -->
