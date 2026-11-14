---
id: TASK-2660
title: 'Add ops backlog task create --parent <id>: link a new task to its parent in the same call'
status: Done
assignee: []
created_date: '2026-11-13 18:36'
updated_date: '2026-11-13 19:19'
labels:
  - feature
  - backlog
  - skills-integration
dependencies: []
modified_files:
  - crates/cli/src/args.rs
  - crates/cli/src/backlog_cmd.rs
  - crates/backlog/src/cmd/create.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: `ops backlog task edit` has `--parent` / `--clear-parent` (`crates/cli/src/args.rs:825`), but `ops backlog task create` (`BacklogCreateArgs`, `crates/cli/src/args.rs:734`) has no way to set `parent_task_id`. A task filed as a child of another needs a second `edit --parent` call.

**Why it matters**: the review skills (`code-review-rust`, `code-review-web`, `code-review-lovable`) are often run under a tracking task (e.g. dbsec `TASK-1764.02`, "REVIEW: run code-review-rust against dbsec-derive"). Their findings came out top-level because the create template has nothing to carry the link, and the follow-up edit was skipped. A second call is also not atomic with `--unless-exists`: when create prints `Exists <id>`, the caller has to decide whether to re-parent a task it did not file.

**Shape**: `--parent <TASK-ID>` on create, validated like edit's (the parent must exist; same id normalisation). Combined with `--unless-exists`, an `Exists` hit leaves the existing task's parent untouched and says so.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ops backlog task create --parent TASK-N writes parent_task_id in the new task's frontmatter
- [x] #2 A parent id that does not exist is refused before anything is written
- [x] #3 With --unless-exists, an Exists hit does not change the existing task's parent
- [x] #4 The create help text and the backlog docs list the flag

<!-- AC:END -->
