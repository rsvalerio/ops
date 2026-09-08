---
id: TASK-2206
title: >-
  SEC-33: the plan-JSON size cap reports an opaque UTF-8 error instead of the
  documented cap message when the cut point splits a multibyte character
status: To Do
assignee:
  - TASK-2235
created_date: '2026-09-08 07:20'
updated_date: '2026-09-08 10:54'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:298-291` (`read_capped`)

**What**: `read_capped` does `reader.take(cap + 1).read_to_string(&mut buf)` and only *afterwards* compares `buf.len()` against the cap. `Read::read_to_string` validates UTF-8 over the whole truncated window and returns `ErrorKind::InvalidData` ("stream did not contain valid UTF-8") whenever byte `cap + 1` lands in the middle of a multi-byte sequence. In that case the function never reaches the `bail!` that names the cap, and the caller sees:

    failed to read plan JSON from `terraform show -json`: stream did not contain valid UTF-8

instead of the documented

    plan JSON from `terraform show -json` exceeds N bytes (override via OPS_PLAN_JSON_MAX_BYTES)

Terraform plan documents routinely contain non-ASCII (resource descriptions, tags, provider diagnostics embedded in `after` values), so the boundary is not exotic — it is a ~3-in-4 chance for any given oversized plan whose cut point falls inside a multi-byte run.

**Why it matters**: `OPS_PLAN_JSON_MAX_BYTES` is the only escape hatch for an operator whose stack legitimately exceeds the 256 MiB default. When the cap fires through the UTF-8 path the error names neither the cap nor the override env var, so the operator has no way to discover the control that would unblock them and will reasonably conclude that terraform emitted corrupt output. The three existing cap tests (`read_json_file_rejects_oversized_payload`, `read_stdin_rejects_oversized_payload`, `read_capped_rejects_oversized_terraform_show_output`) all use pure-ASCII `b'x'` payloads and therefore cannot see this.

Reading into a `Vec<u8>` and doing the cap comparison before `String::from_utf8` fixes both the message and the ordering: over-cap is a cap error regardless of encoding, and a genuinely non-UTF-8 payload under the cap still gets its own distinct error.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 read_capped decides over-cap from the byte count before any UTF-8 validation, so an oversized payload always produces the 'exceeds N bytes (override via OPS_PLAN_JSON_MAX_BYTES)' error
- [ ] #2 A payload that is under the cap but not valid UTF-8 still fails with a distinct, self-describing error naming the source
- [ ] #3 A regression test feeds an oversized payload whose byte at cap+1 splits a multi-byte UTF-8 sequence and asserts the cap message, not a UTF-8 message
<!-- AC:END -->
