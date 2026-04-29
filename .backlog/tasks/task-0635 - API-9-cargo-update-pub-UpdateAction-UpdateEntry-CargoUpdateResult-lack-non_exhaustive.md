---
id: TASK-0635
title: >-
  API-9: cargo-update pub UpdateAction, UpdateEntry, CargoUpdateResult lack
  #[non_exhaustive]
status: Done
assignee:
  - TASK-0636
created_date: '2026-04-29 05:50'
updated_date: '2026-04-29 06:17'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:26` (UpdateAction), `:34` (UpdateEntry), `:46` (CargoUpdateResult)

**What**: `pub enum UpdateAction { Update, Add, Remove }`, `pub struct UpdateEntry { action, name, from, to }`, and `pub struct CargoUpdateResult { entries, update_count, add_count, remove_count }` are all part of the public surface returned by `parse_update_output` / the data provider, but none carries `#[non_exhaustive]`.

**Why it matters**: API-9 — adding a new action verb (e.g. cargo-edit `Modify`/`Yank`) or another aggregate count is a SemVer break for callers exhaustively matching the enum or struct-initialising the result. Sister wave-40 task TASK-0611 already filed the same gap on `UpgradeEntry`/`AdvisoryEntry`/`DenyEntry` — same crate family, same pattern.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 UpdateAction, UpdateEntry, and CargoUpdateResult each carry #[non_exhaustive]
- [ ] #2 Internal pattern matches use wildcard arms where required
- [ ] #3 cargo build --all-targets and cargo test -p ops-cargo-update pass
<!-- AC:END -->
