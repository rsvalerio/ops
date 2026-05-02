---
id: TASK-0869
title: 'DUP-1: Maven parse_top_level repeats five identical try_set_once blocks'
status: Triage
assignee: []
created_date: '2026-05-02 09:22'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:286-312`

**What**: Five sequential blocks of identical structure differing only in the &mut Option<String> field and the tag name. A small helper try_set_once(field: &mut Option<String>, line, open, close) removes the boilerplate and makes the "first writer wins on duplicates" invariant explicit.

**Why it matters**: The <scm><url> precedence rule (line 198-203 in handle_scm and 307-311 in parse_top_level) interacts with this guard - if anyone removes the is_none() check during a refactor, top-level <url> would clobber <scm><url>. Encoding the rule in a single helper makes the invariant survive future edits.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A try_set_once (or set_if_none) helper replaces the five blocks in parse_top_level and the matching block in handle_scm
- [ ] #2 A regression test pins later top-level <url> does not override earlier <scm><url> (already exists as parse_pom_scm_takes_precedence_over_url)
- [ ] #3 File line count drops; no behaviour change
<!-- AC:END -->
