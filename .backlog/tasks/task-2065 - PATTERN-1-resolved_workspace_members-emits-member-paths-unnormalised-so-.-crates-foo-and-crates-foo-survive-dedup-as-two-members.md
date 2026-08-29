---
id: TASK-2065
title: >-
  PATTERN-1: resolved_workspace_members emits member paths unnormalised, so
  ./crates/foo and crates/foo survive dedup as two members
status: Triage
assignee: []
created_date: '2026-08-29 17:57'
labels:
  - code-review-rust
  - correctness
dependencies: []
modified_files:
  - extensions-rust/about/src/members.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/members.rs` (`resolved_workspace_members`)

**What**: TASK-2055 taught `ExcludeSet::excludes` to normalise a leading `./`
on both sides, so a `./`-spelled entry no longer defeats an exclude match. The
*output* list is still emitted verbatim: a `[workspace].members` entry written
`./crates/foo` is pushed through `MemberShape::Literal` unchanged, so the
`sort` + `dedup` in `resolved_workspace_members` sees `./crates/foo` and
`crates/foo` as two distinct strings.

**Why it matters**: same over-counting failure TASK-2040 and TASK-2055 fixed,
one step later in the pipeline. A workspace listing both spellings (or a glob
plus a `./`-prefixed literal that the glob also expands) double-counts the
crate in `module_count` (identity provider) and emits duplicate `ProjectUnit`s
in the units / coverage providers, diverging from `cargo metadata` with no warn.
Consumers see the raw strings too — the paths are joined against the workspace
root downstream, and `create-review-tasks-rust` uses them as target identity.

**Fix direction**: normalise each resolved member (strip leading `./`, collapse
repeated separators) before `sort`/`dedup`, reusing the `strip_dot_prefix` /
`path_segments` helpers TASK-2055 added. Left out of TASK-2055 deliberately:
`resolved_workspace_members` is `pub` and read by sibling extension crates, so
changing the shape of what it returns is a cross-crate change, not a bounded
one.

**Origin**: discovered during TASK-2064 (code-review-plan-wave193) while fixing
TASK-2055.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A workspace listing both ./crates/foo and crates/foo resolves to a single member
- [ ] #2 Resolved member paths are emitted in one canonical spelling, and sibling consumers still resolve against the workspace root
<!-- AC:END -->
