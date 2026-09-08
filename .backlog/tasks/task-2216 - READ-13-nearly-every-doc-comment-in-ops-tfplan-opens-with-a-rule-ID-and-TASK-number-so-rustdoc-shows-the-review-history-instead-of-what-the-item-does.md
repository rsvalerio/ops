---
id: TASK-2216
title: >-
  READ-13: nearly every doc comment in ops-tfplan opens with a rule ID and TASK
  number, so rustdoc shows the review history instead of what the item does
status: To Do
assignee:
  - TASK-2248
created_date: '2026-09-08 07:21'
updated_date: '2026-09-08 11:02'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions-terraform/plan/src/lib.rs
  - extensions-terraform/plan/src/model.rs
  - extensions-terraform/plan/src/render.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs`, `extensions-terraform/plan/src/model.rs:37-41`, `extensions-terraform/plan/src/render.rs:7-47`

**What**: The first line of most doc comments in this crate is a citation of the review finding that produced the code, not a summary of the item. Rustdoc renders the first sentence as the item's short description in every module and index listing, so those listings currently read as a changelog. Examples:

- `PlanOptions` — "FN-3 / TASK-1281: a single clap-derived struct is the canonical definition of every `ops plans` flag."
- `run_plan_pipeline` — "FN-9 / TASK-0850: thin wrapper that locks `io::stdout()` and delegates to …"
- `run_plan_pipeline_to_with_tty` — "PATTERN-1 / TASK-1017: explicit form that accepts …"
- `run_plan_pipeline_code` — an entire paragraph on why `ExitCode` has no `PartialEq` and why the test was missing
- `read_capped`, `with_artifact_cleanup`, `cleanup_artifacts`, `expand_path`, `write_plan_json`, `capture_plan_json`, `prepare_artifact_paths`, `reject_reserved_passthrough`, `show_failure_error` — same shape
- `render_resource_table` — three of its four paragraphs describe the *previous* behaviour and why it was wrong

Much of this content is genuinely valuable, but it is describing decisions and migrations ("used to", "previously", "the previous version re-derived both paths"), which is what READ-13 names: rationale and history belong in the task record and, where a reader of the code needs them, in `//` implementation comments — not in the `///` API surface. `Plan`, `ResourceChange`, `Change` and `ClassifiedChange` and their fields, meanwhile, carry no docs at all, so the rendered page is entirely history with no description.

**Why it matters**: The task IDs are dangling references for anyone outside this repo's `.backlog`, and they age badly — TASK-1017's "previously" is two refactors old and the reader cannot tell which sentences still describe current behaviour. A doc comment that leads with provenance also crowds out the thing callers actually need: `run_plan_pipeline_to` vs `run_plan_pipeline_to_with_tty` is a real choice a caller has to make, and the summary line spends itself on a rule ID instead of making it.

Keep the *invariants* (why `is_tty` and `use_color` are separate, why cleanup runs on both arms, why the cap must be uniform across ingress points) — restate them as present-tense properties of the code. Move the "used to / previously / TASK-N" narration out.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No public item's doc summary line opens with a rule ID or TASK number; each opens with a present-tense description of the item
- [ ] #2 Invariants that a caller or maintainer must preserve are kept, restated as properties of the current code rather than as a history of what changed
- [ ] #3 Remaining historical rationale, where it is still useful to a maintainer, lives in // implementation comments rather than /// docs
<!-- AC:END -->
