---
id: TASK-1757
title: >-
  TEST-5: normalize_urls keep-first collision contract (TASK-1110) is entirely
  untested
status: Done
assignee:
  - TASK-1992
created_date: '2026-08-27 11:18'
updated_date: '2026-08-28 20:04'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-python/about/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:317-346` (`normalize_urls`)

**What**: `normalize_urls` carries a 9-line doc comment (`lib.rs:308-316`) describing a deliberate, non-obvious policy added by PATTERN-1 / TASK-1110: when two `[project.urls]` keys collapse to the same normalised form (e.g. `"Homepage"` and `"home page"`, or `"Source-Code"` and `"source code"`), the function keeps the **first-seen** entry and emits a `tracing::warn!` naming both raw keys and both URLs, rather than letting a `HashMap` silently pick a last-write-wins winner.

Neither half of that contract has a test:

- No test constructs a `BTreeMap` with two colliding keys, so "first-seen wins" is unverified. The only collision-adjacent test, `pick_url_repository_takes_precedence_over_source_and_source_code` (`lib.rs:484`), uses `Source-Code` / `source` / `Repository` — three keys that normalise to three *distinct* strings, so it exercises candidate-list precedence, not collision handling.
- The `tracing::warn!` at `lib.rs:331` is never asserted, and neither is `first_seen_raw` (`lib.rs:322`), the map that exists solely to populate the `first_key` field of that warn.

**Why it matters**: "first-seen wins" is order-dependent behaviour that rides on `BTreeMap` iteration order — precisely the kind of invariant that regresses invisibly. The whole `first_seen_raw` bookkeeping map plus the collision branch (roughly 20 of the function's 30 lines) is dead as far as the test suite can tell: replace the body with `urls.iter().map(|(k, v)| (normalize_url_key(k), v)).collect()` — the exact last-write-wins bug TASK-1110 was filed to prevent — and every test still passes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A test feeds normalize_urls two raw keys that collapse to the same normalised key and asserts the first-seen (BTreeMap-order) URL is the one retained
- [x] #2 A test asserts the collision emits a tracing warn carrying both raw keys and both URLs (via ops_about::test_support)
- [x] #3 A test covers the end-to-end case: a pyproject.toml with both `Homepage` and `home page` yields the first-seen URL in ProjectIdentity.homepage
- [x] #4 Reverting normalize_urls to a naive .collect() makes at least one of the new tests fail
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
`colliding_url_keys_keep_the_first_seen_entry_and_warn` feeds `normalize_urls`
two raw keys that collapse to one normalised key and asserts both halves of
the contract: the first-seen (BTreeMap-order) URL is retained, and the warn
carries `first_key`, `first_url`, `duplicate_key`, `duplicate_url` and
`recovery="keep-first"`.
`colliding_url_keys_in_a_manifest_keep_the_first_seen_homepage` covers the
end-to-end path through a real `pyproject.toml`.

Note on the colliding pair: `normalize_url_key` lowercases and maps `-` to a
space, so "Homepage" (-> "homepage") and "home page" do NOT collide — the
example in the finding is off by one. The tests use "Home-Page" / "home page",
which both normalise to "home page".

AC#4 verified empirically: temporarily replacing the body with
`urls.iter().map(|(k, v)| (normalize_url_key(k), (k, v))).collect()` fails
both new tests (2 failed / 43 passed); reverted afterwards.
<!-- SECTION:NOTES:END -->
