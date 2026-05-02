---
id: TASK-0845
title: >-
  ERR-2: parse_deny_output defaults missing severity to 'error', conflicting
  with fail-closed has_issues
status: Triage
assignee: []
created_date: '2026-05-02 09:15'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:426` and `extensions-rust/deps/src/lib.rs:230-265`

**What**: decode_diagnostic substitutes "error" when severity is absent. has_issues fails closed on unknown severities. The two policies are coherent only if cargo-deny always emits a severity. If a future cargo-deny variant emits diagnostics without severity, every such entry is now unconditionally rated as actionable error - including notes/help - silently inverting the intent of the new fail-closed semantics.

**Why it matters**: The defaults conflict. The comment chain (TASK-0601) installs fail-closed semantics for unknown values, but the absent-severity branch chooses a fail-loud-as-error sentinel rather than None. A schema drift scenario therefore promotes informational diagnostics to gate failures with no visibility.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Missing severity is preserved as a distinct sentinel (e.g. severity: <missing>) and routed through the same warn-and-fail-closed path as unknown severities
- [ ] #2 Or the default is documented and validated as the intended behaviour with an integration test
- [ ] #3 tracing::warn! fires once per missing-severity diagnostic so drift is observable
<!-- AC:END -->
