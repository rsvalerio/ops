---
id: TASK-0915
title: >-
  SEC-33: terraform plan read_json_file reads user-supplied path with no size
  cap
status: Done
assignee: []
created_date: '2026-05-02 10:11'
updated_date: '2026-05-02 14:55'
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
- [x] #1 read_json_file enforces a configurable size cap (env-overridable) and bails with a clear error when exceeded
- [x] #2 Test covers oversized file rejection
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
read_json_file now uses File::open + Read::take to bound the read at DEFAULT_PLAN_JSON_MAX_BYTES (256 MiB by default), with operator override via OPS_PLAN_JSON_MAX_BYTES. Oversized payloads bail with a clear error naming the cap and the env override. Added read_json_file_rejects_oversized_payload test that lowers the cap to 64 bytes and asserts the error message.
<!-- SECTION:NOTES:END -->
