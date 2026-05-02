---
id: TASK-0853
title: >-
  SEC-11: extract_required_version quote-strip is brittle; ignores inline #///
  comments
status: Triage
assignee: []
created_date: '2026-05-02 09:17'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:99-119`

**What**: `let v = rest.trim_matches(\'"\').trim();` - trim_matches only strips outer double quotes. A line like `required_version = ">= 1.5" # patch needed` is taken verbatim including the trailing comment; a single-quoted or unquoted value is mis-handled. There is also no length cap on v despite being rendered into the About card.

**Why it matters**: Operators see misleading version strings ("\">= 1.5\" # patch needed") on otherwise valid HCL. Also, .tf parser leniency invites the same drift as extensions-go go.mod parser, which already strips trailing // comments.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Strip trailing # and // line comments before quote-stripping (mirroring go_mod::strip_line_comment)
- [ ] #2 Reject the value if not double-quoted (HCL standard); add a unit test for required_version = >= 1.5 # comment
- [ ] #3 Cap the rendered length (e.g., 64 chars) and log truncation, per SEC-11/SEC-33
<!-- AC:END -->
