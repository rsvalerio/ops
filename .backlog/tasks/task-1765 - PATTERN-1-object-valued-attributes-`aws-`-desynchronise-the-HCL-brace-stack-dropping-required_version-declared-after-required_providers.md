---
id: TASK-1765
title: >-
  PATTERN-1: object-valued attributes (`aws = {`) desynchronise the HCL brace
  stack, dropping required_version declared after required_providers
status: To Do
assignee:
  - TASK-2001
created_date: '2026-08-27 11:20'
updated_date: '2026-08-28 14:14'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:186-222` (`extract_required_version`), with `block_open_ident` at `:264-300`

**What**: `extract_required_version` tracks block depth with a `Vec<String>` that is pushed only when `block_open_ident` recognises a line as a block opener, and popped on any line starting with `}`. `block_open_ident` deliberately rejects lines containing `=` before the trailing `{` (`:291-298`), so an object-valued *attribute* such as `aws = {` opens a real brace that is never pushed — but its closing `}` is still popped. Every multi-line object literal therefore pops one level too many, and the enclosing `terraform` block is silently closed early.

The canonical terraform block shape hits this:

```hcl
terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
  required_version = ">= 1.5"
}
```

Verified by running the crate's own `extract_required_version` against that input: it returns `None`. Same for a `cloud { workspaces = { name = "x" } }` block followed by `required_version`. Reordering so `required_version` comes first happens to work, so the bug is invisible in half of real configs and in every existing test.

**Why it matters**: `required_providers` with an object-literal per provider is the shape the terraform docs and `terraform init` scaffolding produce, and declaring `required_version` after it is entirely conventional. For those projects the About card silently loses the whole `Terraform <version>` stack detail — an incorrect-by-omission render with no warning, which is exactly the class of silent drift ERR-2/TASK-0919 introduced the block stack to prevent. The failure is also order-dependent, so it looks like flakiness rather than a parse bug when reported.

**Fix direction**: count braces rather than assuming one structural token per line — push a sentinel (e.g. `None`/`""` for "not a named block") whenever a line opens a brace that is not a named block opener, so pops stay balanced, and treat only `[Some("terraform")]`-shaped stacks as depth-1-inside-terraform. A `}` on an empty stack should be treated as malformed input rather than a silent no-op.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extract_required_version returns Some(">= 1.5") for a terraform block where required_providers with object-literal providers precedes required_version
- [ ] #2 Brace tracking stays balanced for object-valued attributes (key = { ... }) at any nesting depth, including cloud/workspaces and default = { ... } shapes
- [ ] #3 A regression test covers required_version declared both before and after a multi-line required_providers block
<!-- AC:END -->
