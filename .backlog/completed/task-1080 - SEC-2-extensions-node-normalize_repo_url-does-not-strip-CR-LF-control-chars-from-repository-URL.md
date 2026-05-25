---
id: TASK-1080
title: >-
  SEC-2: extensions-node normalize_repo_url does not strip CR/LF/control chars
  from repository URL
status: Done
assignee: []
created_date: '2026-05-07 21:20'
updated_date: '2026-05-08 06:24'
labels:
  - code-review-rust
  - SEC
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/repo_url.rs:16-43`

**What**: `normalize_repo_url` only `trim()`s outer whitespace and rewrites prefixes; it never rejects/escapes embedded `\n`, `\r`, or ANSI/control bytes inside the URL body. An adversarial `package.json` `repository.url` like `"github:owner/repo\nINJECT"` flows verbatim into the About card, markdown, HTML, and log lines.

**Why it matters**: Sister site to the SEC-14 traversal fix (TASK-0811) and the ERR-7 path-debug-escape pattern applied across the codebase. Repository URLs render into operator-facing surfaces; they belong in the same control-char-sanitisation policy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 URLs with embedded \n / \r / \u{1b} either drop or escape the offending segment
- [x] #2 Unit tests pin the behaviour for both Text and Object{url} repository shapes
- [x] #3 Downstream debug-log of the URL stays single-line
<!-- AC:END -->
