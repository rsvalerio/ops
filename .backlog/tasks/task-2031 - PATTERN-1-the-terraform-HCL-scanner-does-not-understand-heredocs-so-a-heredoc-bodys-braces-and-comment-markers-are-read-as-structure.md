---
id: TASK-2031
title: >-
  PATTERN-1: the terraform HCL scanner does not understand heredocs, so a
  heredoc body's braces and comment markers are read as structure
status: Triage
assignee: []
created_date: '2026-08-28 21:26'
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
**File**: `extensions-terraform/about/src/lib.rs` (`strip_comments`, `scan_line`)

**What**: HCL heredocs (`<<EOT` / `<<-EOT` … `EOT`) are unquoted multi-line
string literals, but neither the comment stripper nor the per-line brace
scanner tracks them. Everything inside a heredoc body is therefore read as
live HCL:

```hcl
locals {
  script = <<-EOT
    # not an HCL comment - a shell comment inside a string value
    if [ -f x ]; then echo "}" ; fi
  EOT
}
```

- `strip_comments` blanks the `#` line as if it were an HCL comment (harmless
  today, since heredoc *values* are never read, but it silently corrupts the
  content the scanner sees).
- `scan_line` counts the bare `}` on the `if` line as a structural close. With
  the TASK-1765 balance fix in place, that pops one level too many, and if the
  pop empties the stack the file is now refused outright (`LineScan::Malformed`
  → `None`) rather than mis-tracked.

**Why it matters**: a `terraform { … required_version … }` block declared
*after* such a heredoc loses its constraint entirely, and the About card shows
no `Terraform <version>` line. Heredocs are common in `locals` blocks feeding
`user_data`, `templatefile` inputs and policy documents, and they routinely
contain braces and `#` lines. Pre-TASK-1765 the same input mis-tracked depth
without refusing the file, so the failure mode changed shape rather than
appearing from nothing — but the input class is real and untested.

**Fix direction**: recognise `<<[-]?<IDENT>` outside a string in
`strip_comments` and pass the body through verbatim until a line whose trimmed
content is the terminator, marking those lines so `scan_line` skips them
(a per-line "inside heredoc" flag threaded alongside the block stack, or a
pre-pass that blanks heredoc bodies the way block comments are blanked).

**Origin**: discovered during TASK-2001 while fixing TASK-1765 / TASK-1771.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 strip_comments leaves heredoc bodies untouched - a # or /* inside a heredoc is not treated as an HCL comment
- [ ] #2 scan_line does not count braces that appear inside a heredoc body as structural
- [ ] #3 extract_required_version returns the constraint for a terraform block declared after a locals heredoc containing an unbalanced brace
- [ ] #4 Regression tests cover a heredoc containing a bare } and a heredoc containing a # line
<!-- AC:END -->
