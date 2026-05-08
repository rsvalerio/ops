---
id: TASK-1156
title: 'FN-1: resolved_workspace_members is 110 lines with 5-deep nesting'
status: To Do
assignee:
  - TASK-1264
created_date: '2026-05-08 07:44'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - FN
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:293`

**What**: `resolved_workspace_members` (lines 293-403) mixes manifest access, glob-shape detection, read_dir walk, per-entry filtering, three tracing-warn arms, UTF-8 boundary handling, exclude filter, sort, and dedup at FN-2 nesting depth ~5 (`for member` → `if let star` → `match read_dir` → `for entry` → `match entry` → `if path.is_dir() && exists` → `if let Ok(rel)` → `match rel.to_str()`).

**Why it matters**: Single function carries six independent responsibilities. New glob shape (`{a,b}`, `**`) cannot be added without re-reading the entire walk. Cyclomatic complexity (FN-6) past McCabe 10.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract expand_member_glob(prefix, parent, root) -> Vec<String> covering the read_dir walk and per-entry boundary checks
- [ ] #2 Extract classify_member(spec) -> MemberShape::Literal | Glob | Unsupported so dispatch reads as a state machine
- [ ] #3 Top-level fn at orchestration: classify → expand → exclude/sort/dedup; cyclomatic complexity ≤10
<!-- AC:END -->
