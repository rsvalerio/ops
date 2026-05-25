---
id: TASK-1123
title: >-
  TEST-17: tokei tests scan CARGO_MANIFEST_DIR producing non-deterministic
  results
status: Done
assignee:
  - TASK-1266
created_date: '2026-05-08 07:27'
updated_date: '2026-05-09 14:01'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: \`extensions/tokei/src/tests.rs:191\`, \`extensions/tokei/src/tests.rs:223\`, \`extensions/tokei/src/tests.rs:247\`, \`extensions/tokei/src/tests.rs:266\`

**What**: Four \`#[test]\` functions (\`flatten_tokei_real_project_structure\`, \`flatten_tokei_strips_workspace_prefix\`, \`collect_tokei_on_real_project\`, \`tokei_collect_and_load_cycle\`) call \`PathBuf::from(env!(\"CARGO_MANIFEST_DIR\"))\` and scan the live ops repository tree, asserting on file counts and per-record properties. The sibling \`tokei_provider_returns_valid_json_on_real_project\` recognised the same hazard and is gated behind \`#[ignore = \"scans CARGO_MANIFEST_DIR; non-deterministic and slow (TEST-17)\"]\`, but the four tests above are not ignored — they run on every \`cargo test\` invocation.

**Why it matters**: TEST-17 / TEST-18 require unit tests to use isolated state (tempdirs, fixed inputs) so results do not depend on the shared workspace contents. Today these tests scan the entire crate tree under tokei's exclusion list every run, slowing the test binary and producing assertions whose stability depends on the working tree (e.g. an editor-tempfile or a partially-staged checkout can flip \`!arr.is_empty()\` or alter the per-record sums). The same pattern is the reason \`tokei_provider_returns_valid_json_on_real_project\` was already explicitly ignored — these four are an oversight in the same sweep.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace CARGO_MANIFEST_DIR with a tempdir+fixture for the four named tests, OR mark them `#[ignore = "..."]` matching the precedent set by tokei_provider_returns_valid_json_on_real_project
- [x] #2 Add a regression test or doc that prevents new `env!("CARGO_MANIFEST_DIR")` introductions outside #[ignore]'d cases
<!-- AC:END -->
