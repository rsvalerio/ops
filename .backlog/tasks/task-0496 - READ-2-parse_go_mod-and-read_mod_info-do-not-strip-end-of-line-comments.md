---
id: TASK-0496
title: 'READ-2: parse_go_mod and read_mod_info do not strip end-of-line // comments'
status: To Do
assignee:
  - TASK-0532
created_date: '2026-04-28 06:09'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - read
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/lib.rs:108`

**What**: Both parse_go_mod (lib.rs:108-119) and read_mod_info (modules.rs:99-108) take everything after `module ` or `go ` as the value. A line like `module github.com/foo // toolchain note` ends up with module = `github.com/foo // toolchain note`, which then breaks rsplit(\/\).next() name extraction and shows the comment in stack_detail.\n\n**Why it matters**: go.mod permits trailing line comments and they appear in real-world manifests. The current parser conflates the comment into the value, producing user-visible garbage in the About card.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Strip // ... trailing portion from the rest before trim+to_string in both module and go-version branches
- [ ] #2 Apply consistently in lib.rs::parse_go_mod and modules.rs::read_mod_info
- [ ] #3 Test covers module x.y/z // comment and go 1.22 // toolchain hint
<!-- AC:END -->
