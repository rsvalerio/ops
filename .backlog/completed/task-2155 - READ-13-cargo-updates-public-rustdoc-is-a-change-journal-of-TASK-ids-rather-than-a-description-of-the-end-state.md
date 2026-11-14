---
id: TASK-2155
title: 'READ-13: cargo-update''s public rustdoc is a change journal of TASK ids rather than a description of the end state'
status: Done
assignee: []
created_date: '2026-09-08 07:04'
updated_date: '2026-09-10 19:31'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2248'
modified_files:
  - extensions-rust/cargo-update/src/lib.rs
priority: low
ordinal: 68000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs`

**What**: Doc comments on public items narrate past bugs, removed implementations and the task that changed them, instead of describing what the item is now. These render in `cargo doc`:

- `UpdateAction::Downgrade` (:37-42) — the variant's *entire* doc is `PATTERN-1 / TASK-1778: ... It was previously dropped with no entry, no count and no log record.` Nothing says what the variant means.
- `CargoUpdateResult::downgrade_count` (:66-70) — `PATTERN-1 / TASK-1778:` plus an explanation of why `#[serde(default)]` is there.
- `CargoUpdateExtension` (:641) — `API-9 / TASK-0922: construct via the registered extension factory only.`
- `CARGO_UPDATE_TIMEOUT`'s neighbours and the private-but-rustdoc-visible-to-maintainers block on `strip_ansi` (:251-256), which spends six lines on `The previous \`bytes[i] as char\` cast interpreted each continuation byte as a Latin-1 code point and silently corrupted every multi-byte character.`
- `map_run_error` (:661-669) and `interpret_output` (:674-678), whose docs lead with `ERR-4 / TASK-1535:` and `TEST-5 / TASK-1787:` respectively.
- `is_version_shaped` (:522-529), `is_control_free` (:535-541), `match_verb` (:476-486), `is_index_progress_line` (:435-447), `ACTION_PREFIXES` (:421-427) — each opens with a rule id and a task number.

The rationale itself is often genuinely useful (why the `->` guard exists, why truncated escapes are preserved) and should stay. The problem is placement and framing: it belongs in a `//` implementation comment or below a real summary line, not as the item's rendered description, and it should describe the current constraint rather than the defect it replaced.

Note the same pattern is filed for other crates as TASK-2075 / TASK-2101 / TASK-2136; this task covers the cargo-update crate's own occurrences.

**Why it matters**: The reader of `cargo doc` gets no answer to "what is `UpdateAction::Downgrade`?" — only an account of a bug they cannot see and a task id they cannot open. The TASK references also rot: they point at a backlog that is archived and renumbered, so within a release or two the docs cite identifiers that resolve to nothing while still occupying the summary position.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every doc comment on a public item opens with a summary of what the item is or does in the current design
- [x] #2 Rationale worth keeping (the arrow guard, truncated-escape preservation, the scan caps, the serde default) is retained, moved below the summary or into an implementation comment, and phrased as the current constraint rather than as the history of a past defect
- [x] #3 No TASK id or rule id appears in the first line of a rendered doc comment

<!-- AC:END -->
