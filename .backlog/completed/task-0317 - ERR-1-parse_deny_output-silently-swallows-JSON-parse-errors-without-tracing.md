---
id: TASK-0317
title: 'ERR-1: parse_deny_output silently swallows JSON parse errors without tracing'
status: Done
assignee:
  - TASK-0326
created_date: '2026-04-24 08:53'
updated_date: '2026-04-25 13:17'
labels:
  - rust-code-review
  - errors
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions-rust/deps/src/parse.rs:199-211

**What**: Two match arms drop malformed JSON lines with Err(_) => continue and no logging.

**Why it matters**: Masks upstream cargo-deny schema drift; operators see silent coverage loss.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Emit tracing::debug! with truncated line and error before continue
- [ ] #2 Test verifies tracing output includes error context
<!-- AC:END -->
