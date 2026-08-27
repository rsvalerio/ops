---
id: TASK-1731
title: >-
  DUP-1: lib.rs re-declares an identical `GoMod` struct and copies it
  field-by-field from `go_mod::GoMod`
status: Triage
assignee: []
created_date: '2026-08-27 11:12'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions-go/about/src/lib.rs
  - extensions-go/about/src/modules.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/lib.rs:121-134`, `extensions-go/about/src/go_mod.rs:13-18`, `extensions-go/about/src/modules.rs:187-189`

**What**: `lib.rs` declares a private `GoMod` whose fields are identical —
same names, same types, same order — to `go_mod::GoMod`, and `parse_go_mod`
exists only to copy one into the other:

```rust
// lib.rs
struct GoMod {
    module: Option<String>,
    go_version: Option<String>,
    local_replaces: Vec<String>,
}

fn parse_go_mod(project_root: &Path) -> Option<GoMod> {
    let raw = go_mod::parse(project_root)?;
    Some(GoMod {
        module: raw.module,
        go_version: raw.go_version,
        local_replaces: raw.local_replaces,
    })
}
```

`go_mod::GoMod` already exposes all three fields as `pub(crate)`, so the
copy buys no encapsulation. It does cost: adding a field to the parser
requires a matching edit in `lib.rs` or it is silently unavailable to
`compute_module_count`, and the go_mod test module already has to construct
the *other* `GoMod` by hand to reach `compute_module_count`
(`go_mod.rs:465-470`), which is the duplication surfacing as test friction.

A second, smaller instance of the same pattern sits in `modules.rs`:

```rust
fn workspace_use_dirs(root: &Path) -> Option<Vec<String>> {
    crate::go_work::parse_use_dirs(root)
}
```

— a pass-through wrapper that adds no behaviour, no normalization, and no
name clarification over the function it calls.

**Why it matters**: two structurally identical types with the same name in
one crate make "which `GoMod`?" a live question at every use site, and the
manual field copy is the classic place a newly added field goes missing.
Neither indirection layer earns its maintenance cost.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `lib.rs` uses `go_mod::GoMod` directly; the duplicate local struct and `parse_go_mod` are deleted (or `parse_go_mod` becomes a plain re-export/alias with no field copying)
- [ ] #2 `compute_module_count` takes `Option<&go_mod::GoMod>` and the go_mod test at go_mod.rs:465 stops hand-constructing a second GoMod
- [ ] #3 `modules::workspace_use_dirs` is removed and `collect_units` calls `crate::go_work::parse_use_dirs` directly, or the wrapper gains behaviour that justifies it
- [ ] #4 All existing tests in lib.rs, go_mod.rs and modules.rs still pass unchanged in intent
<!-- AC:END -->
