---
id: TASK-1211
title: >-
  ERR-1: build_identity_value error message renders non-UTF-8 cwd via Display,
  smuggling U+FFFD
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
**File**: `extensions/about/src/identity.rs:120-125`

**What**: build_identity_value rejects a non-UTF-8 cwd with `format!("project_root path is not valid UTF-8: {}", cwd.display())`. Display on Path::display substitutes invalid bytes with U+FFFD. The fix was added under TASK-1103 to keep U+FFFD out of the JSON payload, but the error message itself still embeds U+FFFD-mangled bytes that the operator sees in logs / CLI output.

**Why it matters**: Same root cause as the broader ERR-7 sweep: Path::display is not safe for any operator-facing rendering when the underlying bytes may not be UTF-8 or may contain control characters.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The error message uses format!('{:?}', cwd) which prints the raw OsStr quoted with escaped non-printables, OR reports byte length and a hex preview of the leading invalid sequence.
- [ ] #2 A unit test extends build_identity_value_rejects_non_utf8_cwd to capture the DataProviderError::ComputationFailed(msg) payload and assert the rendered message contains no U+FFFD and instead encodes the invalid bytes faithfully.
<!-- AC:END -->
