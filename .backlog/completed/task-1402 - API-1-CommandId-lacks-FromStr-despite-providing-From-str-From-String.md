---
id: TASK-1402
title: 'API-1: CommandId lacks FromStr despite providing From<&str> / From<String>'
status: Done
assignee:
  - TASK-1456
created_date: '2026-05-13 18:10'
updated_date: '2026-05-14 07:39'
labels:
  - code-review-rust
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/commands.rs:249-323`

**What**: `CommandId` is a newtype intended for CLI use but provides only `From<&str>` / `From<String>` conversions — there is no `FromStr` impl, so callers cannot `.parse::<CommandId>()` nor use it as a `clap` value-parser target idiomatically.

**Why it matters**: Newtypes conventionally implement `FromStr` (with `type Err = Infallible` when conversion cannot fail). Without it, downstream integration with `clap`, `serde` string adapters, or generic `parse()` chains is awkward and the API feels incomplete.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 CommandId implements FromStr with Infallible error
<!-- AC:END -->
