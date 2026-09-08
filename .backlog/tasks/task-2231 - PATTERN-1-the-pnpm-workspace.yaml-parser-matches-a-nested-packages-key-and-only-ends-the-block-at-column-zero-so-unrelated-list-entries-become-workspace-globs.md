---
id: TASK-2231
title: >-
  PATTERN-1: the pnpm-workspace.yaml parser matches a nested packages: key and
  only ends the block at column zero, so unrelated list entries become workspace
  globs
status: To Do
assignee:
  - TASK-2238
created_date: '2026-09-08 07:23'
updated_date: '2026-09-08 10:55'
labels:
  - code-review-rust
  - correctness
dependencies: []
modified_files:
  - extensions-node/about/src/units.rs
priority: low
ordinal: 137000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/units.rs:228`

**What**: `parse_pnpm_workspace_yaml` is indentation-blind in two connected
ways.

1. The key is matched on the *left-trimmed* line
   (`trimmed_start.strip_prefix("packages:")`, line 238), so a `packages:` key
   nested under any other mapping is accepted as the workspace list.
2. The block is only closed by a line with **zero** leading whitespace
   (`leading_ws == 0`, line 256). The indentation at which `packages:` itself
   appeared is never recorded, so a sibling key at the same non-zero indent
   does not end the block and its list items are appended to the globs.

```yaml
tooling:
  packages:
    - apps/*
  ignoredBuiltDependencies:
    - esbuild
```

yields `["apps/*", "esbuild"]`. Neither entry should have been read at all,
and `esbuild` is then resolved against the project root as a member glob.

**Why it matters**: pnpm's workspace file has grown sibling top-level keys
(`catalog:`, `catalogs:`, `onlyBuiltDependencies:`, `ignoredBuiltDependencies:`,
`overrides:`) whose values are lists, and a nested shape is a realistic
hand-edit. The failure is silent — a bogus glob simply resolves to no
directory — so an operator sees a short or empty units table with no
breadcrumb, the same class of silent-wrong-globs the module already calls out
for unsupported YAML scalars.

Related, same hand-rolled-parser class in the Go stack: TASK-2181 (an
unterminated `use (` / `replace (` block swallows the rest of the file).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 only a top-level packages: key is recognised as the workspace list
- [ ] #2 the block ends at the first subsequent line whose indentation is not greater than the packages: key's own indentation
- [ ] #3 a fixture with a nested packages: block followed by a sibling list key yields no entries from the sibling key
- [ ] #4 existing top-level block and inline-flow fixtures continue to parse unchanged
<!-- AC:END -->
