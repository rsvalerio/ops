---
id: TASK-0915
title: >-
  SEC-33: terraform plan read_json_file reads user-supplied path with no size
  cap
status: Triage
assignee: []
created_date: '2026-05-02 10:11'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:110`

**What**: read_json_file calls std::fs::read_to_string on a user-supplied path without bounding size. A symlink-pointed FIFO or oversized JSON (terraform plans for huge stacks routinely exceed 100 MB) loads entirely into memory before parse_and_classify runs.

**Why it matters**: Same DoS posture as TASK-0831 SEC-33 for manifest readers; the terraform plan path is not gated by a size cap and is reachable with `ops plan --json-file <path>`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 read_json_file enforces a configurable size cap (env-overridable) and bails with a clear error when exceeded
- [ ] #2 Test covers oversized file rejection
<!-- AC:END -->
