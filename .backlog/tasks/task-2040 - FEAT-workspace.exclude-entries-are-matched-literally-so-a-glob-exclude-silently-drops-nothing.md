---
id: TASK-2040
title: >-
  FEAT: workspace.exclude entries are matched literally, so a glob exclude
  silently drops nothing
status: To Do
assignee:
  - TASK-2048
created_date: '2026-08-29 06:53'
updated_date: '2026-08-29 11:35'
labels:
  - code-review-rust
dependencies: []
modified_files:
  - extensions-rust/about/src/members.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/members.rs` (`resolved_workspace_members`, around the `resolved.retain(|m| !exclude.contains(m.as_str()))` line)

**What**: `[workspace].exclude` is applied as an exact string-set membership test against the *resolved* member list. Cargo, however, accepts the same glob shapes in `exclude` that it accepts in `members` — `exclude = ["crates/generated-*"]` is legal and excludes every matching directory. `ops about` expands `members` globs (`expand_member_glob`) but never expands `exclude`, so a glob exclude matches no resolved member and every directory the user meant to drop is still counted.

**Why it matters**: the failure is silent and biased toward *over*-counting. `module_count` (identity provider) and the `ProjectUnit` list (units / coverage providers) include crates the workspace explicitly excluded, so `ops about` diverges from `cargo metadata` for any workspace using a glob exclude — with no warn to explain the discrepancy.

Note this is incomplete behaviour rather than a wrong result for literal excludes, which are handled correctly. Raised by CodeRabbit on PR #39 and deliberately deferred out of the review pass as a feature request.

**Possible fix**: route `exclude` entries through the same `classify_member` / `expand_member_glob` machinery as `members` before building the retain set, or match each resolved member against the exclude patterns with the same single-trailing-`*` semantics; either way keep `MemberShape::Unsupported` warning rather than silently ignoring a shape that cannot be expanded.
<!-- SECTION:DESCRIPTION:END -->
