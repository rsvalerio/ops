---
id: TASK-1315
title: >-
  TEST-25: collect_compiled_extensions_unfiltered_by_config discards its own
  subject with let _ = compiled
status: Done
assignee:
  - TASK-1383
created_date: '2026-05-11 20:26'
updated_date: '2026-05-12 23:16'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/tests.rs:111-127`

**What**: The test is named `collect_compiled_extensions_unfiltered_by_config` and the doc comment says *"the key point is that it's not filtered by the enabled list"*. The body, however, only asserts that the **filtered** call returns empty, and then drops the unfiltered result without inspection:

```rust
let compiled = collect_compiled_extensions(&config, std::path::Path::new("."));
let filtered = builtin_extensions(&config, std::path::Path::new(".")).unwrap();
assert!(filtered.is_empty());
let _ = compiled;     // <-- the subject of the test is silently discarded
```

So nothing in the test verifies that `collect_compiled_extensions` *ignores* the empty `enabled` list. If `collect_compiled_extensions` were changed to filter by `config.extensions.enabled` (the exact regression the test is meant to catch), this test would still pass.

**Why it matters**: TEST-25 — name promises one behaviour, body asserts another. The test gives misleading coverage signal: the registration-vs-discovery split is a non-trivial invariant (it's why `builtin_extensions` and `collect_compiled_extensions` are separate functions), and the only test guarding it is a no-op on the side that matters. Combined with [[task-1309]] (extension_info_provides_metadata asserts nothing) and the sibling TEST-11 in this same file, the registry test module has a pattern of compiling-but-not-asserting tests.

The fix is straightforward: assert that `compiled` contains at least one entry (under a feature gate that guarantees compiled-in extensions), or, on default features, assert `compiled.len() >= filtered.len()` and that every name in `filtered` appears in `compiled` — i.e. the actual unfiltered-superset property.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test body actually exercises the 'unfiltered' invariant — e.g. asserts every name in filtered appears in compiled, or compiled.len() >= filtered.len(), and (under a feature gate that guarantees at least one compiled extension) asserts compiled is non-empty when enabled = vec![]
- [ ] #2 If collect_compiled_extensions were modified to filter by config.extensions.enabled, this test fails
- [ ] #3 The doc comment is rewritten to describe what the assertions actually verify (or removed if the assertions are self-explanatory)
<!-- AC:END -->
