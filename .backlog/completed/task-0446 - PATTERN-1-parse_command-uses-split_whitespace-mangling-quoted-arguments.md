---
id: TASK-0446
title: 'PATTERN-1: parse_command uses split_whitespace, mangling quoted arguments'
status: Done
assignee:
  - TASK-0536
created_date: '2026-04-28 05:43'
updated_date: '2026-04-28 16:13'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/new_command_cmd.rs:51-56`

**What**: `parse_command("cargo install --features \"a b\"")` returns args `["install", "--features", "\"a", "b\""]`. The implementation calls `input.split_whitespace()`, ignoring quotes and backslash escapes.

**Why it matters**: The help message advertises shell-style command pasting; users who follow it with quoted args (paths with spaces, JSON literals, regex patterns) get silently-wrong TOML. The bug only surfaces when the resulting command runs and fails strangely.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Use shlex::split (or equivalent) to honour shell quoting; bail with a clear error on unbalanced quotes
- [x] #2 Tests cover quoted args, escaped quotes, and a clear failure mode for malformed input
<!-- AC:END -->
