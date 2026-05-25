---
id: TASK-1089
title: >-
  ERR-1: resolve_alias returns None for an orphan config alias instead of
  falling back to non-config alias map
status: Done
assignee: []
created_date: '2026-05-07 21:31'
updated_date: '2026-05-08 06:54'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/resolve.rs:112-121` (also `canonical_with_spec` at 94-97)

**What**: If `config.resolve_alias(alias)` returns `Some(name)` but `config.commands.get(name)` is `None` (alias points to a deleted/typo'd command — possible when alias maps survive a config edit that removed the underlying entry), `resolve_alias` returns `None` and never consults `non_config_alias_map`. The same alias may be defined by a stack default or extension; users see "unknown command" for an alias that should still resolve.

**Why it matters**: User-visible "alias broken" cliff after a config edit; canonical_with_spec has the same orphan trap.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 An orphan config alias falls through to the non_config_alias_map path instead of short-circuiting to None
- [x] #2 A regression test seeds a config with aliases on a command, then deletes the command, and verifies resolve finds a stack-default of the same name
- [x] #3 Apply the same fix to canonical_with_spec so single-pass and double-pass paths agree
<!-- AC:END -->
