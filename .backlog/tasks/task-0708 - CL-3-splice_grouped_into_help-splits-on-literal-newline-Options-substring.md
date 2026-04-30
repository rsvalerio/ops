---
id: TASK-0708
title: 'CL-3: splice_grouped_into_help splits on literal newline-Options substring'
status: To Do
assignee:
  - TASK-0742
created_date: '2026-04-30 05:29'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/help.rs:200-211`

**What**: `splice_grouped_into_help` locates the insertion point with `help_str.find("\nOptions:")`. clap renders the section heading as `Options:` at column 0, so the literal match works for the expected case, but any subcommand `about` text or argument help that contains the substring `Options:` (e.g. `Override CLI options:`) would match first and the categorized command list would be inserted into the middle of an unrelated help line. There is no anchoring (start of section, no leading whitespace).

**Why it matters**: A future help-text edit can silently corrupt `ops --help` output. The current test suite passes because no built-in subcommand description currently contains the substring; this is convention-only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Match the Options heading on a line boundary or split on the unique two-newline form so an embedded substring in subcommand help cannot win
- [ ] #2 Add a regression test where a subcommand description contains the substring Options: and assert the grouped section is still spliced before the real Options block
<!-- AC:END -->
