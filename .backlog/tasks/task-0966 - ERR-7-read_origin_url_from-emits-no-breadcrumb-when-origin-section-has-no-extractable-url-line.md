---
id: TASK-0966
title: >-
  ERR-7: read_origin_url_from emits no breadcrumb when origin section has no
  extractable url= line
status: Triage
assignee: []
created_date: '2026-05-04 21:47'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:140` (read_origin_url_from)

**What**: When `[remote "origin"]` is matched but every `url = ...` line fails `strip_url_key` (malformed key, empty value after inline-comment strip in TASK-0726), the function returns None — indistinguishable from "no origin remote".

**Why it matters**: Operators chasing "branch shows but remote_url is None" get no signal pointing at the corrupted config. A single tracing::debug breadcrumb (matching read_origin_url's NotFound vs real-error split) makes the difference visible without changing the silent-on-missing-remote happy path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Origin section present but no valid url key emits one tracing::debug naming the section
- [ ] #2 Genuinely-missing remote stays silent (no log)
- [ ] #3 Test pins both behaviours
<!-- AC:END -->
