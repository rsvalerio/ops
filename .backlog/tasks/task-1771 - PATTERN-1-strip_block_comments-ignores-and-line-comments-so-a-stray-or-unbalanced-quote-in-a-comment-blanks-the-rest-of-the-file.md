---
id: TASK-1771
title: >-
  PATTERN-1: strip_block_comments ignores # and // line comments, so a stray /*
  or unbalanced quote in a comment blanks the rest of the file
status: Done
assignee:
  - TASK-2001
created_date: '2026-08-27 11:21'
updated_date: '2026-08-28 21:24'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:309-360` (`strip_block_comments`)

**What**: The stripper is a two-state machine (`in_string` / not) that knows about `"` and `/* … */` but has no notion of `#` or `//` line comments. Anything inside a line comment is therefore interpreted as live HCL:

```hcl
terraform {
  # see https://example.com/*note
  required_version = "~> 1.5"
}
```

The `/*` inside the comment opens a block comment that, per the documented "unterminated `/*` runs to EOF" behaviour (`:307-308`), blanks the remainder of the file — `extract_required_version` returns `None`. Verified against the crate's own code.

The same hole applies to quotes: an unbalanced `"` inside a `#` comment (`# don't use "old style`) flips `in_string` for everything that follows, disabling block-comment stripping and quote tracking for the rest of the file. That case happens to still return a value in the example tested, but the state machine is desynchronised from that point on and any subsequent `/* … */` is no longer stripped.

**Why it matters**: This is a silent wrong answer driven by the *contents of a comment* — the least suspicious place in a config file. A URL with a glob (`s3://bucket/*`), a shell snippet, or an apostrophe-free-but-quote-containing note is enough. Unlike the block-comment feature it implements (PATTERN-1 / TASK-1020), the failure mode is not "stale declaration surfaces" but "the whole file goes dark", which is harder to notice and impossible to attribute from the About card. Terraform's own lexer resolves `#`/`//` before `/*`, and the sibling helper `ops_about`/`go_syntax::strip_line_comment` already encodes the whitespace-or-SOL rule this scanner needs.

**Fix direction**: handle all three comment forms in the single pass — on encountering `#` or `//` outside a string, blank to end-of-line (preserving the newline) and continue, so their contents can never open a block comment or toggle string state.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A /* sequence inside a # or // line comment does not start a block comment
- [x] #2 An unbalanced double quote inside a line comment does not put the scanner into string state
- [x] #3 Existing behaviour is preserved: /* inside a quoted value stays literal, and a block-commented required_version is still ignored
- [x] #4 Regression tests cover a comment containing /* and a comment containing an unbalanced quote
<!-- AC:END -->
