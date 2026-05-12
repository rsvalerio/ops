---
id: TASK-1333
title: >-
  READ-5: env_flag_enabled treats 'no'/'off'/'n' as truthy, contradicting common
  operator expectation
status: Done
assignee:
  - TASK-1386
created_date: '2026-05-12 16:26'
updated_date: '2026-05-12 23:44'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:107-115`

**What**: `env_flag_enabled` treats only empty, `0`, and `false` (case-insensitive) as off; any other value — including `no`, `off`, `n` — counts as enabled. Tools such as systemd, bash, and most CLI conventions accept those as falsy.

**Why it matters**: An operator setting e.g. `OPS_NONINTERACTIVE=no` to disable noninteractive mode silently enables it instead. Behaviour conflicts with documented "truthy convention" without being obvious from the function.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Values 'no', 'off', 'n' (case-insensitive) parse as off.
- [x] #2 Unit tests cover both the falsy-extension set and the prior empty/0/false set; doc-comment reflects the actual semantics.
<!-- AC:END -->
