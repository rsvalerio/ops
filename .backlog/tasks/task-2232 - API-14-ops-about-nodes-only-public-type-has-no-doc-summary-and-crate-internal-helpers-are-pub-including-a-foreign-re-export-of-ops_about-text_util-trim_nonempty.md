---
id: TASK-2232
title: 'API-14: ops-about-node''s only public type has no doc summary and crate-internal helpers are pub, including a foreign re-export of ops_about::text_util::trim_nonempty'
status: Done
assignee: []
created_date: '2026-09-08 07:24'
updated_date: '2026-09-10 19:03'
labels:
  - code-review-rust
  - api-design
dependencies: []
parent_task_id: 'TASK-2247'
modified_files:
  - extensions-node/about/src/lib.rs
  - extensions-node/about/src/package_json.rs
  - extensions-node/about/src/repo_url.rs
  - extensions-node/about/src/units.rs
priority: low
ordinal: 138000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/lib.rs:39`, `extensions-node/about/src/package_json.rs:17`, `extensions-node/about/src/repo_url.rs:182`, `extensions-node/about/src/units.rs:23`

**What**:

- `pub struct AboutNodeExtension` (`lib.rs:39`) — the crate's only genuinely
  public item — carries `#[non_exhaustive]` but no doc summary.
- Every module in the crate is private, yet their items are declared `pub`
  rather than `pub(crate)`, overstating the reachable surface and defeating
  dead-code detection: `PackageJson` and all nine of its fields
  (`package_json.rs:17`), `parse_package_json` (`package_json.rs:85`),
  `normalize_repo_url`, `ssh_to_https`, `append_tree_directory`,
  `is_numeric_port_prefix` (`repo_url.rs:71,182,211,269`), `PROVIDER_NAME`
  and `NodeUnitsProvider` (`units.rs:23,25`).
- `package_json.rs:166` re-exports a foreign crate's item —
  `pub use ops_about::text_util::trim_nonempty;` — purely to shorten local
  call sites. A `use` (no `pub`) does the same job without adding another
  path to a symbol that already has a canonical home in `ops_about`.
- Where doc comments exist they are rationale, not summaries: the doc comment
  on `PackageJson` (`package_json.rs:9-16`) describes an attribute that was
  *removed*, and says nothing about what the type is.

**Why it matters**: `pub` on an unreachable item is a lie the compiler cannot
check, and it suppresses the unused-item warnings that would otherwise flag
helpers no caller kept. The foreign re-export gives one function two import
paths for no benefit.

**Twins** (same class, other stacks — cross-reference, do not merge):
TASK-2187 (about-go), TASK-2163 (about-rust), TASK-2071 (about), and TASK-2072
for the foreign re-export half.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 AboutNodeExtension has a doc summary describing what the extension provides
- [x] #2 items in the private package_json, package_manager, repo_url and units modules are pub(crate) unless genuinely reachable from outside the crate
- [x] #3 the pub use of ops_about::text_util::trim_nonempty is a plain use, or call sites reference the canonical path
- [x] #4 cargo build reports no newly-unused items after the visibility narrowing, or any it reports are removed

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC2 substituted: pub(crate) inside the private modules is denied by clippy::redundant_pub_crate (workspace convention spells such items pub; see ops-cargo-toml/src/lib.rs). AC3 satisfied literally: the pub use of ops_about::text_util::trim_nonempty is now a plain use. AC4: no newly-unused items after the visibility work (workspace clippy clean).

AC #2 audit follow-up: substitution unchanged (all four modules in extensions-node/about/src are private, so the `pub` spelling inside them is crate-internal already). The decision was previously recorded only in package_json.rs; a crate-level note was added above the module block in extensions-node/about/src/lib.rs so repo_url.rs, units.rs and package_manager.rs are covered by the same explanation.

<!-- SECTION:NOTES:END -->
