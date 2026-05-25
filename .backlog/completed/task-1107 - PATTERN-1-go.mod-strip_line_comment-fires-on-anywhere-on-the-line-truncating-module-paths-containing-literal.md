---
id: TASK-1107
title: >-
  PATTERN-1: go.mod strip_line_comment fires on '//' anywhere on the line,
  truncating module paths containing literal '//'
status: Done
assignee: []
created_date: '2026-05-07 21:34'
updated_date: '2026-05-08 06:28'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_mod.rs:58-63`

**What**: The helper is used by both `go.mod` and `go.work` parsers and treats the first `//` as a line-comment delimiter. Go's own `cmd/go` lexer is whitespace-sensitive — `//` is a comment only when it follows whitespace or starts the line. A `module example.com/foo//bar` line (which `cmd/go` accepts as a module path) is silently truncated to `example.com/foo`. A `replace` target like `./has//double-slash` similarly suffers if it reaches the comment stripper.

**Why it matters**: Single-character difference between this lexer and Go's own — module path drift flips identity-card values silently.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 strip_line_comment only fires on // preceded by whitespace or at start-of-line
- [x] #2 Regression test: a module line 'module example.com/foo//bar\n' round-trips to 'example.com/foo//bar', not 'example.com/foo'
- [x] #3 Apply the fix in one place since both go.mod and go.work share the helper
<!-- AC:END -->
