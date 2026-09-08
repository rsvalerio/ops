---
id: TASK-2112
title: >-
  API-13: ops-git re-exports duplicate still-public module paths and re-exports
  a foreign crate item
status: To Do
assignee:
  - TASK-2247
created_date: '2026-09-08 06:53'
updated_date: '2026-09-08 10:59'
labels:
  - code-review-rust
  - api
dependencies: []
modified_files:
  - extensions/git/src/lib.rs
  - extensions/git/src/config.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/lib.rs:19-26`, `extensions/git/src/config.rs:5`

**What**: `lib.rs` declares `pub mod config; pub mod provider; pub mod remote;` and then re-exports the same items at the crate root:

```rust
pub use provider::{resolve_repository_with_git_fallback, GitInfo, GitInfoProvider, DATA_PROVIDER_NAME};
pub use remote::{parse_remote_url, RemoteInfo};
```

So `ops_git::GitInfo` and `ops_git::provider::GitInfo` are both public and both valid — two documented paths to one type. Separately, `config.rs:5` does `pub use ops_hook_common::find_git_dir;`, re-exporting another crate's item as part of `ops_git`'s public API.

**Why it matters**: Duplicate paths split rustdoc, make "where does this live" ambiguous for callers, and mean a future move between modules breaks whichever path callers happened to pick. The foreign re-export makes `ops-hook-common`'s API surface implicitly part of `ops-git`'s contract: a signature or behaviour change there is a silent breaking change here, and `find_git_dir` shows up in `ops_git::config` docs with no indication that ops-git does not own it.

<!-- Reviewer note: pick one canonical path per item — either keep the modules public and drop the root re-exports, or make the modules `pub(crate)` and keep the flat root API. For find_git_dir, either call `ops_hook_common::find_git_dir` at the (single) use site in provider.rs or wrap it in a documented local fn. -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each public ops-git item is reachable through exactly one path
- [ ] #2 find_git_dir is no longer re-exported from ops_git::config, or the re-export is documented as a deliberate part of ops-git's contract
- [ ] #3 cargo build and the workspace test suite pass after the path consolidation
<!-- AC:END -->
