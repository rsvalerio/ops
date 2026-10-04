---
id: TASK-2394
title: 'READ-13: ops-git source comments, rustdoc and runtime log messages are a change journal of TASK ids and past behaviour'
status: Triage
assignee: []
created_date: '2026-10-04 14:15'
labels:
  - code-review-rust
  - READ
dependencies: []
modified_files:
  - extensions/git/src/config.rs
  - extensions/git/src/provider.rs
  - extensions/git/src/remote.rs
  - extensions/git/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/git/src:ops-git crate'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs`, `extensions/git/src/provider.rs`, `extensions/git/src/remote.rs`, `extensions/git/src/lib.rs`

**What**: Production docs and comments narrate history instead of the end state: ~93 `TASK-` references in config.rs (55 in production code), 46 in remote.rs, 12 in provider.rs, 1 in lib.rs. Examples: provider.rs `GitInfo::collect` ("previously the fallback shipped the post-redaction raw string verbatim ... SEC-2 / TASK-1102 already closed"), provider.rs schema comment ("this said 'Normalized https URL', the same claim PATTERN-1 / TASK-1237 invalidated"), lib.rs "the three cast allows this root used to carry are gone". Task IDs and rule IDs are also embedded in runtime `tracing::warn!` messages (config.rs:297, :302, :645, :659, `SEC-33: .git/config exceeds byte cap`), and a test asserts on `TASK-1215` in the logged output (config.rs ~:902), coupling a test to a ticket number.

**Why it matters**: Readers of the API/log output cannot use ticket narratives; they go stale and look authoritative. Operator-facing log text with ticket IDs is noise. Same class as the READ-13 fixes already landed for other crates.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Doc comments and inline comments in ops-git describe current behaviour and durable rationale only, with no TASK ids or past-behaviour narration
- [ ] #2 Runtime tracing messages carry no TASK/rule IDs; tests assert on structured fields or message substance, not a ticket number
<!-- AC:END -->
