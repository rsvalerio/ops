---
id: TASK-1796
title: >-
  PATTERN-1: count_local_modules requires main.tf, undercounting local modules
  that use any other .tf entry file
status: Triage
assignee: []
created_date: '2026-08-27 11:24'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:388-422` (`count_local_modules`), specifically the filter at `:415`

**What**: A subdirectory of `modules/` is counted only if it contains a file named exactly `main.tf`:

```rust
.filter(|e| e.path().join("main.tf").exists())
```

Terraform's own definition is broader: a module is any directory containing `.tf` files — the filenames are conventions, not requirements. Layouts that this filter reports as zero modules:

- `modules/network/network.tf` (entry file named after the module — common in generated and vendored stacks)
- `modules/vpc/{variables.tf,outputs.tf,resources.tf}` with no `main.tf`
- `modules/foo/main.tf.json` (the JSON syntax terraform accepts natively)

Because the function returns `None` when the count is zero (`:417-421`), a project whose modules all use another entry filename renders no `modules` line at all rather than an undercount — indistinguishable from a project with no local modules.

**Why it matters**: The `modules` count is one of five facts the About card states about the project, and it is wrong-by-omission for a legitimate and reasonably common layout, with no warning. The existing tests (`count_local_modules_counts_module_dirs`) encode the `main.tf` assumption, so the gap is invisible from the suite. The `.exists()` probe is also one syscall per subdirectory purely to answer "is this a module", where a single `read_dir` of the subdirectory answers the broader question directly.

**Fix direction**: count a subdirectory as a module when it contains at least one `*.tf` (extension compared ASCII-case-insensitively, matching the treatment `find_required_version` already gives `.TF` at `:131-135`); consider `.tf.json` as well. Update the doc comment at `:380`, which currently documents the `modules/*/main.tf` rule.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A modules/<name>/ directory containing any .tf file is counted, not only one containing main.tf
- [ ] #2 The .tf extension check is ASCII-case-insensitive, consistent with find_required_version
- [ ] #3 Directories with no .tf file at all are still excluded, and the existing count_local_modules tests still pass
- [ ] #4 The function doc comment no longer claims the modules/*/main.tf rule
<!-- AC:END -->
