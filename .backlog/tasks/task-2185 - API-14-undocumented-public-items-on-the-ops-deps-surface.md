---
id: TASK-2185
title: 'API-14: undocumented public items on the ops-deps surface'
status: To Do
assignee:
  - TASK-2247
created_date: '2026-09-08 07:13'
updated_date: '2026-09-08 11:00'
labels:
  - code-review-rust
  - api-design
dependencies: []
modified_files:
  - extensions-rust/deps/src/lib.rs
  - extensions-rust/deps/src/types.rs
  - extensions-rust/deps/src/parse/upgrade.rs
priority: low
ordinal: 98000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:59`, `extensions-rust/deps/src/lib.rs:380`, `extensions-rust/deps/src/types.rs:11`, `extensions-rust/deps/src/parse/upgrade.rs:417`

**What**: public items exported from the crate root with no doc summary:

- `lib.rs:59-62` — `pub const NAME`, `DESCRIPTION`, `SHORTNAME`, `DATA_PROVIDER_NAME`, all bare.
- `lib.rs:380` — `pub struct DepsProvider;` has no doc comment at all, despite being the crate's `DataProvider` implementation and a name-visible item.
- `types.rs` — every public field of `UpgradeEntry` (`name`, `old_req`, `compatible`, `latest`, `new_req`, `note`), `AdvisoryEntry`, `DenyEntry`, `UpgradeResult`, `DenyResult`, `DepsReport` is undocumented. `note` in particular carries load-bearing semantics: `categorize_upgrades` decides compatible vs breaking by substring-matching `"incompatible"` in it, and nothing on the field says so.
- `types.rs:51-59` — `LicenseEntry`, `BanEntry`, `SourceEntry` share one `///` line above the first of the three; the second and third have none.
- `parse/upgrade.rs:417` — `categorize_upgrades` has a one-line summary that does not state the classification rule.

The workspace does not enable `missing_docs`, so nothing catches these. Sibling crates already carry the same finding (TASK-2071 for ops-about, TASK-2098 for ops-extension, TASK-2164 for ops-tokei); this is the ops-deps instance.

**Why it matters**: `ops_deps::UpgradeEntry` and friends are the payload other extensions and the data-cache decode against, and `DepsReport` is `#[non_exhaustive]` and still growing. A consumer reading the rustdoc cannot tell which fields are cargo-edit verbatim strings, which are derived, or that `note` drives categorisation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 every public item and public field re-exported from the ops-deps crate root has a doc summary
- [ ] #2 UpgradeEntry::note documents that categorize_upgrades classifies an entry as breaking when the note contains 'incompatible' (case-insensitive)
- [ ] #3 DepsProvider carries a doc summary naming the two cargo subcommands it shells out to
- [ ] #4 categorize_upgrades states its classification rule in its summary
<!-- AC:END -->
