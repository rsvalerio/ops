---
id: TASK-2233
title: 'READ-13: ops-about-node docs and comments are a change journal of TASK ids and past bugs rather than a description of current behaviour'
status: Done
assignee: []
created_date: '2026-09-08 07:24'
updated_date: '2026-09-10 19:31'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2248'
modified_files:
  - extensions-node/about/src/repo_url.rs
  - extensions-node/about/src/package_json.rs
  - extensions-node/about/src/units.rs
  - extensions-node/about/src/package_manager.rs
priority: low
ordinal: 139000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/repo_url.rs:31`, `extensions-node/about/src/package_json.rs:9`, `extensions-node/about/src/units.rs:158`, `extensions-node/about/src/package_manager.rs:16`

**What**: Rationale and history dominate the crate's documentation. Doc
comments describe what the code *used to do* and which task changed it, so a
reader must reconstruct the current contract from the diff narrative.

Representative sites:

- `repo_url.rs:31-70` — a 40-line doc comment on `normalize_repo_url` in
  which the actual contract is two lines and the remainder is six numbered
  historical fixes ("TASK-1080 ... TASK-1165 ... PERF-3 / TASK-1257 ...
  SEC-11 / TASK-1722"), each restating what the previous shape did wrong.
- `package_json.rs:9-16` — the doc comment on `PackageJson` is entirely about
  a `#[non_exhaustive]` attribute that is no longer there; it never says what
  the type represents.
- `units.rs:158-169` and `package_manager.rs:16-22` — the same shape on
  `split_include_exclude` and `detect_package_manager`.
- Inline `// RULE-ID / TASK-NNNN:` markers appear roughly 40 times across the
  four source files, including on test functions.

**Why it matters**: the task ids resolve to a backlog a reader of the source
does not have, and the historical framing hides the invariant that is actually
being asserted. `package_json.rs:9` is the failure mode made concrete — the
only documentation on a public type describes a deleted attribute. The
regression-pinning value of these notes is already carried by the tests; the
doc comment should state the current rule and let the test name carry the
history.

**Twins** (same class, other crates — cross-reference, do not merge):
TASK-2191 (about-go), TASK-2208 (about-python), TASK-2182 (ops-deps).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 doc comments state the current contract of each item; history and task ids are removed from them
- [x] #2 the PackageJson doc comment describes the type rather than a removed attribute
- [x] #3 where a rule must stay pinned, the invariant is named in the test, not narrated in the item's docs

<!-- AC:END -->
