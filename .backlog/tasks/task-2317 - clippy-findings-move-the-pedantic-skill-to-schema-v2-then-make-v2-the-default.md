---
id: TASK-2317
title: 'clippy-findings: move the pedantic skill to schema v2, then make v2 the default'
status: Triage
assignee: []
created_date: '2026-09-27 15:30'
updated_date: '2026-09-27 16:52'
labels:
  - code-review-rust
  - consistency
  - clippy
  - json
dependencies: []
modified_files: []
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `rsvalerio/ai: skills/rust-make-clippy-pedantic/SKILL.md`, `references/extraction.md`, `references/lint-catalog.md` (external repo); ops side: `crates/cli/src/clippy_findings_cmd.rs`, `crates/cli/src/args.rs`, `docs/clippy.md`

**What**: `ops clippy-findings` emits schema v1 (snake_case keys) by default. `--schema-version 2` opts into camelCase keys (`schemaVersion`, `manifestDir`, `targetKind`, `droppedOutOfTree`, `rustcWarnings`), matching the other ops JSON reports. v1 stays the default so existing consumers keep working. The rust-make-clippy-pedantic skill in rsvalerio/ai still reads the v1 keys.

**Why it matters**: the goal of TASK-2312 (camelCase like every other ops report) is only half done while v1 is the default. The default can flip once the known consumers pass `--schema-version 2`. Flipping the default is a breaking CLI change and needs a breaking-change commit.

**Origin**: TASK-2316 / TASK-2312. A review of PR #75 asked for a v1-compatible output path, and the user chose v1 default with v2 opt-in.

<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 rust-make-clippy-pedantic runs `ops clippy-findings --schema-version 2`, reads the camelCase keys and checks schemaVersion == 2
- [ ] #2 After that ships, the ops default flips to 2 in a breaking-change commit; docs/clippy.md is updated

<!-- AC:END -->
