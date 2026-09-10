---
id: TASK-2098
title: 'API-14: undocumented public items across the ops-extension surface'
status: Done
assignee: []
created_date: '2026-09-07 22:59'
updated_date: '2026-09-09 19:00'
labels:
  - code-review-rust
  - structure
dependencies: []
parent_task_id: 'TASK-2247'
modified_files:
  - crates/extension/src/data.rs
  - crates/extension/src/extension.rs
priority: low
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:452`, `crates/extension/src/data.rs:718`, `crates/extension/src/extension.rs:80-86`, `crates/extension/src/extension.rs:172-175`, `crates/extension/src/extension.rs:312`, `crates/extension/src/extension.rs:350`

**What**: <!-- scan confidence: verified per item, complete list --> Public items with no doc summary:

- `DataRegistry::get` (data.rs:452) — no doc at all
- `Context::new` (data.rs:718) — no doc (its sibling `from_cwd_arc` has one)
- `ExtensionType` bitflags type (extension.rs:81-86) — no type-level doc
- `CommandRegistry::new` (extension.rs:172-175) — no doc
- `Extension::name` (extension.rs:312) — the trait's core identifying method, no doc
- `Extension::register_commands` (extension.rs:350) — no doc; this is the method whose duplicate-registration contract (audit trail, last-write-wins) is documented on `CommandRegistry::insert` but not at the trait where implementers arrive

**Why it matters**: API-14 — every public item carries a summary doc. The rest of this crate is densely documented (often to excess), so these gaps are the inconsistent tail; `Extension::register_commands` in particular is the entry point every extension implementer codes against.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each listed item has at least a one-line summary doc
- [x] #2 Extension::register_commands and register_data_providers docs link the duplicate-registration policy (registry_duplicate_policy) so implementers see the audit-trail contract at the trait
- [x] #3 cargo doc -p ops-extension renders the items with summaries; no new broken intra-doc links (workspace rustdoc deny)

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC #3: cargo doc -p ops-extension now clean both with and without the duckdb feature. Two pre-existing broken intra-doc links (Context::db, Context::attach_db — cfg-gated methods referenced unconditionally in the Context struct doc) were de-linked in passing; they failed the workspace rustdoc broken-link deny on a no-default-features doc build.
<!-- SECTION:NOTES:END -->
