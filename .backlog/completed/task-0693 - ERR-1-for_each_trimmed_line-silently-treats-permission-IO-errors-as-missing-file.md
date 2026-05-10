---
id: TASK-0693
title: >-
  ERR-1: for_each_trimmed_line silently treats permission/IO errors as missing
  file
status: Done
assignee:
  - TASK-0737
created_date: '2026-04-30 05:26'
updated_date: '2026-04-30 18:01'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/text.rs:50-56`

**What**: `for_each_trimmed_line` calls `std::fs::read_to_string(path).ok()?` which collapses every IO failure (NotFound, PermissionDenied, IO error mid-read, non-UTF-8) into the same `None` return. Callers (Go/Gradle/Python manifest parsers via TASK-0321) interpret `None` as "manifest absent", so a `chmod 000 go.mod` or a transient FS error silently produces an empty/fallback ProjectIdentity instead of a diagnosable failure.

**Why it matters**: Production users debugging "why does ops about show no version" have no signal that a sibling stack manifest was unreadable. Sibling parsers in extensions (e.g. parse_pom_xml per TASK-0561) have already been migrated to distinguish NotFound from other errors; this central helper was missed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Distinguish NotFound (return None / Some with no callbacks) from other read_to_string errors
- [ ] #2 Surface non-NotFound errors via tracing::warn or a Result return
- [ ] #3 Update existing call sites to keep their current behavior on NotFound
<!-- AC:END -->
