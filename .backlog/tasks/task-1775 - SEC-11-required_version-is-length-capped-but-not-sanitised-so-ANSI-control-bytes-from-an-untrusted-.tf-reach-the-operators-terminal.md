---
id: TASK-1775
title: >-
  SEC-11: required_version is length-capped but not sanitised, so ANSI/control
  bytes from an untrusted .tf reach the operator's terminal
status: Triage
assignee: []
created_date: '2026-08-27 11:22'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:243-256` (`extract_required_version`), cap constant at `:152-157`

**What**: The SEC-11 / TASK-0853 hardening on this value validates exactly one property — length (`REQUIRED_VERSION_MAX_LEN = 64`) — and its own doc comment states the threat model: "An adversarial `.tf` could otherwise embed a long string that ends up rendered into the About card". The characters themselves are never validated. Whatever sits between the two quotes is returned verbatim, wrapped as `format!("Terraform {v}")` at `:73`, and lands in `ProjectIdentity::stack_detail`, which `compose_stack_value` (`crates/core/src/project_identity/format.rs:175-187`) joins straight into the rendered card.

So `required_version = "1.0\u{1b}[2J\u{1b}[31mCOMPROMISED"` (or any string with `\u{1b}`, `\r`, `\u{7}`, bidi overrides) is emitted to the terminal with the escapes intact — colour/cursor control, screen clears, and text the operator did not author, all attributed to the About card. The 64-char cap does not help: a working ANSI injection fits in well under 64 bytes.

**Why it matters**: `ops about` is run inside repositories the operator has cloned but not audited — that is the premise behind `MAX_MANIFEST_BYTES` and the cap this rule already implements. Terminal-escape injection from repository content is the classic version of this bug (the same class the codebase already guards against in `format_error_tail`'s CR normalisation at `crates/core/src/output.rs:95-98` and in the `assert_debug_escapes_control_chars` policy in `ops_about::test_support`), and here it reaches an operator terminal with no escaping layer anywhere between the `.tf` file and stdout.

**Cross-crate note**: no sanitiser exists on the shared render path, so sibling stacks feed unsanitised manifest strings into the same fields; the durable fix is a shared helper (e.g. in `ops_about::text_util`, next to `trim_nonempty`) that strips or escapes control characters, adopted here and by the siblings. This task is scoped to this crate's producing site — `extract_required_version` must not return control characters — and can be satisfied crate-locally if the shared helper lands later.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extract_required_version rejects or escapes control characters (including ESC, CR, BEL) so no raw control byte can reach stack_detail
- [ ] #2 The cap and the sanitisation are both covered by tests, including an ESC-bearing value under the 64-char cap
- [ ] #3 Preferred implementation reuses or introduces a shared ops_about helper so sibling stack providers can adopt the same policy
<!-- AC:END -->
