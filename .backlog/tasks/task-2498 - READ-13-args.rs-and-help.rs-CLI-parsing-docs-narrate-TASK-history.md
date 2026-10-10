---
id: TASK-2498
title: 'READ-13: args.rs and help.rs CLI-parsing docs narrate TASK history'
status: To Do
assignee: []
created_date: '2026-10-10 15:31'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2611'
modified_files:
  - crates/cli/src/args.rs
  - crates/cli/src/help.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:crates/cli/src/args.rs:cli parsing docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/args.rs:981,994`, `crates/cli/src/help.rs:12,21`

**What**: Doc comments on the builtin-shadowing warning (`TASK-2306: [commands.<name>] entries clap resolves to a builtin...`) and on `is_toplevel_help` narrate change history: "PATTERN-1 (TASK-1377): global flags ... used to make the next positional look like a subcommand", "The set of such flags is derived from Cli::command() rather than transcribed (TASK-1750)".

**Why it matters**: "Used to" narration and RULE/TASK provenance tags are process artifacts (READ-13); the reader needs the current rule (consume one extra argv slot for value-taking globals), not the bug that motivated it. Note TASK-2075 already cleaned help-string rationale in args.rs; these doc-comment sites remain.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No /// line in args.rs or help.rs references a TASK id or a past implementation
- [ ] #2 The value-taking-global rule is stated as present-tense behaviour
<!-- AC:END -->
