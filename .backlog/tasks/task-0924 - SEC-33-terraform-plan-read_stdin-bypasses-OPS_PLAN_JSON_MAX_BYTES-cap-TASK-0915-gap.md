---
id: TASK-0924
title: >-
  SEC-33: terraform plan read_stdin bypasses OPS_PLAN_JSON_MAX_BYTES cap
  (TASK-0915 gap)
status: Done
assignee: []
created_date: '2026-05-02 15:10'
updated_date: '2026-05-02 17:26'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:117`

**What**: TASK-0915 byte-capped `read_json_file` via `plan_json_max_bytes()` + `take(limit)` so a symlink-to-`/dev/zero` or adversarially-large JSON cannot exhaust memory. The sibling path triggered by `--json-file=-` (`read_stdin`, lines 117-124) calls `io::stdin().read_to_string(&mut buf)` with no cap. Both branches feed the same `parse_and_classify` pipeline, so the cap should be uniform.

**Why it matters**: The threat model that motivated TASK-0915 (bounded memory for plan JSON ingestion) applies equally to stdin: a process upstream piping unbounded bytes (e.g. `cat /dev/zero | ops terraform plan --json-file=-`) will OOM the renderer. Closes the gap left after TASK-0915.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 read_stdin uses the same plan_json_max_bytes() cap via io::stdin().lock().take(limit).read_to_string(...) and bails with the same OPS_PLAN_JSON_MAX_BYTES override message when exceeded
- [x] #2 Add a unit test that wraps a Read source returning > cap bytes and asserts the bail (parallel to read_json_file cap test)
- [ ] #3 ops verify and ops qa pass with no new clippy warnings
<!-- AC:END -->
