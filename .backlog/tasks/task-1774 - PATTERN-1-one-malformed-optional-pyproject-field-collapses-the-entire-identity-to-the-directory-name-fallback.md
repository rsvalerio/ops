---
id: TASK-1774
title: >-
  PATTERN-1: one malformed optional pyproject field collapses the entire
  identity to the directory-name fallback
status: Triage
assignee: []
created_date: '2026-08-27 11:22'
labels:
  - code-review-rust
  - idioms
dependencies: []
modified_files:
  - extensions-python/about/src/lib.rs
  - extensions-python/about/src/units.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:136-177` (`RawPyproject` / `RawProject`) and `lib.rs:188-203` (the all-or-nothing error arm)

**What**: `parse_pyproject` deserializes the whole manifest in one `toml::from_str::<RawPyproject>` call. `Option<T>` on those fields only models *absence*, not *type mismatch* — so any single field whose type does not match aborts the entire deserialization and every other field is lost.

Verified against `toml` 0.8 with the crate's exact struct shapes:

```
[project]
name = "demo"
version = "1.2.3"
authors = ["Alice <a@x.com>"]
```
→ `Err(invalid type: string "Alice <a@x.com>", expected struct RawAuthor, keys: ["project", "authors"])`

The `authors`-as-list-of-strings form is a very common real-world mistake (it is what Poetry's `[tool.poetry]` table uses, and authors migrating to `[project]` carry it over). The same happens for `version = 1` (unquoted), a `description` written as a table, or a `[project.urls]` value that is not a string.

The consequence: `parse_pyproject` returns `None`, `unwrap_or_default()` at `lib.rs:85` produces an empty `Pyproject`, and the About card falls back to the *directory name* with no version, no license, no description, no URLs — even though `name` and `version` in the example above are perfectly well-formed and sitting right there in the file.

**Why it matters**: `ops about` reports the wrong project name and drops every identity field on manifests that are common in the wild, because of one unrelated field. Every stack provider is expected to degrade gracefully — the crate doc (`lib.rs:8-10`) frames the fallback as being for a *malformed manifest*, but here the manifest is well-formed TOML and mostly valid PEP 621. The `#[serde(default)]` + per-field-tolerant shape (e.g. deserialize `[project]` into a `toml::Table` / `toml::Value` map and project each field independently, or make `authors` an untagged enum accepting both the table and the bare-string form) recovers everything except the one bad field.

Note the sibling `units.rs` `RawRoot` shape has the same exposure for `[tool.uv.workspace].members` / `exclude`: one non-string element there zeroes the whole workspace unit list.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A pyproject.toml with a valid name/version and an authors list of bare strings still yields the correct name and version in ProjectIdentity
- [ ] #2 authors written as a list of bare strings is either parsed (untagged enum accepting both PEP 621 table and string form) or skipped, without discarding the rest of [project]
- [ ] #3 A type mismatch on any single [project] field degrades that field only; the others still populate
- [ ] #4 The per-field failure still emits a tracing warn naming the offending field path
- [ ] #5 units.rs read_workspace_members likewise tolerates a bad members/exclude element without returning an empty unit list
<!-- AC:END -->
