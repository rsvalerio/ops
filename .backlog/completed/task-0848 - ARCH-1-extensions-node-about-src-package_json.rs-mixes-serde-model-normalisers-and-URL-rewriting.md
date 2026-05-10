---
id: TASK-0848
title: >-
  ARCH-1: extensions-node/about/src/package_json.rs mixes serde model,
  normalisers, and URL rewriting
status: Done
assignee: []
created_date: '2026-05-02 09:16'
updated_date: '2026-05-02 14:16'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_json.rs`

**What**: 514 lines (~250 production). The file owns: (a) the RawPackage/LicenseField/RepositoryField/PersonField serde shapes, (b) the parse_package_json orchestrator, (c) format_person/trim_nonempty normalisers, (d) normalize_repo_url/ssh_to_https/is_numeric_port_prefix/append_tree_directory URL rewriting (which is the SEC-14 path-sanitisation surface). The repo-URL rewriting is a self-contained concern that is the highest-risk code in the file.

**Why it matters**: When the SEC-14 traversal-suffix bug (TASK-0811) was fixed, the fix shipped right next to the serde model. Future adversarial-input fixes deserve a dedicated repo_url.rs module so they have a clear test target and a documented boundary.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 normalize_repo_url, ssh_to_https, append_tree_directory, is_numeric_port_prefix move to extensions-node/about/src/repo_url.rs along with their tests
- [x] #2 package_json.rs shrinks to serde shapes + parse_package_json + format_person + trim_nonempty
- [x] #3 Public surface unchanged (functions stay pub(crate))
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Moved normalize_repo_url, ssh_to_https, append_tree_directory, is_numeric_port_prefix and their tests to extensions-node/about/src/repo_url.rs (pub(crate)). package_json.rs now imports them via  and shrinks to serde shapes + parse_package_json + format_person + trim_nonempty. Public surface unchanged. SEC-14 path-sanitisation tests for append_tree_directory now live next to their implementation.
<!-- SECTION:NOTES:END -->
