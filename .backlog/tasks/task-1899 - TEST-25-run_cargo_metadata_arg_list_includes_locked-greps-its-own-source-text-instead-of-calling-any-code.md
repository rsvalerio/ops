---
id: TASK-1899
title: >-
  TEST-25: run_cargo_metadata_arg_list_includes_locked greps its own source text
  instead of calling any code
status: Triage
assignee: []
created_date: '2026-08-27 15:37'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-rust/metadata/src/tests/wiring.rs
  - extensions-rust/metadata/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/tests/wiring.rs:119-131`

**What**: The test does not invoke `run_cargo_metadata`, or anything else in the crate. It reads `lib.rs` as a string at compile time and searches it for a literal:

```rust
let src = include_str!("../lib.rs");
let needle = "[\"metadata\", \"--format-version\", \"1\", \"--locked\"]";
assert!(src.contains(needle), ...);
```

This passes if the string appears anywhere in `lib.rs` — inside a doc comment, inside a `#[cfg(test)]` block, inside dead code, or in a second function that is never called. It fails if `cargo fmt` ever wraps that argument list across lines, if the arguments are reordered, or if `--locked` is factored into a shared constant, none of which change behaviour. And it keeps passing if `run_cargo_metadata` is deleted outright, provided the literal survives somewhere in the file.

The comment concedes the shape ("This is a coarse pin") and justifies it as avoiding "a fake `cargo` on PATH". That constraint is real but the conclusion does not follow: `run_cargo_metadata` is a pure function of its arguments over `run_cargo`, so the arg list is testable by extracting it (`const CARGO_METADATA_ARGS: [&str; 4]`) and asserting on the constant, or by having `run_cargo` accept an injectable command builder. Either is a real assertion about the value the production path uses.

**Why it matters**: TEST-25. A test that asserts on source text tests the formatter, not the program. It also produces the most misleading possible failure — a `cargo fmt` run reports "run_cargo_metadata arg list must include --locked (TASK-1059)" when `--locked` is right there. The invariant it guards (TASK-1059: the read-only ingestor must not mutate Cargo.lock) is worth guarding; this is not a guard.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The --locked assertion reads the actual argument value the production call site passes (e.g. a named const or an injected command builder), not include_str! of the source file
- [ ] #2 Reformatting lib.rs cannot fail the test, and deleting or bypassing run_cargo_metadata does fail it
- [ ] #3 No test in this crate asserts on include_str! of a source file
<!-- AC:END -->
