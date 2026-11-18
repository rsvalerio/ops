---
id: TASK-2795
title: 'API-23: AboutOptions::new takes two positional bools (refresh, is_tty)'
status: To Do
assignee: []
created_date: '2026-11-18 18:45'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - API
dependencies: []
parent_task_id: 'TASK-2858'
modified_files:
  - extensions/about/src/lib.rs
  - crates/cli/src/subcommands.rs
priority: low
ordinal: 1000
dedup_key: 'API-23:extensions/about/src/lib.rs:AboutOptions::new'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/lib.rs:111-117` (`AboutOptions::new`); call sites `crates/cli/src/subcommands.rs:97`, `extensions/about/src/lib.rs:340` (`AboutOptions::new(false, None, false)`)

**What**: The only public constructor of the `#[non_exhaustive]` `AboutOptions` is `new(refresh: bool, visible_fields: Option<Vec<String>>, is_tty: bool)`. The two bools are separated only by the `Option`, so `new(is_tty, fields, refresh)` compiles, and `new(false, None, false)` cannot be read without opening the signature.

**Why it matters**: API-23 flags any public signature with two or more bools. Swapping `refresh` and `is_tty` silently changes cache semantics and ANSI output. Two-variant enums (`Refresh::{Cached, Recollect}`, or `Output::{Tty, Plain}`) or a builder over `Default` make each call site self-describing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 AboutOptions can no longer be built from two adjacent positional bools; refresh and tty-ness are named at each call site
- [ ] #2 All call sites updated
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.22 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
