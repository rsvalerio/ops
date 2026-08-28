---
id: TASK-1950
title: >-
  DUP-1: the capped-read block is duplicated verbatim between read_stdin_capped
  and read_json_file
status: To Do
assignee:
  - TASK-2002
created_date: '2026-08-27 15:49'
updated_date: '2026-08-28 14:15'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:211-229` and `:246-270`

**What**: Both functions carry the same nine-line sequence, differing only in the reader and the wording of the bail:

    let cap = plan_json_max_bytes();
    let limit = cap.saturating_add(1);
    let mut buf = String::new();
    reader.take(limit).read_to_string(&mut buf)...?;
    // identical five-line comment about usize widening, copied verbatim
    if u64::try_from(buf.len()).unwrap_or(u64::MAX) > cap {
        anyhow::bail!("... exceeds {cap} bytes (override via {PLAN_JSON_MAX_BYTES_ENV})");
    }

Even the explanatory comment ("`usize` is at most 64 bits on every supported target ...") is copy-pasted word for word at `:220-222` and `:261-263`.

**Why it matters**: DUP-1 flags 5+ line identical blocks. This one is a security control, so drift between the two copies is a security bug, not a style one - and a third copy is now needed for the `terraform show -json` branch (see the SEC-33 finding on `run_terraform_pipeline`), which is exactly the moment to extract rather than triplicate.

**Suggested fix**: one `fn read_capped<R: Read>(reader: &mut R, source: &str) -> anyhow::Result<String>` that both call, with `source` supplying the "on stdin" / "at {path}" fragment of the message.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A single generic capped-read helper is used by the stdin and file branches
- [ ] #2 The usize-widening comment exists in exactly one place
- [ ] #3 Existing read_json_file_rejects_oversized_payload and read_stdin_rejects_oversized_payload tests still pass with their current assertions on the message wording
<!-- AC:END -->
