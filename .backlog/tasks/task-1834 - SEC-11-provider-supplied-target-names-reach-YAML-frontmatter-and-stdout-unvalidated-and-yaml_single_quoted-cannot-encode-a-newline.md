---
id: TASK-1834
title: >-
  SEC-11: provider-supplied target names reach YAML frontmatter and stdout
  unvalidated, and yaml_single_quoted cannot encode a newline
status: To Do
assignee:
  - TASK-2005
created_date: '2026-08-27 15:22'
updated_date: '2026-08-28 14:16'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/create-review-tasks/src/lib.rs
  - extensions/create-review-tasks/src/backlog.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/backlog.rs:224-233` (`yaml_single_quoted`), `extensions/create-review-tasks/src/lib.rs:131-151` (`fetch_review_targets`), `388-415` (`report`)

**What**: `fetch_review_targets` decodes the `review_targets` payload with a bare `serde_json::from_value` and applies **no validation** to `ReviewTargets::skill` or `ReviewTarget::{name, path}` — no length bound, no character-class check, no rejection of control characters. Those strings then reach three sinks verbatim:

1. `render_task_file` → `title: {}` via `yaml_single_quoted` (backlog.rs:206)
2. `slugify` → the on-disk filename
3. `report` → `writeln!` straight to `stdout` (lib.rs:400-406)

`yaml_single_quoted` doubles `'` and stops there, but its doc comment claims it is *"safe for any future label/title shape"*. It is not: **a newline cannot be represented in a YAML single-quoted scalar at column 0**. A target named `ops\ncore` renders

```
title: 'REVIEW: Run skill code-review-rust against ops
core'
status: To Do
```

which a YAML parser rejects — verified with PyYAML, which reports `ComposerError: expected a single document in the stream … but found another document` at the closing `---`, because the malformed scalar desynchronises the document and the terminator is read as a new document marker. The file the crate just wrote is unreadable by the backlog.md CLI, and the run reports success. `slugify` maps the newline to `-`, so the filename looks fine and nothing signals the corruption. Any other control character (`\r`, `\t`, `\x00`) is likewise emitted raw where YAML requires a double-quoted scalar with escapes.

This is reachable. The registered producer is `extensions-rust/create-review-tasks/src/provider.rs`, which takes `name` from `[package].name` in each workspace member's `Cargo.toml`, falling back to a path-derived display name. TOML multi-line basic strings carry literal newlines, so a checked-out repository controls this value — and that crate's own doc comment already names the threat model: *"tracing path fields use the `?` formatter so **attacker-controlled member names** cannot forge log records."* The producer defends its log sink; the consumer defends none of its three.

The `report` sink has the same gap in the other direction: `subtask.title` and `subtask.path` are written to `stdout` unescaped, so a member name containing `\n` or an ANSI CSI sequence forges additional `created TASK-…` lines in the run report, or rewrites the terminal around them.

Related boundary hole in `slugify` (backlog.rs:167): it tests `is_ascii_alphanumeric`, so every non-ASCII letter collapses to `-` — `naïve-crate` slugs to `na-ve-crate`, and a fully non-ASCII name slugs to the empty string, yielding `task-0042.01 - .md`.

**Why it matters**: The crate's stated contract is that its output is "indistinguishable from CLI-created ones when re-read by the CLI" (backlog.rs:1-8). A task file the CLI cannot parse violates that contract silently — the run prints `created TASK-…`, the rollback guard sees no error, and the breakage surfaces later as a backlog tree the CLI refuses to load. Validating at the one boundary where the payload enters (SEC-11 layer 3: format) is cheaper and more honest than hardening three sinks, and it is the layer `fetch_review_targets` currently skips entirely.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 fetch_review_targets validates the decoded payload before it is used: skill and every target name/path are rejected (as an anyhow error naming the offending field and value) when empty, over a documented length bound, or containing any control character including newline, carriage return and ESC
- [ ] #2 yaml_single_quoted either refuses values it cannot encode or switches to a double-quoted scalar with escapes; its doc comment no longer claims safety for 'any future label/title shape' unless that is actually true
- [ ] #3 a test writes a task file for a title containing a newline and asserts the result is either a rejected run or a file whose frontmatter still parses as a single YAML document
- [ ] #4 report does not emit unvalidated provider strings containing control or ANSI escape sequences to stdout
- [ ] #5 slugify's behaviour on non-ASCII and fully-non-ASCII target names is pinned by a test, and a name that slugs to the empty string cannot produce a 'task-NNNN.MM - .md' filename
<!-- AC:END -->
