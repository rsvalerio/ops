---
id: TASK-1165
title: >-
  SEC-2: Control-char stripping in repository URLs silently concatenates
  adjacent text
status: Done
assignee:
  - TASK-1259
created_date: '2026-05-08 07:45'
updated_date: '2026-05-08 13:28'
labels:
  - code-review-rust
  - SEC
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/repo_url.rs:18-22`

**What**: `strip_control_chars` removes CR/LF/ESC/DEL by `filter`, so an adversarial `\"github:owner/repo\\nINJECT\"` becomes `github:owner/repoINJECT`, which then normalises to `https://github.com/owner/repoINJECT` — a clickable URL pointing at an attacker-named repo. Tests at lines 358-397 explicitly pin this concatenating behaviour.

**Why it matters**: The defence against log-injection succeeds (no embedded newlines reach logs/cards), but the resulting URL is now an attacker-chosen repository link rendered as legitimate metadata in the About card / markdown / HTML. A user clicking the homepage/repository hyperlink lands on the attacker's repo rather than a clearly broken value.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 When repository (Text or Object{url}) contains any control byte, either drop the field or replace each stripped char with a single non-URL sentinel that obviously breaks the URL
- [x] #2 Existing log-injection tests still pass (no \n/\r/\u{1b} in output)
- [x] #3 New test: a repository containing a control byte does NOT produce a syntactically valid URL pointing at attacker-chosen path segments
<!-- AC:END -->
