---
id: TASK-2116
title: >-
  DUP-2: control / bidi codepoint rejection policy is reimplemented in ops-git
  and diverges from the shared About helper
status: Done
assignee:
  - TASK-2236
created_date: '2026-09-08 06:54'
updated_date: '2026-09-08 15:46'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions/git/src/config.rs
  - extensions/about/src/text_util.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:79-115` (`is_ascii_control_byte`, `is_unicode_format_or_separator`), vs `extensions/about/src/text_util.rs:66-68` (`contains_control_chars`), `crates/core/src/table.rs:155-165` and `crates/core/src/ui.rs:88-91`

**What**: The "untrusted text must not carry control / bidi / zero-width codepoints into operator-facing surfaces" policy exists in at least four independent implementations with different strictness:

- `ops-git`: `char::is_control` **plus** an explicit list (U+200B/C/D, U+2060, U+FEFF, U+200E/F, U+202A..U+202E, U+2066..U+2069, U+2028/9) — the strictest.
- `ops-about::text_util::contains_control_chars`: `raw.chars().any(char::is_control)` only — no bidi, no zero-width, no BOM.
- `ops-core::table` / `ops-core::ui`: yet another range set, applied by escaping rather than rejecting.

The ops-git doc comments explicitly say they "mirror" the node/python About implementations, i.e. the duplication is known and has already been copied further.

**Why it matters**: The strictest copy is the one on the git remote path; the weakest copy (`contains_control_chars`) guards About-card manifest fields rendered in the *same* card. A U+202E in a manifest field is accepted where the identical codepoint in a git remote is rejected, so the hardening filed under SEC-2 / TASK-1238 is only partially in force on the surface it was filed to protect. Duplicated security predicates also drift silently: the next codepoint added to one list will not reach the others.

<!-- Reviewer note: promote a single `ops-core` predicate (e.g. `ops_core::text::is_unsafe_display_char`) and have ops-git, ops-about, and the node/python/terraform About crates call it, keeping the drop-vs-escape decision at each call site. -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 One shared predicate defines the rejected control / formatting / separator codepoint set
- [ ] #2 ops-git and ops-about consume that predicate instead of local copies; behaviour on the git path is unchanged (existing SEC-2 tests still pass)
- [ ] #3 About-card manifest fields reject the same bidi / zero-width codepoints the git remote path rejects, with a test pinning U+202E and U+200B
<!-- AC:END -->
