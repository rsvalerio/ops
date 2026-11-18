---
id: TASK-2732
title: 'TEST-4: repeated provider-to-identity setup across lib.rs tests'
status: Done
assignee: []
created_date: '2026-11-18 18:35'
updated_date: '2026-11-18 22:24'
labels:
  - code-review-rust
  - test
dependencies: []
parent_task_id: 'TASK-2848'
modified_files:
  - extensions-node/about/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-4:extensions-node/about/src/lib.rs:tests'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/lib.rs:179-434` (`mod tests`)

**What**: Ten tests repeat the same setup: `let provider = NodeIdentityProvider; let mut ctx = Context::test_context(dir.path().to_path_buf()); let id: ProjectIdentity = serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();`. They are at roughly lines 198-201, 253-256, 327-330, 344-347, 359-362, 371-374, 386-389, 396-399, 415-418 and 429-432. Only the fixture differs between them.

**Why it matters**: TEST-4 asks for a helper when setup is identical across 3 or more tests. A small `fn identity_for(dir: &Path) -> ProjectIdentity` removes about 30 lines of noise and leaves each test with only its fixture and assertions. It would also give one place to change if the provider's context or output shape changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A single test helper produces ProjectIdentity from a fixture dir; the ten call sites use it
- [x] #2 All ops-about-node tests still pass

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.09 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
