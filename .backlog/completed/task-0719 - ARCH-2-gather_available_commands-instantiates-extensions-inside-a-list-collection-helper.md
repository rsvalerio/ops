---
id: TASK-0719
title: >-
  ARCH-2: gather_available_commands instantiates extensions inside a
  list-collection helper
status: Done
assignee:
  - TASK-0740
created_date: '2026-04-30 05:31'
updated_date: '2026-04-30 19:20'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/hook_shared.rs:73-138`

**What**: `gather_available_commands` is shaped like a pure config -> Vec<SelectOption> helper, but its third source path (line 109-135) calls `crate::registry::builtin_extensions(config, cwd)` which performs disk I/O (factory probes, optional filesystem detection per extension), then runs every extension`s register_commands which itself can do I/O per extension (per the comment on extension_summary at extension_cmd.rs:96-100). The function name suggests cheap data shaping; the actual cost depends on every compiled-in extensions probe. There is also no caching across calls — `run_hook_install` invokes it once but other callers (TASK-0445 fixed the swallow) may invoke it on every prompt refresh.

**Why it matters**: ARCH-2 (depend on traits at module boundaries) is violated: a pre-built `&CommandRegistry` (or trait abstraction) should be passed in, not re-instantiated inside a helper that callers reasonably assume is a synchronous accessor. The current shape (a) duplicates work with run_command_cli/setup_extensions, (b) re-emits the duplicate-registration WARN every time gather_available_commands is called, and (c) makes the function untestable without compiling-in extension state. Refactor: take a precomputed CommandRegistry or extension-list parameter; let main wire it once.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 gather_available_commands accepts a pre-built CommandRegistry (or trait providing it) rather than calling builtin_extensions+register_extension_commands internally
- [ ] #2 run_hook_install builds the registry once (matching the run_cmd::setup_extensions wiring) and passes it down; existing tests still pass; one new test injects a mock registry without compiling extensions
<!-- AC:END -->
