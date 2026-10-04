---
id: TASK-2367
title: 'SEC-11: content_declares_workspace treats `"""` inside a comment as a multi-line string opener'
status: To Do
assignee: []
created_date: '2026-10-04 14:11'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - security
dependencies: []
parent_task_id: 'TASK-2417'
modified_files:
  - extensions-rust/cargo-toml/src/workspace_root.rs
priority: low
ordinal: 1000
dedup_key: 'SEC-11:extensions-rust/cargo-toml/src/workspace_root.rs:content_declares_workspace'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/workspace_root.rs:content_declares_workspace` (the `!trimmed.starts_with('[')` branch, ~line 455-470)

**What**: The multi-line-string tracker counts `"""` / `'''` occurrences on every non-header line, including pure comment lines (`# see """docs`) and trailing comments (`a = 1 # '''`). An odd count flips `in_multiline_string` on, and every following line (including a real `[workspace]` header) is skipped until some later line contains the delimiter. No test covers a comment carrying triple quotes (existing tests: `content_declares_workspace_ignores_literal_multiline_string`, `..._accepts_trailing_comment`).

**Why it matters**: A false negative makes the ancestor walk climb past the real workspace root into attacker-plantable ancestors; the function's own doc comment calls this security-relevant (SEC-11 / SEC-25 threat model). Also yields wrong root for benign manifests whose comments mention triple quotes.

<!-- scan confidence: single site, confirmed by reading; not executed -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Comment text (full-line and trailing, outside strings) is ignored when counting triple-quote delimiters
- [ ] #2 Regression tests: a comment with an odd number of `"""` / `'''` before `[workspace]` still detects the workspace
<!-- AC:END -->
