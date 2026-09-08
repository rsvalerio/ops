---
id: TASK-2202
title: >-
  PATTERN-1: [project.urls] is the one all-or-nothing shape left in pyproject
  parsing - a single non-string value drops both homepage and repository
status: Done
assignee:
  - TASK-2238
created_date: '2026-09-08 07:19'
updated_date: '2026-09-08 16:18'
labels:
  - code-review-rust
  - pattern
dependencies: []
modified_files:
  - extensions-python/about/src/lib.rs
priority: medium
ordinal: 115000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:278`

**What**: Every other `[project]` key in `parse_pyproject` degrades per-entry: scalars go through `project_field` (one bad key warns and yields `None`), `authors` uses the untagged `RawAuthorEntry` with an `Unsupported` arm, and the workspace globs in `units.rs` use `RawGlob::Unsupported`. `urls` is the exception — it is deserialised as `project_field::<BTreeMap<String, String>>`, so one non-string value anywhere in the table fails the whole map:

```toml
[project.urls]
Homepage = "https://demo.dev"
Repository = "https://github.com/x/demo"
Funding = { url = "https://sponsor.dev" }   # table, not string
```

This is well-formed TOML, and the two good URLs are string-valued, yet `T::deserialize` fails and both `homepage` and `repository` fall to `None`.

**Why it matters**: It is the same failure mode TASK-1774 removed from the rest of the manifest, reintroduced in the one place the About card's two URL bullets come from. The nested-table spelling is exactly the sort of drift PEP 621 tooling produces, and the only signal is a single `field=project.urls` warn — the operator sees a project with no homepage and no repository link.

**Suggested fix**: deserialise into `BTreeMap<String, toml::Value>` (or a `RawUrlEntry` untagged enum mirroring `RawAuthorEntry`), keep the string-valued entries, and warn per skipped key with `recovery = "skip-entry"`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A [project.urls] table containing one non-string value still yields the homepage and repository from its string-valued siblings
- [ ] #2 The skipped url entry emits a tracing warn naming the key and a recovery field, consistent with the RawAuthorEntry::Unsupported and RawGlob::Unsupported warns
- [ ] #3 A regression test covers the mixed-value [project.urls] manifest end to end through the identity provider
<!-- AC:END -->
