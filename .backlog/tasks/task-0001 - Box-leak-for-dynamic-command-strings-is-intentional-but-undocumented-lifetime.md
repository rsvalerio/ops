---
id: TASK-0001
title: 'Box::leak for dynamic command strings is intentional but undocumented lifetime'
status: Done
assignee: []
created_date: '2026-04-10 07:15:00'
updated_date: '2026-04-11 09:55'
labels:
  - rust-idioms
  - EFF
  - OWN-6
  - low
  - crate-cli
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/cli/src/main.rs:224-226`
**Anchor**: `fn leak`
**Impact**: `Box::leak` is used to produce `&'static str` from dynamic command names and help text for clap subcommands. The comment says "Safe here because this runs once at process exit (help display)" but `inject_dynamic_commands` is called on every invocation (line 99 and implicitly via `dispatch`), not just help display. Each leaked string is small and the function runs once per process, so the actual memory impact is negligible — but the comment is misleading.

**Notes**:
The `leak` helper converts owned `String`s to `&'static str` because clap's `Command::new` requires `&'static str` (or `impl Into<...>` depending on version). This is a well-known pattern for CLI apps. The comment should be corrected to say "runs once per process invocation" rather than "at process exit (help display)". Rule OWN-6: prefer `Cow` or owned types over leaked allocations when the API supports it — check if the clap version in use accepts `String` directly.
<!-- SECTION:DESCRIPTION:END -->
