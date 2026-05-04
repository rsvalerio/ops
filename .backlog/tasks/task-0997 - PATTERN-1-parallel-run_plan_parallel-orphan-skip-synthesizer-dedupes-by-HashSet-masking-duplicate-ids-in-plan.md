---
id: TASK-0997
title: >-
  PATTERN-1: parallel run_plan_parallel orphan-skip synthesizer dedupes by
  HashSet, masking duplicate ids in plan
status: Triage
assignee: []
created_date: '2026-05-04 22:01'
labels:
  - code-review-rust
  - idioms
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/parallel.rs:295-326`

**What**: `run_plan_parallel` tracks terminal-event ids in
`terminal_ids: HashSet<CommandId>`. After draining the channel it walks
`command_ids` and emits a synthetic `StepSkipped` for any id whose
terminal event never arrived:

\`\`\`rust
for id in command_ids {
    if !terminal_ids.contains(id) {
        on_event(RunnerEvent::StepSkipped { id: id.clone(), display_cmd: None });
    }
}
\`\`\`

If the same command id appears twice in `command_ids` (a composite that
fans out the same leaf id twice — legal in the current expansion model:
`expand_to_leaves` does not dedupe, only cycles are guarded) and *both*
slots are aborted before producing terminal events, the synthesizer fires
StepSkipped exactly **once** because the HashSet collapses the second
visit. JSON event consumers and the display then see one StepStarted
without a corresponding terminal event.

**Why it matters**: AC #2 in the task block above
(`terminal_ids.insert(id.clone())`) explicitly aimed to "pair every
StepStarted with a terminal event". The dedupe defeats that for the
duplicate-id case. The collect_join_results path is fine because each
JoinSet entry produces its own StepResult; only the synthesized SKIPPED
events drop. Suggestion: count occurrences (`HashMap<CommandId, usize>`)
or walk `id_map` (which preserves task identity) and synthesize one
StepSkipped per missing task instead of per missing id.

A simpler workaround if duplicate-id composites are out of scope: assert
in expand_to_leaves that the leaf list contains no duplicates, with a
clear error pointing the user at the offending composite.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 duplicate ids in a parallel plan either get one synthetic StepSkipped per occurrence, or are rejected up front with a typed error
- [ ] #2 regression test: a composite with [a, a] under fail_fast that aborts both still pairs every StepStarted with a terminal event
<!-- AC:END -->
