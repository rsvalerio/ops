---
id: TASK-1030
title: >-
  PATTERN-1: cargo-update starts_with_known_verb matches verb prefixes without
  word boundary, e.g. 'Updatingxyz'
status: Done
assignee: []
created_date: '2026-05-07 20:23'
updated_date: '2026-05-07 23:35'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:216-220` (and the same shape in `parse_action_line` at 227-228 via `strip_prefix`)

**What**: `starts_with_known_verb` and the matching `parse_action_line` `strip_prefix` calls match a verb at the start of the line without enforcing that the verb is followed by whitespace or end-of-string. A line like `Updatingxyz serde v1 -> v2` matches `Updating`, `parse_action_line` strips that prefix, the remainder `xyz serde v1 -> v2` is destructured by `split_whitespace` as name=xyz from=serde arrow=v1 to=-> — `arrow != \"->\"` so the function returns None, but the line goes through `starts_with_known_verb` and emits a `warn!("skipping cargo-update line that begins with a known verb but did not parse — possible format drift")`. False-positive drift warnings appear whenever a future cargo line happens to begin with a known verb concatenated with a non-space (`UpdatingProgress: 50%`).

**Why it matters**: TASK-0472's intent is to make format drift loud. False positives from prefix-without-boundary matches train operators to ignore the warn, defeating the original purpose. Low severity because the user-facing parse output (CargoUpdateResult counts) is unaffected.

**Suggested fix**: change `line.starts_with(prefix)` to also require `line.as_bytes().get(prefix.len()).copied() == Some(b' ')` (or use `strip_prefix(prefix).and_then(|r| r.strip_prefix(' '))`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Verb prefix matches require a trailing whitespace boundary
- [ ] #2 Unit test pins that 'Updatingxyz ...' does not classify as a known verb
<!-- AC:END -->
