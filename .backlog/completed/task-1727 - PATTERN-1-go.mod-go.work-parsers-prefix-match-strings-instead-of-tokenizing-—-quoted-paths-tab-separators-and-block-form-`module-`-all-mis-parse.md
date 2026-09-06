---
id: TASK-1727
title: >-
  PATTERN-1: go.mod/go.work parsers prefix-match strings instead of tokenizing —
  quoted paths, tab separators and block-form `module (` all mis-parse
status: Done
assignee:
  - TASK-1989
created_date: '2026-08-27 11:11'
updated_date: '2026-08-28 15:29'
labels:
  - code-review-rust
  - correctness
dependencies: []
modified_files:
  - extensions-go/about/src/go_mod.rs
  - extensions-go/about/src/go_work.rs
  - extensions-go/about/src/go_syntax.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_mod.rs:27-63` (`parse`), `extensions-go/about/src/go_mod.rs:65-108` (`parse_replace_directive`), `extensions-go/about/src/go_work.rs:24-58` (`parse_use_dirs`)

**What**: Both parsers recognise directives with `str::strip_prefix("module ")`
/ `"go "` / `"replace "` / `"use "` and then treat the remainder as a raw
path. The go.mod / go.work grammar (`golang.org/x/mod/modfile`) is a token
grammar, not a line-prefix grammar, so three legal shapes mis-parse today:

1. **Quoted tokens are never unquoted.** modfile lexes Go-style quoted
   strings, and quoting is in fact *required* for any token containing a
   space:
   - `module "example.com/m"` → `out.module = Some("\"example.com/m\"")`;
     `modules::last_segment` then yields `m"` and the About card renders a
     module named `m"`.
   - `use "./api"` → the directive string keeps its quotes, so
     `normalize_module_path` cannot strip the `./`, `ProjectUnit::path`
     becomes `"./api"` (quotes included), and it matches no `tokei_files`
     row — the module reports zero LOC.
   - `replace ex.com/m => "./has space/sub"` → the target starts with `"`,
     so none of the `starts_with("./" | "../" | ...)` arms in
     `parse_replace_directive` match and the local replace is **dropped**,
     under-counting modules in `compute_module_count`.

   Note the irony: `parse_replace_directive` carries a dedicated
   TASK-0815 test (`accepts_local_replace_target_with_whitespace`) pinning
   the *unquoted* `./has space/sub` shape, which cmd/go would reject —
   while the quoted shape cmd/go actually produces is the one that fails.

2. **Only a single ASCII space separates verb from argument.** modfile
   splits on arbitrary whitespace, so `module\texample.com/m`,
   `go\t1.22`, and `use\t./api` are legal. `strip_prefix("module ")`
   returns `None` for all of them, so the module name silently drops to
   `None` and lib.rs falls back to the directory name; the Go version
   disappears from the card.

3. **Block-form `module (` sets the module name to the literal `"("`.**
   modfile parses every verb in block form, including
   `module (\n\texample.com/m\n)`. Trace through `go_mod.rs`:
   `is_block_opener(line, "replace")` is false, then
   `line.strip_prefix("module ")` on `"module ("` yields `Some("(")`,
   whose trim is non-empty — so `out.module = Some("(")`. The About card
   shows a project named `(` instead of falling back to the directory
   name. The same happens for `go (`.

**Why it matters**: each shape produces silently wrong About output — a
mangled project name, a missing Go version, an under- or over-counted
module list, or a module whose LOC row never joins. There is no diagnostic
in any of these paths; the user sees plausible-looking but incorrect data.
Cases 1 and 3 are also the kind of input an adversarial or merely
tool-generated repository produces without any intent.

<!-- scan confidence: candidates to inspect — each shape traced by hand against the current code; no fixture exists for any of them -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A shared tokenizer helper in `go_syntax.rs` splits a modfile line into verb + arguments on arbitrary whitespace and unquotes Go-style quoted tokens; both parsers use it instead of `strip_prefix("<verb> ")`
- [x] #2 `module "example.com/m"` yields module `example.com/m` (no quotes) and the About card name `m`
- [x] #3 `use "./api"` yields the use dir `./api`, and `replace ex.com/m => "./has space/sub"` is retained in `local_replaces` as `./has space/sub`
- [x] #4 Tab-separated verbs (`module\texample.com/m`, `go\t1.22`, `use\t./api`) parse identically to their space-separated forms
- [x] #5 Block-form `module (\n\texample.com/m\n)` yields module `example.com/m`, and never the literal `(`; the same for `go (`
- [x] #6 Existing go_mod and go_work tests still pass, including the embedded-`//` (TASK-1107) and inline-comment block-opener (TASK-1255) cases
<!-- AC:END -->
