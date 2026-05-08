---
id: TASK-1215
title: >-
  ERR-1: read_origin_url_from drops control-byte URL lines at debug level only,
  hiding stale last-wins value
status: To Do
assignee:
  - TASK-1267
created_date: '2026-05-08 08:19'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - err
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:172-188`

**What**: When RedactedUrl::redact rejects a `url = ...` line (SEC-2 / TASK-1102) the dropped value is logged at tracing::debug! only, with no field naming the rejected key or count. The control-byte rejection path bypasses the no-url breadcrumb because last may still be Some from a prior valid URL, leaving the operator with no signal that the most recent (last-wins) URL was silently rejected.

**Why it matters**: The whole rationale for last-wins (ERR-4 / TASK-0594) is that templated includes routinely rewrite url; a final url = ... line that gets dropped for control bytes is the exact case where last-wins lies to the operator.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The control-byte rejection branch logs at tracing::warn (not debug) with path Debug-formatted and a count of rejected lines so a malformed config that drops every value differs from one that drops only the latest.
- [ ] #2 A regression test feeds read_origin_url_from a config where the trailing url = ... line contains an embedded ANSI escape; asserts the function returns the previous valid URL AND that one warn-level event was emitted naming the rejected line count.
<!-- AC:END -->
