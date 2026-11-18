---
id: TASK-2673
title: 'SEC-11: list items and priority are written into frontmatter unencoded, allowing key injection and unparseable files'
status: To Do
assignee: []
created_date: '2026-11-18 18:26'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2837'
modified_files:
  - crates/backlog/src/model.rs
priority: high
ordinal: 1000
dedup_key: 'SEC-11:crates/backlog/src/model.rs:Frontmatter::render_into'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/model.rs:379-410` (`Frontmatter::render_into`), `crates/backlog/src/model.rs:597-606` (`render_list`)

**What**: `title`, `status` and extras go through `yaml_scalar` / `bare_or_quoted`, but `render_list` writes every list item (`assignee`, `labels`, `dependencies`, `modified_files`) as a raw `  - {item}` line, and `priority` / `ordinal` / `id` are written bare with no encoding or validation. Nothing at the CLI boundary rejects control characters either.

Reproduced on ops 0.94.1 in a scratch backlog:
- `ops backlog task create y -l $'a\nstatus: Done'` writes a second `status: Done` line into the frontmatter; the parser takes the last one, so the task reads back as Done (any key can be injected this way — id, parent_task_id, status).
- `ops backlog task create x --priority ""` writes `priority: ` which re-parses as an empty block list; `Frontmatter::parse` then errors (`priority carries a list where a scalar was expected`) and **every** subsequent `task list` / `search` / keyed `create` fails because `scan_tasks` aborts on one unparseable file.

The `yaml_scalar` doc comment claims "Values are validated at the provider boundary" — they are not.

**Why it matters**: Values come from LLM agents and shell scripts; one bad label or empty priority silently corrupts task state or bricks the whole backlog for every command.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 List items containing control characters, a leading/trailing space, or YAML-significant characters are either encoded so they round-trip or rejected with an error naming the field
- [ ] #2 priority is validated (or encoded) so an empty or multi-line value cannot produce an unparseable file
- [ ] #3 Round-trip tests: render then parse of a Frontmatter with a newline-bearing label and empty priority yields the same values or a create-time error, never an injected key
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.01 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
