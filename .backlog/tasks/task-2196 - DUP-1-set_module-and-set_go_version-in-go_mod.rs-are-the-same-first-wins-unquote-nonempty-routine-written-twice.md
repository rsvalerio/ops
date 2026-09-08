---
id: TASK-2196
title: >-
  DUP-1: set_module and set_go_version in go_mod.rs are the same
  first-wins-unquote-nonempty routine written twice
status: To Do
assignee:
  - TASK-2242
created_date: '2026-09-08 07:15'
updated_date: '2026-09-08 10:57'
labels:
  - code-review-rust
  - duplication
dependencies: []
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
- [ ] #1 The first-wins / unquote / drop-empty policy exists once (e.g. a helper taking &mut Option<String>) and both the module and go-version directives route through it
- [ ] #2 The trim_nonempty rationale is documented on the single shared implementation
- [ ] #3 Existing go_mod tests, including parses_quoted_module_and_replace_target and the whitespace-only module fallback, still pass unchanged
<!-- AC:END -->
