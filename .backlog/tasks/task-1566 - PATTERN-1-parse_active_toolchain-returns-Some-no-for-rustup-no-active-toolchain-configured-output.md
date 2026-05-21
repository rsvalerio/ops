---
id: TASK-1566
title: >-
  PATTERN-1: parse_active_toolchain returns Some("no") for rustup 'no active
  toolchain configured' output
status: Done
assignee:
  - TASK-1575
created_date: '2026-05-19 16:09'
updated_date: '2026-05-19 17:12'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe/rustup.rs:37-52` (parser) and `extensions-rust/tools/src/tests.rs:239-249` (test pins the bug)

**What**: `parse_active_toolchain` takes the first whitespace-bounded token of the first non-diagnostic line. When rustup emits `"no active toolchain configured\n"` (the rustup >=1.28 phrasing without a `error:` prefix), the first token is `no`, which is not in `RUSTUP_DIAGNOSTIC_PREFIXES`, so the parser returns `Some("no".to_string())`. The test `parse_active_toolchain_rejects_no_active_toolchain_message` (tests.rs:241-244) explicitly asserts this — the assertion name says "rejects" but the body pins acceptance.

The string `"no"` then flows downstream to `install_rustup_component(component, "no")`, where `validate_cargo_tool_arg` will accept it (alphanumeric, no dash, no dot), and finally to `rustup component add <component> --toolchain no`, which is an invalid toolchain and produces a confusing rustup error instead of the operator-facing "no active toolchain configured" diagnostic the user should see.

**Why it matters**:
- Correctness: an explicit rustup error message is silently converted into a fake toolchain name, then propagated through install flows that the SEC-13 validator cannot catch because `no` is a syntactically valid identifier.
- The misleading test name (`*_rejects_no_active_toolchain_message`) actively documents-as-correct what is, on reading, a defect. Future contributors will preserve the buggy behaviour because the test pins it.
- Operator experience: instead of "rustup has no active toolchain — run `rustup default stable`", the operator sees a generic rustup `--toolchain no` failure.

**Fix sketch**: reject any first token containing only ASCII letters and lacking a `-` or version-like suffix when the line begins with a known rustup status prefix word (`no`, `none`, `installed`, ...) OR — simpler and more robust — require that the returned token contain at least one of `-`/`.`/`:` so it has the shape of an actual toolchain identifier (`stable-aarch64-apple-darwin`, `1.70.0-...`, `linked:custom`). Update `parse_active_toolchain_rejects_no_active_toolchain_message` to assert `None` for the "no active toolchain configured" line, and add a regression test covering `none configured` and similar.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 parse_active_toolchain returns None for 'no active toolchain configured' input
- [ ] #2 Test currently asserting Some("no") is updated to assert None and renamed to reflect the assertion
- [ ] #3 New regression test covers at least one additional bare-word rustup status line ('none configured', 'unknown') returning None
- [ ] #4 Existing positive cases (stable-aarch64-apple-darwin, linked:custom-toolchain, 1.70.0-...) continue to parse
<!-- AC:END -->
