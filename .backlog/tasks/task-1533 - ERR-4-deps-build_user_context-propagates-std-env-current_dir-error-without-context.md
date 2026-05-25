---
id: TASK-1533
title: >-
  ERR-4: deps build_user_context propagates std::env::current_dir error without
  context
status: Done
assignee:
  - TASK-1648
created_date: '2026-05-19 07:33'
updated_date: '2026-05-25 19:07'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:177`

**What**: `std::env::current_dir()?` bubbles up a raw `io::Error` with no indication this came from deps context construction.

**Why it matters**: When the failure surfaces in CLI output it looks like a generic "current dir" I/O error with no breadcrumb to `ops deps`, hampering triage.

Note: TASK-1493 already covers an identical pattern in another build_user_context site; verify on triage whether this is the same call site before closing as duplicate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Wrap with .with_context(|| "deps: failed to determine current working directory")
- [ ] #2 Caller-facing message identifies the deps path
- [ ] #3 No behavioural change on the happy path
<!-- AC:END -->
