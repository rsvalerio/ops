---
id: TASK-1110
title: >-
  PATTERN-1: pyproject normalize_urls collapses duplicate normalised keys,
  silently drops one URL last-write-wins
status: Done
assignee: []
created_date: '2026-05-07 21:47'
updated_date: '2026-05-08 04:18'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:294-300` (`normalize_urls`)

**What**: `normalize_urls` collects a `BTreeMap<String, String>` into a `HashMap<String, &String>` keyed by `normalize_url_key(k)` (`trim().to_ascii_lowercase().replace('-', " ")`). When the source `[project.urls]` table has two keys that collapse under that normalisation — e.g. `"Homepage"` and `"home page"`, or `"Source-Code"` and `"source code"` — `.collect()` into the HashMap silently keeps one (BTreeMap iteration order, last-write-wins) and discards the other with no diagnostic. There is no warn breadcrumb akin to the rest of the file (parse failure, missing severity, schema drift).

**Why it matters**: PEP 621 places no constraint on key casing or punctuation, and pyproject docs in the wild commonly use both kebab and space forms. A user who declared `"Source-Code"` as their canonical repository URL will see whichever one BTreeMap iterates first surface in About, with the other URL silently lost. Same class of finding as TASK-1019 (`Metadata::package_index_by_name`) and TASK-1100 (`package_index_by_id`): a HashMap collect over a normalised key with no de-dup awareness.

**Suggested fix**: Either (a) warn-and-keep-first when an inserted key already exists, or (b) detect collisions during the collect pass and surface a `tracing::warn!` so the operator sees the schema drift instead of an arbitrary winner.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 When two pyproject [project.urls] keys collapse under normalize_url_key, behaviour is deterministic AND a tracing::warn fires
- [ ] #2 Unit test pins both the deterministic-winner choice and the warn breadcrumb
<!-- AC:END -->
