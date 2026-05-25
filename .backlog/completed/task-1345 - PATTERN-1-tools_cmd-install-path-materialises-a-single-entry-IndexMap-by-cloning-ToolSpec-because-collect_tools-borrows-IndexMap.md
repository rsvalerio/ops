---
id: TASK-1345
title: >-
  PATTERN-1: tools_cmd install path materialises a single-entry IndexMap by
  cloning ToolSpec because collect_tools borrows &IndexMap
status: Done
assignee:
  - TASK-1384
created_date: '2026-05-12 16:42'
updated_date: '2026-05-12 23:27'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/tools_cmd.rs:140-160`

**What**: The named-install branch builds `Option<IndexMap<String, ToolSpec>>` containing one entry by `.cloned()`-ing the spec from `config.tools`, solely so the downstream `collect_tools(&IndexMap)` API can borrow it. The comment acknowledges the indirection ("skip the deep-clone of every ToolSpec on the install hot path") but only for the unnamed branch — the named branch still pays one full ToolSpec clone per invocation.

**Why it matters**: The API is parameterised wrong: `collect_tools` should accept an iterator over `(&str, &ToolSpec)` (or a slice of `&ToolSpec`) so the single-tool case can borrow directly from `config.tools`. The current shape lies about the optimisation and forces the caller to allocate and clone.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Single-tool path borrows ToolSpec directly from config.tools — no clone, no single-entry IndexMap
- [ ] #2 collect_tools / install_missing_tools signature accepts an iterator or slice view; cargo test passes
<!-- AC:END -->
