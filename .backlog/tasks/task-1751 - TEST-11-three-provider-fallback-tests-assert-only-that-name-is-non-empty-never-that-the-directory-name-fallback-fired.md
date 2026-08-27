---
id: TASK-1751
title: >-
  TEST-11: three provider fallback tests assert only that name is non-empty,
  never that the directory-name fallback fired
status: Triage
assignee: []
created_date: '2026-08-27 11:15'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-java/about/src/maven/mod.rs
  - extensions-java/about/src/gradle/tests.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/mod.rs:62`, `extensions-java/about/src/maven/mod.rs:74`, `extensions-java/about/src/gradle/tests.rs:470`

**What**: Three tests exist to cover the "no manifest name — fall back to the working-directory name" path (`build_identity_value` in `extensions/about/src/identity.rs`: *"Falls back to the working-directory name when `name` is absent"*), but none of them asserts the fallback value:

| Test | Assertion |
|---|---|
| `maven_provider_provide_no_pom` (maven/mod.rs:67) | `assert!(!name.is_empty())` |
| `maven_provider_provide_uses_dir_name_fallback` (maven/mod.rs:85) | `assert!(!name.is_empty())` |
| `gradle_provider_provide_minimal` (gradle/tests.rs:477) | `assert!(!name.is_empty())` |

`maven_provider_provide_uses_dir_name_fallback` is the clearest case: its name states the contract it is testing and its body never checks it. All three would pass if the fallback returned a hardcoded `"unknown"`, the wrong ancestor directory's name, a whitespace string, or the parent's name instead of the leaf's — i.e. every realistic way the fallback can break.

The expected value is available at zero cost in each test: `dir.path().file_name()` on the `tempfile::tempdir()` already in scope.

**Why it matters**: TEST-11 — these are the only tests covering the fallback, so the assertion strength is the whole coverage. As written they pin "some string came back", which is exactly the assertion a mutation-testing pass survives.

**Fix**: assert the exact expected name in all three, e.g.

```rust
let expected = dir.path().file_name().unwrap().to_str().unwrap();
assert_eq!(result["name"], expected);
```

`maven_provider_provide_no_pom` should additionally keep its distinct concern (no `pom.xml` at all) rather than duplicating the fallback assertion of the sibling test — name each for the scenario it actually pins (TEST-2).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 maven_provider_provide_uses_dir_name_fallback asserts result["name"] equals the tempdir's file_name, not just that it is non-empty
- [ ] #2 maven_provider_provide_no_pom and gradle_provider_provide_minimal assert the same exact fallback value
- [ ] #3 The three tests remain distinct in what they pin (no manifest / manifest without a name / gradle settings without rootProject.name) and their names reflect it
<!-- AC:END -->
