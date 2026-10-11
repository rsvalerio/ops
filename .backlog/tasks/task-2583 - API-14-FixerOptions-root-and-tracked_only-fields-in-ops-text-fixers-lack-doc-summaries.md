---
id: TASK-2583
title: 'API-14: FixerOptions root and tracked_only fields in ops-text-fixers lack doc summaries'
status: Done
assignee: []
created_date: '2026-10-10 15:45'
updated_date: '2026-10-10 22:07'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2627'
modified_files:
  - extensions/text-fixers/src/options.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:extensions/text-fixers/src/options.rs:FixerOptions'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/options.rs:15-16`

**What**: `FixerOptions` is public (re-exported from the crate root) and two of its four fields carry no doc comment: `root: PathBuf` (line 15) and `tracked_only: bool` (line 16), while their siblings `max_bytes` and `check` are documented. `tracked_only` in particular has non-obvious semantics a field summary should state: it selects the git index via `git ls-files` and silently widens to a full filesystem walk (with a `Fallback` reason on the report) when git is unavailable or the root is not a repository — behavior currently documented only in `discovery::discover`, a private-path module away from the struct a consumer reads.

**Why it matters**: API-14 — public items carry the canonical doc sections, and the summary sentence is mandatory. `clippy::missing_docs` is not in the workspace lint set, so nothing mechanical catches the gap; the two undocumented fields are also an internal-consistency issue (READ-6) since the other half of the struct is documented. Precedent: TASK-2071 filed the same rule for ops-about.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every public field of FixerOptions has a doc summary; tracked_only's summary states the git-index selection and the documented fallback-to-walk widening
- [x] #2 cargo doc on the workspace succeeds with no new warnings

<!-- AC:END -->
