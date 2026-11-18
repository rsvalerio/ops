---
id: TASK-2785
title: 'SEC-33: cfg(..) target-key evaluator recurses without a depth limit'
status: Done
assignee: []
created_date: '2026-11-18 18:44'
updated_date: '2026-11-18 21:18'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2857'
modified_files:
  - extensions/about/src/machine.rs
priority: low
ordinal: 1000
dedup_key: 'SEC-33:extensions/about/src/machine.rs:CfgParser::predicate'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/machine.rs:495-529` (`CfgParser::predicate` / `CfgParser::list`), entered from `cfg_matches` (line 447) for every `[target.'cfg(..)']` key in every config layer (`matching_cfg_tables`, line 534)

**What**: `predicate` calls `list`, which calls `predicate` again for each `all(`/`any(`/`not(` level, with no nesting bound. The input is a TOML table key from a repo-controlled `.cargo/config.toml`.

**Why it matters**: A key like `cfg(not(not(not(...))))` nested tens of thousands deep overflows the stack, aborting `ops about machine` instead of degrading to "unparseable cfg" as the function's doc promises. Real cargo cfg expressions are a handful of levels deep, so a small cap costs nothing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CfgParser tracks depth and returns None past a named const limit (with a rationale comment)
- [x] #2 A test feeds a deeply nested cfg key and asserts cfg_matches returns false without overflowing

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.22 (code-review-rust run, 2026-11-18).

Fixed in wave TASK-2857: CfgParser tracks list depth, MAX_CFG_DEPTH = 32 with rationale; test deeply_nested_cfg_key_does_not_match_or_overflow (100k levels).

<!-- SECTION:NOTES:END -->
