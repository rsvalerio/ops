---
id: TASK-1061
title: >-
  PATTERN-1: parse_pnpm_workspace_yaml leaves trailing '# comment' inside list
  items, producing patterns that match no directory
status: Done
assignee: []
created_date: '2026-05-07 21:04'
updated_date: '2026-05-07 23:35'
labels:
  - code-review
  - triage
  - extensions-node
  - pnpm-workspace
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions-node/about/src/units.rs:178-234 parse_pnpm_workspace_yaml only strips `#`-prefixed comment-only lines. YAML allows a trailing comment on the same line as a list item (`- 'apps/*' # legacy`). The current branch strips `- ` then calls unquote on `'apps/*' # legacy`. unquote requires the value to BOTH start AND end with a matching quote — `'apps/*' # legacy` ends with `y`, so unquote returns the entire string verbatim. The resulting include pattern `'apps/*' # legacy` matches no directory and the workspace member is silently dropped from the About card. Compare with the go.work parser, which calls strip_line_comment (// ...) before unquote/use. Add a YAML-aware trailing-comment stripper for unquoted items: scan from the right, drop any `#` that follows whitespace, then unquote/trim. Quoted items can legitimately contain `#` so only strip the comment from items that are NOT inside matching quotes. Add regression tests covering: (1) `- 'apps/*' # note`, (2) `- apps/* # note`, (3) `- '#literal-pattern'` (must NOT be stripped — the # is inside quotes).
<!-- SECTION:DESCRIPTION:END -->
