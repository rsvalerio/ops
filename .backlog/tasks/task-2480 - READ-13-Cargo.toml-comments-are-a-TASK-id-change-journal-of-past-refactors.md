---
id: TASK-2480
title: 'READ-13: Cargo.toml comments are a TASK-id change journal of past refactors'
status: To Do
assignee: []
created_date: '2026-10-10 15:29'
updated_date: '2026-10-10 21:13'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2615'
modified_files:
  - extensions-rust/cargo-update/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/cargo-update/Cargo.toml:Cargo.toml'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/Cargo.toml:11`

**What**: Three dependency comments narrate the crate's migration history rather than the end state:
- line 11: `# DUP-3 / TASK-2148: the ANSI grammar this crate used to carry privately lives in ops-theme; the parser consumes it via ...`
- line 22: `# DUP-3 / TASK-1794: the tracing-capture harness and the control-character assertions live in ops-about ...; this crate no longer keeps a private copy of either`
- line 26: `# TEST-9 / TASK-1803: the parser is a byte-oriented scanner over untrusted subprocess output with eight example-based regression fixes behind it.`

**Why it matters**: READ-13 — documentation describes the end state, not the journey. "Used to carry privately", "no longer keeps a private copy", and "eight fixes behind it" are process artifacts meaningless to a reader using the manifest, and the TASK-ids go stale. The repo has already stripped this exact class from ops-git (TASK-2394), ops-theme, and ops-about in recent commits; cargo-update was missed. What deserves to remain is the enduring fact (the ANSI grammar lives in ops-theme and is consumed via `strip_ansi_preserving_raw`) without the task tags and the before/after narration.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No Cargo.toml comment references a TASK id or narrates what the crate used to contain
- [ ] #2 Each dependency comment states only the enduring reason for the dependency
<!-- AC:END -->
