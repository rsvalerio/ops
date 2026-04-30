---
id: TASK-0716
title: >-
  READ-5: register_extension_commands seeds owners with sentinel
  "<pre-existing>" that surfaces in WARN logs
status: To Do
assignee:
  - TASK-0740
created_date: '2026-04-30 05:30'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry.rs:140-187`

**What**: `register_extension_commands` seeds the `owners: HashMap<CommandId, &static str>` map with `<pre-existing>` (line 148) for every key already present in `registry` before this call. Later, when an extension reuses one of those ids, the `if prev != ext.name()` branch on line 174-181 fires and emits `tracing::warn!(first = %prev, second = %ext.name(), ...)`. `prev` will be the literal string `"<pre-existing>"` whenever the collision is against a pre-seeded entry. That is a leaking implementation detail: operators reading the warning see `first=<pre-existing> second=ext_a` and have no way to know whether the prior owner was the runner, an earlier register call, or another path entirely.

**Why it matters**: WARN logs are the operator-facing diagnostic for the CommandRegistry collision policy (SEC-31 / TASK-0402). Surfacing internal sentinels in the user-visible field undermines the diagnostic and breaks the contract that the warning fields name the responsible extension. Either (a) skip the WARN for `<pre-existing>` collisions because the runner has its own duplicate-warning at register_commands (mod.rs:209), or (b) replace the sentinel with a typed enum so the warning can render `first=runner-pre-registered` (or omit `first` entirely when unknown).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 owners map switches from &static str to a typed enum (PreExisting | Extension(name)) so the warning either suppresses the pre-existing case or names the source explicitly
- [ ] #2 No WARN log surfaces the literal substring <pre-existing> at the user-facing boundary; existing tests still pass; new test asserts the WARN field excludes the sentinel
<!-- AC:END -->
