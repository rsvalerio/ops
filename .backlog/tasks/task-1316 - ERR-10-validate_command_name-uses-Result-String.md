---
id: TASK-1316
title: 'ERR-10: validate_command_name uses Result<(), String>'
status: To Do
assignee:
  - TASK-1385
created_date: '2026-05-11 20:30'
updated_date: '2026-05-12 22:16'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/new_command_cmd.rs:61`

**What**: `fn validate_command_name(name: &str) -> Result<(), String>` returns a `String` as its error type. Callers must wrap each error with `.map_err(|e| anyhow::anyhow!(e))?` at line 43 to lift it into the `anyhow::Result` flow.

**Why it matters**: ERR-10 forbids `Result<T, String>` in library/internal APIs because string errors lose context-chain information and cannot be matched on. Every caller has to translate the error manually (already done once at line 43), and the inquire validator at lines 34-39 also has to wrap the same string in `Validation::Invalid(msg.into())`. A typed error or `anyhow::Result` would simplify both call sites.

The function is small and there is one production caller and one validator caller, so the change is localized.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 validate_command_name returns anyhow::Result<()> (or a domain error enum), not Result<(), String>
- [ ] #2 Caller at new_command_cmd.rs:43 drops the .map_err(...) wrap
- [ ] #3 Inquire validator at lines 34-39 still maps the resulting error to inquire::validator::Validation::Invalid(...)
- [ ] #4 cargo clippy --all-targets -- -D warnings and cargo test --workspace pass
<!-- AC:END -->
