---
id: TASK-1020
title: >-
  PATTERN-1: extract_required_version misses required_version inside HCL block
  comments and accepts it after one
status: Done
assignee: []
created_date: '2026-05-07 20:22'
updated_date: '2026-05-07 23:15'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:159-228`

**What**: `extract_required_version` skips line comments (`#`, `//`) but does not recognise HCL block comments (`/* … */`). The HCL spec includes block comments, and Terraform formatters emit them in real configs. Two consequences:

1. A real `required_version = "~> 1.5"` wrapped in `/* … */` is still extracted — block-commented declarations are silently used. Mirrors the bug TASK-0846 fixed for Maven `<!-- … -->` wrapping `<artifactId>`.
2. Conversely, a block comment that legitimately wraps body lines (e.g. `terraform { /* TODO bump\n   required_version = ">= 99" */ required_version = "~> 1.5" }`) leaves the parser confused about block depth, since `block_open_ident` and the `}` pop logic do not understand `/*` / `*/`.

The block-stack tracking added by ERR-2 / TASK-0919 is otherwise correct for line-comment-only files; this is the next layer down.

**Why it matters**: The About card exists to communicate ground truth from the manifest. A `required_version` that lives inside a multi-line `/* ... */` block is by definition not the active terraform constraint, but the parser surfaces it anyway — symmetric with the CL-3 / TASK-0846 Maven fix. Low priority because real terraform configs use `#`/`//` overwhelmingly more than `/* */`, but the contract gap is the same shape as the Maven one.

Concrete repro for case (1):
```
terraform {
  /* required_version = ">= 99.0" */
}
```
extract_required_version returns `Some(">= 99.0")`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Strip /* ... */ block comments before block-stack tracking, mirroring strip_xml_comments in maven/pom.rs
- [ ] #2 Add tests for required_version inside a block comment (rejected) and around a block comment (accepted)
<!-- AC:END -->
