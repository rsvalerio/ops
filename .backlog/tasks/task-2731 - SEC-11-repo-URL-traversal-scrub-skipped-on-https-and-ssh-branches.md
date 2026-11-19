---
id: TASK-2731
title: 'SEC-11: repo URL traversal scrub skipped on https and ssh branches'
status: To Do
assignee: []
created_date: '2026-11-18 18:35'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - security
dependencies: []
parent_task_id: 'TASK-2848'
modified_files:
  - extensions-node/about/src/repo_url.rs
priority: low
ordinal: 1000
dedup_key: 'SEC-11:extensions-node/about/src/repo_url.rs:normalize_repo_url_shape'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/repo_url.rs:130` (`normalize_repo_url_shape`, clean-URL fall-through), `extensions-node/about/src/repo_url.rs:172` (`ssh_to_https`), module doc `extensions-node/about/src/repo_url.rs:14`

**What**: The module doc lists "`..` path traversal" among the rejection rules. In practice `..` is stripped, not rejected, and only on some branches: npm shorthand, `git://`, `git+<scheme>://` and `append_tree_directory`. Two branches pass `..` through untouched:
- plain `https://github.com/o/../../evil/x` returns `Cow::Borrowed` unchanged
- `ssh://git@github.com/o/../../evil/x` becomes `https://github.com/o/../../evil/x`

So `git+https://host/o/../../x` is scrubbed while `https://host/o/../../x` is not.

**Why it matters**: The repository value is untrusted manifest text that is rendered as a clickable link. The host cannot change, so the impact is low, but the sanitisation boundary is inconsistent across branches, and the doc overstates it. A reader trusting the doc will assume that no traversal-shaped URL can come out.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every branch of normalize_repo_url applies the same traversal policy (scrub or drop), including the clean https fall-through and ssh_to_https
- [ ] #2 The module doc describes the actual policy (strip vs reject)
- [ ] #3 Tests pin https://host/o/../../x and ssh://git@host/o/../../x
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.09 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
