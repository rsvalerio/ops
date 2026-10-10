---
id: TASK-2458
title: 'READ-13: doc comments narrate rejected alternatives and past bugs rather than current behavior'
status: To Do
assignee: []
created_date: '2026-10-10 15:25'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2621'
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-terraform/about/src/lib.rs:is_heredoc_ident_start'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:634`, `extensions-terraform/about/src/lib.rs:1904`, `extensions-terraform/about/src/lib.rs:2376`

**What**: Three doc comments describe the journey that produced the code instead of the end state:

1. `is_heredoc_ident_start` (lines 634-645): paragraphs 2-3 are a "why we picked X over Y" essay — they narrate why `char::is_alphabetic` / `is_alphanumeric` are "not a workable stand-in", walk through the failure story of the approach not chosen ("A decomcomposed `<<é` would then have its terminator truncated, the real closing line would never match, and the rest of the file would be swallowed as heredoc body"), and justify the `unicode-ident` choice against HCL's exact `ID_*` tables. The enduring fact (terminators are UAX #31 identifiers, matched via `unicode-ident`) is one sentence; the rest is design-journal.
2. Test doc for `extract_required_version_after_a_decomposed_unicode_heredoc_terminator` (lines 1904-1908): "`ID_Continue` admits `U+0301` ..., but `char::is_alphanumeric` does not: it truncated `<<e\u{301}` to `e`, so the real closing line never matched and the rest of the file was eaten as heredoc body" — narrates the pre-fix behavior of an implementation no longer present.
3. Test doc for `count_local_modules_follows_symlinked_module_dirs` (lines 2376-2381): "`DirEntry::file_type` does not follow symlinks, so such an entry reported `Symlink` and was dropped from the count; `fs::metadata` resolves it" — narrates the past bug and its fix rather than what the test pins now.

**Why it matters**: READ-13: process narration in `///` docs goes stale on the next change while looking authoritative, and a reader must separate the current invariant from the history of the bug that motivated it. Sibling crates have already had this exact cleanup (e.g. TASK-2426 for ops-about-rust, and the docs(theme)/docs(sqlite) narration strips).

<!-- scan confidence: 3 doc-comment sites verified by reading; the rest of the crate's docs state current invariants and are fine -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Doc comments at the three listed sites state the current behavior or the invariant the test pins, without rejected-alternative essays or pre-fix/pre-change narration
- [ ] #2 No other doc comment in the crate narrates past bugs, prior implementations, or design-selection history
<!-- AC:END -->
