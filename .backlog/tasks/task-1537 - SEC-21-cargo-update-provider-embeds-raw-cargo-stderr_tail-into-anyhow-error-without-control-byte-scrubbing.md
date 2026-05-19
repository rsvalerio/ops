---
id: TASK-1537
title: >-
  SEC-21: cargo-update provider embeds raw cargo stderr_tail into anyhow error
  without control-byte scrubbing
status: To Do
assignee:
  - TASK-1575
created_date: '2026-05-19 09:54'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - sec
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:436-441`

**What**: On a non-zero `cargo update --dry-run` exit, `provide()` calls `format_error_tail(&output.stderr, 10)` and interpolates the result directly into `anyhow::anyhow!("cargo update --dry-run exited with status {}: {}", output.status, stderr_tail)`. `format_error_tail` (crates/core/src/output.rs:98) normalises CR/CRLF but does NOT scrub other C0 control bytes (ESC `\x1b`, BEL `\x07`, NUL `\x00`, etc.) — only the bare-CR contract is enforced and tested. The resulting `DataProviderError` carries those bytes verbatim through to whatever Display-formats it (tracing, the about page renderer).

**Why it matters**: Cargo's stderr is influenced by crate names, version strings, and registry metadata — surface area an attacker can shape via a poisoned crate. The same SEC-21 family has already been filed against sibling code paths (TASK-1160 for cargo-upgrade stderr tail, TASK-1250 for interpret_deny_result, TASK-1127 for log_and_redact_spawn_error). The fix lands in the same shape: scrub control bytes (or route via the existing redaction helper) before interpolating the tail into a user-visible error.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 stderr_tail is run through a control-byte scrubber (matching the SEC-21 fix used by sibling sites — e.g. cargo-upgrade / interpret_deny_result) before being interpolated into the anyhow!() message.
- [ ] #2 A unit test crafts a non-zero-exit Output whose stderr contains ESC and embedded newline bytes and asserts the rendered error string contains no raw control bytes (the bytes are escaped or stripped).
- [ ] #3 Regression: existing format_error_tail CR/CRLF/bare-CR tests still pass; the cargo-update provider's happy-path tests are unaffected.
<!-- AC:END -->
