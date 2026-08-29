---
id: TASK-2052
title: >-
  SEC-33: the walking providers do not poll Context::check_deadline, so their
  dispatch bound is only post-hoc
status: To Do
assignee:
  - TASK-2060
created_date: '2026-08-29 13:03'
updated_date: '2026-08-29 17:27'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/tokei/src/lib.rs
  - extensions-rust/loc/src/lib.rs
  - extensions/text-fixers/src/discovery.rs
  - crates/extension/src/data.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs`, `extensions-rust/loc/src/lib.rs`, `extensions/text-fixers/src/discovery.rs`

**What**: TASK-2017 put a wall-clock bound on provider dispatch in
`crates/extension/src/data.rs`: `DataRegistry::provide` installs a deadline on
the `Context` for the outermost provider, exposes it as
`Context::check_deadline()`, and converts an over-budget return into
`DataProviderError::TimedOut`. That makes every dispatch bounded *by
construction* and makes an overrun visible and attributable.

What it does not do is shorten the stall for a provider that never polls.
`provide` is synchronous and runs on the caller's thread, so the deadline is
cooperative: the tree-walking providers still run their walk to completion and
only then get told they were too slow. The three walkers that motivated the
finding — tokei (`lib.rs:228`), rust-loc, and text-fixers
(`discovery.rs:221`) — have not been converted. Each needs
`ctx.check_deadline()?` in its per-entry loop, which for text-fixers and
rust-loc means threading `&Context` (or the deadline) into the `walk` helpers
that currently take only a root path.

Separately, no cooperative check can interrupt a thread already blocked in a
syscall — a wedged NFS `readdir` still hangs the CLI. Bounding *that* means
running the dispatch off-thread or making the trait async, which is a breaking
change to `DataProvider` and was deliberately left out of TASK-2017's scope.

**Why it matters**: SEC-33 — resource exhaustion. The dispatch bound is real
but is currently a label on the stall rather than a cure for it in exactly the
providers the original finding named.

**Origin**: discovered during TASK-2047 while fixing TASK-2017; the walking
providers are outside that wave's file scope and the async/off-thread rework
exceeds it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The tokei, rust-loc and text-fixers walk loops call Context::check_deadline() once per entry and propagate the error
- [ ] #2 A test drives one of those providers with a short Context budget and asserts it aborts before completing the walk
- [ ] #3 Whether provider dispatch should run off-thread or the trait become async is decided and recorded, not left implicit
<!-- AC:END -->
