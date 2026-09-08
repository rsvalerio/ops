---
id: TASK-2138
title: >-
  API-14: config-checkers' extension surface and public struct fields carry no
  doc summaries
status: To Do
assignee:
  - TASK-2247
created_date: '2026-09-08 06:56'
updated_date: '2026-09-08 11:00'
labels:
  - code-review-rust
  - api-design
dependencies: []
modified_files:
  - extensions/config-checkers/src/lib.rs
  - extensions/config-checkers/src/options.rs
  - extensions/config-checkers/src/report.rs
priority: low
ordinal: 54000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/lib.rs:30-32`, `extensions/config-checkers/src/lib.rs:49`, `extensions/config-checkers/src/options.rs:10-11`, `extensions/config-checkers/src/report.rs:27-29`, `extensions/config-checkers/src/report.rs:39`

**What**: public items with no `///` summary. Candidate list, verified by
reading each site:

- `lib.rs:30` `pub const NAME: &str`
- `lib.rs:31` `pub const DESCRIPTION: &str`
- `lib.rs:32` `pub const SHORTNAME: &str`
- `lib.rs:49` `pub struct ConfigCheckersExtension` — the crate's extension entry point, no doc at all
- `options.rs:10` `CheckerOptions::root` — undocumented, while the three fields beneath it (`allow_json5`, `max_bytes`, `tracked_only`'s neighbours) are documented
- `options.rs:11` `CheckerOptions::tracked_only` — undocumented; the meaning ("consult the git index instead of walking") is only discoverable from `runner.rs`
- `report.rs:27-29` `FailedFile::{path, kind, message}` — all three undocumented, and `path` in particular is root-relative (`runner.rs:278 relative_to`), which nothing in the type says
- `report.rs:39` `CheckerReport::files_failed` — undocumented, while `files_scanned`, `files_skipped` and `walk_errors` each carry a paragraph

**Why it matters**: API-14 — the rest of this crate is documented to an
unusually high standard (every limit, every skip reason, every security
decision has a rationale attached), which makes the gaps read as
"intentionally uninteresting" rather than "missed". They are not:
`tracked_only` selects between two entirely different candidate sets, and
`FailedFile::path` being root-relative is a contract callers will otherwise
learn by writing a path that does not resolve.

`ConfigCheckersExtension` is the item a reader arrives at first from the
extension registry and the only one with no prose whatsoever.

Note: the crate does not set `#![warn(missing_docs)]`, so none of this is
mechanically enforced today.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every public item listed above has a doc summary; CheckerOptions::tracked_only states which candidate set each value selects, and FailedFile::path states that it is relative to CheckerOptions::root
- [ ] #2 ConfigCheckersExtension has a doc comment describing what it registers
- [ ] #3 The crate root enables #![warn(missing_docs)] (or the workspace equivalent) so the gap cannot silently reopen, and the crate builds clean under it
<!-- AC:END -->
