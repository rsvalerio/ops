---
id: TASK-1244
title: >-
  ERR-1: read_origin_url drops entire .git/config on any non-UTF-8 byte without
  typed diagnostic
status: To Do
assignee:
  - TASK-1267
created_date: '2026-05-08 13:00'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:107-116`

**What**: `(&mut file).take(limit).read_to_string(&mut content)` requires the entire file to be valid UTF-8. A single non-UTF-8 byte anywhere in .git/config (BOM, latin-1 commit-template, hostile injection in an unrelated section) makes the whole-file decode fail with InvalidData, the function logs a generic IO warn, and remote detection silently zeroes out — even when the [remote "origin"] block is well-formed UTF-8.

**Why it matters**: Operators chasing "remote_url is None" get a generic IO warn rather than a typed "non-UTF-8 config" signal, and the line scanner never gets a chance to extract the well-formed url= line.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Decode line-by-line (lossy or per-line strict) so a single bad byte does not poison remote detection
- [ ] #2 Emit a dedicated tracing::debug! when the file is non-UTF-8 vs other IO errors
- [ ] #3 Regression test: a valid url= line survives a non-UTF-8 byte in another section
<!-- AC:END -->
