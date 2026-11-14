---
id: TASK-2196
title: 'DUP-1: set_module and set_go_version in go_mod.rs are the same first-wins-unquote-nonempty routine written twice'
status: Done
assignee: []
created_date: '2026-09-08 07:15'
updated_date: '2026-09-10 16:26'
labels:
  - code-review-rust
  - duplication
dependencies: []
parent_task_id: 'TASK-2242'
modified_files:
  - extensions-go/about/src/go_mod.rs
priority: low
ordinal: 109000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_mod.rs:85` (`set_module`) and `extensions-go/about/src/go_mod.rs:96` (`set_go_version`)

**What**: The two functions are identical apart from the field they assign:

```rust
fn set_module(out: &mut GoMod, rest: &str) {
    if out.module.is_some() { return; }
    let value = unquote_token(rest.trim());
    if !value.is_empty() { out.module = Some(value.into_owned()); }
}
```

`set_go_version` is the same five statements against `out.go_version`. Both encode one policy — "first directive wins; unquote the token; drop it if empty" — and both are reached from two call sites each (the block-form arm at `go_mod.rs:60-62` and the single-line verb arm at `go_mod.rs:73-77`), so the duplication is already load-bearing in four places.

**Why it matters**: The `trim_nonempty` policy documented on `set_module` (the ERR-2 fallback contract with the Node and Python identity providers) has to be kept in sync by hand across two copies. A future change — normalising a `go` version prefix, say, or rejecting a quoted empty string — will land in one and not the other, and the resulting drift is exactly the failure the doc comment was written to prevent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The first-wins / unquote / drop-empty policy exists once (e.g. a helper taking &mut Option<String>) and both the module and go-version directives route through it
- [x] #2 The trim_nonempty rationale is documented on the single shared implementation
- [x] #3 Existing go_mod tests, including parses_quoted_module_and_replace_target and the whitespace-only module fallback, still pass unchanged

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Extracted set_first_wins_unquoted_nonempty(&mut Option<String>, &str) carrying the first-wins/unquote/drop-empty policy and the ERR-2 / TASK-1167 trim_nonempty rationale (extended to note it covers an empty go-directive value too); set_module and set_go_version are one-line wrappers over it. 91 ops-about-go tests green including parses_quoted_module_and_replace_target and the whitespace/local-replace fallbacks, clippy clean.
<!-- SECTION:NOTES:END -->
