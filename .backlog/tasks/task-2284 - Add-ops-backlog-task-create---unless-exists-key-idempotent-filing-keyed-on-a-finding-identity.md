---
id: TASK-2284
title: 'Add ops backlog task create --unless-exists <key>: idempotent filing keyed on a finding identity'
status: Done
assignee: []
created_date: '2026-09-26 17:47'
updated_date: '2026-09-26 19:02'
labels:
  - feature
  - backlog
  - skills-integration
dependencies: []
parent_task_id: 'TASK-2293'
modified_files:
  - crates/backlog/src/cmd/create.rs
  - crates/backlog/src/model.rs
  - crates/backlog/src/store.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: `ops backlog task create … --unless-exists <key>` stores `<key>` on the task, for example in frontmatter. If an open task (not Done) already carries the same key, it creates nothing and prints that task's id. The check and the write happen under the store's allocation lock, so two concurrent runs cannot both file.

**Why**: every finding-filing skill does search-then-create by hand. Each defines an identity (`BF-<check>` + subject, `clippy::<lint>` + package + file + line + column + message, a rule id + location), runs `ops backlog search`, parses plain text, and decides. The skills document that this is "per-run, not a lock", so two concurrent runs file the same findings twice. Search is fuzzy (scored), so an exact key match is also more reliable than a search hit.

**Used by**: code-review-rust, code-review-web, rust-make-clippy-pedantic, rust-make-build-fast.
Source: skills-vs-ops audit of rsvalerio/ai dev-skills, 2026-09-26 (https://claude.ai/artifact/4yaKVkPdfe93hrqhFW5u1z).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A second create with the same --unless-exists key while the first task is open creates nothing and reports the existing id
- [x] #2 A key whose only task is Done files a new task
- [x] #3 Two concurrent creates with the same key produce exactly one task

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Key stored as `dedup_key` frontmatter (model::DEDUP_KEY); existing open task prints `Exists TASK-NNNN`. 'Open' = any status but the configured terminal column, in tasks/ only. The store had no allocation lock (it relied on create_new retry), so Store::lock_allocation was added: std File::lock on the tasks/ directory on Unix (no stray lock file), a persistent .allocation.lock file elsewhere; taken only for keyed creates. AC#3 pinned by an 8-thread barrier test.
<!-- SECTION:NOTES:END -->
