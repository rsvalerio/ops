---
id: TASK-1122
title: 'DUP-3: repo_url segment-filter pattern duplicated across four helpers'
status: Done
assignee:
  - TASK-1265
created_date: '2026-05-08 07:27'
updated_date: '2026-05-09 13:37'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/repo_url.rs:162-188`

**What**: The `split('/').filter(|seg| !seg.is_empty() && *seg != "." && *seg != "..").collect::<Vec<_>>().join("/")` segment scrub is implemented twice: once inline inside `append_tree_directory` (lines 162-174) and once as the helper `scrub_path_segments` (lines 182-188). `scrub_authority_and_path` and `scrub_full_url_path` then layer on top of `scrub_path_segments`. `append_tree_directory` does not route through the helper, so any future tightening of the SEC-14 segment filter (Unicode bidi controls, encoded `..`, etc.) has to be applied in two places or it silently regresses one branch.

**Why it matters**: DUP-3 flags 3+ occurrences of repeated patterns. Here the security-relevant segment filter is duplicated and the two copies must be kept in lockstep manually. Code-review noted the SEC-14 fix shape was "the same as `append_tree_directory`" — that comment is the duplication red flag.

**Suggested fix**: Have `append_tree_directory` call `scrub_path_segments` on the normalized directory, replacing its inline filter chain. Pin the equivalence with a unit test that calls both entry points on a `..`-laden input.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 append_tree_directory delegates segment scrubbing to scrub_path_segments
- [x] #2 Tightening the segment filter requires only one source-code change
<!-- AC:END -->
