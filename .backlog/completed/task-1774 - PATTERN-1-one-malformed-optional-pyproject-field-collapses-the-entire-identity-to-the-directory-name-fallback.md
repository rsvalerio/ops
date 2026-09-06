---
id: TASK-1774
title: >-
  PATTERN-1: one malformed optional pyproject field collapses the entire
  identity to the directory-name fallback
status: Done
assignee:
  - TASK-1992
created_date: '2026-08-27 11:22'
updated_date: '2026-08-28 20:06'
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
- [x] #1 A pyproject.toml with a valid name/version and an authors list of bare strings still yields the correct name and version in ProjectIdentity
- [x] #2 authors written as a list of bare strings is either parsed (untagged enum accepting both PEP 621 table and string form) or skipped, without discarding the rest of [project]
- [x] #3 A type mismatch on any single [project] field degrades that field only; the others still populate
- [x] #4 The per-field failure still emits a tracing warn naming the offending field path
- [x] #5 units.rs read_workspace_members likewise tolerates a bad members/exclude element without returning an empty unit list
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
`RawPyproject.project` is now an untyped `toml::Table`, and each key is
projected independently by a new `project_field::<T>(project, key, path)`
helper. `RawProject` is deleted. On a type mismatch the helper warns with
`path`, `field = "project.<key>"`, the serde error, and
`recovery = "skip-field"`, then yields `None` for that key only -- every other
field still populates.

`authors` entries deserialise through a new untagged `RawAuthorEntry`:
`Table(RawAuthor)` (PEP 621), `Name(String)` (the Poetry / bare-string form,
already in the rendered `Name <email>` shape, passed through the same ERR-2
trim+drop), and `Unsupported(toml::Value)` for anything else, which is skipped
with a `recovery = "skip-author"` warn rather than failing the whole list.

units.rs: `members` / `exclude` are `Vec<RawGlob>` (untagged
`Pattern(String)` / `Unsupported(toml::Value)`), filtered by a `string_globs`
helper that warns per bad entry with
`field = "tool.uv.workspace.<members|exclude>"` and
`recovery = "skip-entry"`, so one non-string element no longer zeroes the unit
list.

Tests: `bare_string_authors_parse_and_keep_the_rest_of_the_project_table`
(AC#1/#2), `one_malformed_project_field_degrades_only_that_field` (AC#3/#4,
asserts the field path and recovery in the captured warn),
`unsupported_author_entry_is_skipped_not_fatal`, and
`non_string_workspace_glob_entry_does_not_zero_the_unit_list` (AC#5).

Scope note: a whole-file TOML syntax error, or a `[project]` that is not a
table at all, still falls back to the default identity -- that is the
documented TASK-0394 contract and is pinned by TASK-1756's new tests.
<!-- SECTION:NOTES:END -->
