---
id: TASK-1724
title: >-
  PATTERN-1: go.work block terminator `)` with a trailing comment is swallowed
  as a use directive and leaves the block open
status: Triage
assignee: []
created_date: '2026-08-27 11:11'
labels:
  - code-review-rust
  - correctness
dependencies: []
modified_files:
  - extensions-go/about/src/go_work.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_work.rs:24-58` (`parse_use_dirs`)

**What**: `parse_use_dirs` performs its structural checks on the *raw*
trimmed line and only strips the line comment afterwards:

```rust
let line = raw.trim();                       // comment still attached
...
if in_use_block {
    if line == ")" { in_use_block = false; continue; }   // exact match
    ...
    let stripped = strip_line_comment(line).trim();      // stripped too late
    if !stripped.is_empty() { dirs.push(stripped.to_string()); }
}
```

cmd/go accepts a trailing line comment after the block terminator, so this
is legal `go.work`:

```
go 1.21

use (
	./api
	./cmd
) // workspace members

go 1.22
```

Trace: `line` is `") // workspace members"`, which is not equal to `")"`,
is not empty, does not start with `//`, and is not a `use(` opener. Control
falls through to `strip_line_comment(...)` → `")"`, which is non-empty and
is therefore **pushed as a use directive**. `in_use_block` stays `true`, so
every subsequent line in the file (`go 1.22`, a `replace` block, anything)
is also absorbed as a directive.

Result: `parse_use_dirs` returns `["./api", "./cmd", ")", "go 1.22", ...]`.
Downstream, `compute_module_count` (lib.rs:109) reports the inflated length
as the module count on the About card, and `collect_units` emits a
`ProjectUnit` per bogus entry — including a unit named `)` and a
`cwd.join(")")` filesystem probe via `read_mod_info`.

The sibling parser gets this right: `go_mod.rs:28` does
`strip_line_comment(raw).trim()` **before** any structural test, so
`) // end` closes a `replace` block correctly there. The two parsers were
split out to share `go_syntax` helpers (ARCH-5 / TASK-1120) but the
normalization order was never unified.

Related, same function: the non-block single-line arm requires a literal
space (`line.strip_prefix("use ")`), so `use(./mymod)` and a tab-separated
`use\t./mymod` are silently dropped — see the companion tokenization
finding.

**Why it matters**: a single legal comment turns a correctly-parsed Go
workspace into garbage output — wrong module count, phantom modules in the
units list, and a stray filesystem read — with no diagnostic. It is a
plain-text formatting choice a user cannot be expected to avoid, and the
failure is silent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `parse_use_dirs` strips the line comment before any structural test (block opener, `)` terminator, directive), matching the normalization order already used in go_mod.rs:28
- [ ] #2 Regression test: a go.work whose block terminator is `) // members` followed by further top-level lines yields exactly the real use dirs — no `)` entry, no absorbed trailing lines
- [ ] #3 Regression test asserts the same for a tab-indented terminator and for `)` followed by a no-whitespace inline comment (`)//members`)
- [ ] #4 Existing go_work tests (inline comments in block, nested-opener warn, single-line use) still pass
<!-- AC:END -->
