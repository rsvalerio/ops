---
id: TASK-1725
title: >-
  ERR-2: split_include_exclude does not trim or drop empty workspace entries —
  "workspaces": [""] adds the project root as its own unit and suppresses the
  pnpm fallback
status: Triage
assignee: []
created_date: '2026-08-27 11:11'
labels:
  - code-review-rust
  - idioms
dependencies: []
modified_files:
  - extensions-node/about/src/units.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/units.rs:155-168` (`split_include_exclude`), consumed at `units.rs:94-153` (`workspace_member_globs`)

**What**: `split_include_exclude` pushes every `workspaces` / `pnpm-workspace.yaml` entry into `includes` or `excludes` after only `trim_start_matches("./")` and an optional `!` strip. Whitespace-only and empty entries are never trimmed or dropped, unlike every other externally-sourced string in this crate. Two observable consequences:

1. `{"workspaces": [""]}` → `includes == [""]`. `resolve_member_globs` takes the non-glob branch (`extensions/about/src/workspace.rs:158`): `root.join("")` is `root` itself, `try_read_manifest(root, "package.json")` succeeds, and `("", manifest)` is pushed. `collect_units` then emits a bogus `ProjectUnit` whose `path` is `""` and whose name comes from `format_unit_name("")` — the project root listed as a member of itself.
2. `{"workspaces": ["  "]}` or `{"workspaces": [""]}` → `includes.is_empty()` is `false` at `units.rs:128`, so the `pnpm-workspace.yaml` fallback is skipped. A repo with a blank entry in `workspaces` plus a real `pnpm-workspace.yaml` silently reports zero units.

**Why it matters**: The crate has an explicit, repeatedly-applied ERR-2 policy — trim and drop empty — for `name`, `version`, `description`, `license`, `engines.node` (`package_json.rs:114-149`, TASK-0563/0813/0814) and for workspace member metadata (`units.rs:62-65`, TASK-1254). Glob entries are the one externally-sourced string list that never got it. The module doc comment already documents the sibling case ("an `workspaces` array containing only `!`-prefixed exclusions is treated as 'no positive includes' and the pnpm fallback is consulted instead", TASK-0488); an array of only blank entries is the same shape and should be treated the same way, but currently is not. `package.json` is attacker- or typo-controlled input, so this is a silent wrong-output path, not a cosmetic one.

**Fix shape**: in `split_include_exclude`, `trim()` each item and skip it when the trimmed value (after `./` and `!` stripping) is empty, before pushing to either vector.

**Cross-cutting note**: the root-as-its-own-member behaviour is realised inside `ops-about`'s `resolve_member_globs` non-glob branch, but the empty entry originates here and the guard belongs here — every caller of `resolve_member_globs` would otherwise need its own guard.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 split_include_exclude trims each entry and skips entries that are empty after trimming and after ./ and ! stripping
- [ ] #2 A package.json with "workspaces": [""] yields zero project units (the project root is not listed as its own member)
- [ ] #3 A package.json with "workspaces": ["  "] plus a pnpm-workspace.yaml declaring real packages falls through to the pnpm source and resolves those members
- [ ] #4 A pnpm-workspace.yaml packages list containing a blank entry does not produce a root-as-member unit
- [ ] #5 Existing precedence tests (exclude_only_workspaces_falls_back_to_pnpm, npm_workspaces_array_form, pnpm_workspace_yaml) still pass
<!-- AC:END -->
