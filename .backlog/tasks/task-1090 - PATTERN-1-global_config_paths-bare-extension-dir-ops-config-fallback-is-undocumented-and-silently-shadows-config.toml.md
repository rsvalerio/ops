---
id: TASK-1090
title: >-
  PATTERN-1: global_config_path's bare-extension '<dir>/ops/config' fallback is
  undocumented and silently shadows config.toml
status: Done
assignee: []
created_date: '2026-05-07 21:31'
updated_date: '2026-05-08 06:35'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:268-284`

**What**: `load_global_config` tries `global_path.with_extension("toml")` first then the bare `global_path` (`<dir>/ops/config` with no extension). The bare-extension form is undocumented in the rustdoc (which only mentions `config.toml`), is unconventional, and silently shadows config.toml if both exist.

**Why it matters**: A stray `~/.config/ops/config` (e.g. an extracted backup) silently wins over no-config-toml — future foot-gun. Operators reading docs and creating only config.toml get expected behaviour, but the dead-code path is a surprise hazard.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Either remove the bare-extension fallback or document it explicitly with a precedence note in the function rustdoc
- [x] #2 If kept, log at tracing::debug! which file actually loaded so operators can diagnose silent shadowing
- [x] #3 A test pins the precedence order between config and config.toml
<!-- AC:END -->
