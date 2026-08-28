---
id: TASK-1830
title: >-
  PERF-1: check_json builds a full serde_json::Value to answer a yes/no question
  — 5x memory amplification
status: To Do
assignee:
  - TASK-2004
created_date: '2026-08-27 15:20'
updated_date: '2026-08-28 14:15'
labels:
  - code-review-rust
  - performance
dependencies: []
modified_files:
  - extensions/config-checkers/src/json.rs
  - extensions/config-checkers/Cargo.toml
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/json.rs:15-26` (`check_json`, both branches)

**What**: the module doc says "Parse-only JSON validation" and the function's only output is `Result<(), CheckError>` — the parsed document is discarded on the very next line (`.map(|_| ())`). But both branches deserialize into `serde_json::Value`, which materialises the entire document as an owned tree of `Map<String, Value>` / `Vec<Value>` / `String` before it is thrown away:

```rust
json5::from_str::<serde_json::Value>(text).map(|_| ())
serde_json::from_slice::<serde_json::Value>(bytes).map(|_| ())
```

`serde::de::IgnoredAny` is the type serde provides for exactly this: it drives the same parser over the same input and validates the same grammar, but allocates nothing. Measured with a tracking global allocator on a ~1 MB JSON document (20 000 keys, each a 4-element array):

```
input bytes         =   986,671
Value    peak delta = 5,072,916   (5.1x the input)
IgnoredAny peak delta =         8
```

Error parity was checked at the same time — the rendered messages are byte-identical, and trailing garbage is still rejected:

```
bad strict Value   err = Some("expected value at line 1 column 7")
bad strict Ignored err = Some("expected value at line 1 column 7")
trailing Ignored   err = Some("trailing characters at line 1 column 9")
json5::from_str::<IgnoredAny>("{ a: 1, }") -> ok ; ("{ a: }") -> err
```

**Why it matters**: PERF-1 — an intermediate collection built solely to be dropped. It is not academic here because the crate sets its own budget: `DEFAULT_MAX_BYTES` is 16 MiB (`lib.rs:42`), so a file at the cap currently costs ~16 MiB for the `Vec<u8>` from `fs::read` plus roughly **80 MiB** of transient `Value` tree, per file, on the CI runners and pre-commit hosts the cap's doc comment is written to protect. `IgnoredAny` removes the second term entirely and leaves the first as the only bound that matters.

**One behavioural difference that must be handled, not ignored**: `serde_json`'s `deserialize_ignored_any` skips values *iteratively* (explicit stack, not native recursion), so it does not enforce `RECURSION_LIMIT = 128`. Probed at the pinned version:

```
depth     200: Value=Some("recursion limit exceeded at line 1 column 128")  Ignored=None
depth  10,000: Value=Some("recursion limit exceeded at line 1 column 128")  Ignored=None
depth 1,000,000: Value=Some("recursion limit exceeded at line 1 column 128") Ignored=None
```

No stack overflow at any depth (that is the good half), but the strict branch would stop rejecting deeply nested documents. That is the same depth bound TASK-1809 is adding for the `allow_json5` branch, so the two should land together: introduce the explicit depth cap once, apply it to both branches, and then `IgnoredAny` is a pure win rather than a silent relaxation.

**Fix shape**: add `serde` (already a workspace dependency, `Cargo.toml:46`) to the crate and deserialize `serde::de::IgnoredAny` in both branches. Sequence it after (or with) TASK-1809's depth cap so the strict branch does not lose its nesting bound in the process.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 check_json validates without materialising a serde_json::Value (serde::de::IgnoredAny or equivalent) in both the strict and allow_json5 branches
- [ ] #2 The strict branch still rejects deeply nested input — the nesting bound is explicit rather than inherited from Value's RECURSION_LIMIT (coordinate with TASK-1809)
- [ ] #3 Existing error messages and accept/reject behaviour are unchanged, including rejection of trailing characters after a complete document
<!-- AC:END -->
