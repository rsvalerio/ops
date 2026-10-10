---
id: TASK-2470
title: 'FN-1: match_section_open exceeds 50 lines with an inlined token-scanning loop inside a classifier'
status: To Do
assignee: []
created_date: '2026-10-10 15:28'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - fn
dependencies: []
parent_task_id: 'TASK-2621'
modified_files:
  - extensions-java/about/src/maven/pom.rs
priority: low
ordinal: 1000
dedup_key: 'FN-1:extensions-java/about/src/maven/pom.rs:match_section_open'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:381`

**What**: `match_section_open` (lines 381-462, ~82 lines) is a section-opener classifier, but the single-line `<developers>...</developers>` branch embeds an 18-line token-scanning while-loop (lines 426-448: find `<developer>`, slice to `</developer>`, extract `<name>`, advance `rest`) directly inside the classification chain, alongside the scm/licenses collapsed-form branches (lines 401-418).

**Why it matters**: FN-1 asks that a function operate at a single abstraction level: this one mixes "which section shape is this line?" (classification) with "walk the developer entries in a collapsed container" (lexical scanning). Extracting the developers walk into a named helper (e.g. `extract_collapsed_developers(line, &mut data)`) mirrors how the scm/licenses branches delegate to `try_set_once`, shrinks the classifier toward the 50-line threshold, and makes the developers branch testable in isolation. Note for triage: an exhaustive classifier is a recognized FN-1 exception; the mixed-abstraction inner loop is the concrete extraction target.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The collapsed-developers scanning loop is extracted to a named helper so match_section_open classifies section shapes at one abstraction level
- [ ] #2 Function body is <=50 lines after extraction, or the residual classifier length is documented as a justified exhaustive-match exception
<!-- AC:END -->
