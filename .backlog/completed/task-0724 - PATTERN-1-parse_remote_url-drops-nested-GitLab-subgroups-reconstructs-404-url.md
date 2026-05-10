---
id: TASK-0724
title: >-
  PATTERN-1: parse_remote_url drops nested GitLab subgroups, reconstructs 404
  url
status: Done
assignee:
  - TASK-0736
created_date: '2026-04-30 05:47'
updated_date: '2026-04-30 13:01'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/remote.rs:103-117` (split_owner_repo) and `:56` (RemoteInfo url synthesis)

**What**: For nested GitLab subgroup URLs like `https://gitlab.com/group/subgroup/repo.git`, `split_owner_repo` keeps only the last two segments (`subgroup`, `repo`) and `parse_remote_url` then synthesises `url = https://gitlab.com/subgroup/repo` — a path that does not exist on gitlab.com (404). The behaviour is documented in a `#[test] gitlab_nested_group_uses_last_two_segments` and a comment "callers can refine later if needed", but the resulting `RemoteInfo` is consumed verbatim by `provider.rs` and emitted as the canonical `git_info.remote_url`. Downstream consumers (provenance metadata, clickable links in reports) get a confidently-wrong URL with no signal that the parse was lossy.

**Why it matters**: Silent data loss / correctness — `git_info.remote_url` is the field every other extension trusts as the source of truth for a project repo URL, and it is provably wrong for any GitLab subgroup project. The host/owner/repo split should preserve the full owner path or refuse to flatten it; the documented "last-wins" approach trades correctness for parser simplicity in a context where the value is then re-published as ground truth.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 split_owner_repo preserves the full owner path for nested groups (e.g. owner = 'group/subgroup') OR parse_remote_url returns None for >2-segment paths
- [x] #2 the synthesised RemoteInfo.url round-trips: fetching it returns the same project as the input URL
- [x] #3 regression test covers gitlab.com/group/subgroup/repo and asserts the reconstructed url is reachable-shaped
<!-- AC:END -->
