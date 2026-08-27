---
id: TASK-1721
title: >-
  SEC-14: go.work `use` directive with embedded `..` escapes the project root
  and reads an out-of-tree go.mod
status: Triage
assignee: []
created_date: '2026-08-27 11:10'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-go/about/src/modules.rs
  - extensions-go/about/src/go_mod.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/modules.rs:71-121` (`unit_from_use_dir`)

**What**: `unit_from_use_dir` only inspects the **first** path component of a
normalized `go.work` `use` directive:

```rust
let first_component = Path::new(&normalized).components().next();
let out_of_tree_via_components = matches!(
    first_component,
    Some(Component::RootDir | Component::Prefix(_) | Component::ParentDir)
);
let out_of_tree_via_string = normalized
    .split(['/', '\\'])
    .next()
    .is_some_and(|first| first == "..");
```

A directive such as `use ./api/../../../etc` normalizes to
`api/../../../etc`. Its first component is `Normal("api")`, so
`out_of_tree` is `false` and `is_absolute_directive` is `false`. The
function then does:

```rust
let mod_path = cwd.join(&normalized);   // <cwd>/api/../../../etc
read_mod_info(&mod_path)                // opens <that>/go.mod
```

`Path::join` does not normalize `..`, and `std::fs::File::open` (via
`ops_about::manifest_io::read_optional_text`) resolves it lexically at the
OS layer, so the read lands **outside the project root**. The `module`
line found there is then written into the emitted `ProjectUnit`'s
`description`, and the traversal path itself into `ProjectUnit::path` —
both of which are rendered in the About card. No `tracing::warn!` is
emitted, because the out-of-tree branch never fires.

The sibling parser in the *same crate* already guards this exact class:
`go_mod.rs::parse_replace_directive` calls
`has_embedded_parent_dir_segment` (TASK-1212) and rejects a replace target
whose `..` appears past the leading prefix run, explicitly citing the
SEC-14 scrub policy of `extensions/about/src/workspace.rs`
(`resolve_member_globs`, TASK-1071). The `go.work` `use` path never got
the equivalent guard: TASK-1027 narrowed the check to the first component,
and TASK-1208 extended it to `RootDir`/`Prefix` — neither covers `..`
after a normal segment.

**Why it matters**: `ops about` runs in whatever working directory the user
is in, and the threat model this crate already documents (TASK-1071 /
TASK-1208 / TASK-1212) is an adversarial repository. A checked-out repo
carrying a crafted `go.work` gets an arbitrary out-of-tree `go.mod`-shaped
file opened and its `module` path echoed into the About output —
filesystem probing plus information disclosure, driven entirely by
untrusted repository content. It also breaks the parity that TASK-1212
deliberately established between `replace` targets and `use` directives.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A go.work `use` directive whose normalized path contains a `..` segment past the leading prefix run (e.g. `./api/../../../etc`) is treated as out-of-tree: no filesystem read of the target's go.mod occurs, the unit is emitted with no module/version, and the description carries the `(outside project root)` marker
- [ ] #2 The embedded-`..` check is shared with go_mod.rs rather than reimplemented: `has_embedded_parent_dir_segment` moves to `go_syntax.rs` (or another single home) and both `parse_replace_directive` and `unit_from_use_dir` call it
- [ ] #3 A `tracing::warn!` is emitted for the rejected directive, Debug-formatted per the ERR-7 policy already used at that call site
- [ ] #4 Regression test: a go.work with `use ./api/../../../<tmpdir>` where a real go.mod sits at the traversal target asserts the target's module name does NOT appear in any emitted ProjectUnit description (mirrors `collect_units_absolute_use_directive_is_marked_out_of_tree`)
- [ ] #5 Leading-`..` directives (`use ../shared`) keep their existing accepted-but-marked-out-of-tree behaviour; `collect_units_dotdot_prefixed_dir_is_in_tree` still passes
<!-- AC:END -->
