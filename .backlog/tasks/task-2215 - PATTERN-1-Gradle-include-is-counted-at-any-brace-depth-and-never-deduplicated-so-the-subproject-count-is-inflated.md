---
id: TASK-2215
title: >-
  PATTERN-1: Gradle 'include' is counted at any brace depth and never
  deduplicated, so the subproject count is inflated
status: Done
assignee:
  - TASK-2238
created_date: '2026-09-08 07:21'
updated_date: '2026-09-08 16:57'
labels:
  - code-review-rust
  - pattern
dependencies: []
modified_files:
  - extensions-java/about/src/gradle/mod.rs
priority: low
ordinal: 125000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/gradle/mod.rs:83-105` (`parse_gradle_settings`), `extensions-java/about/src/gradle/mod.rs:174-197` (`parse_include_line`)

**What**: inside `parse_gradle_settings`'s `scan` closure, `rootProject.name` is accepted only at `depth == 0` (deliberately, per the CL-3 / TASK-1733 note), and `parse_gradle_build` applies the same depth gate to `description`. `parse_include_line` is called with no depth gate at all, and `includes` is a plain `Vec<String>` with no deduplication. Consequences:

- an `include` nested in a conditional or a lifecycle block (`if (file("legacy").exists()) { include(":legacy") }`, `gradle.beforeSettings { … }`) is counted unconditionally, even though its containing block may never execute;
- the same project declared twice — `include ':app'` and a later `include 'app'`, or a generated settings file that repeats an entry — counts twice, because the two spellings are distinct strings and nothing collapses them. Gradle itself treats `include` as idempotent on the project path;
- leading-colon Gradle project paths are stored verbatim, so `":a"` and `"a"` are two entries for one subproject.

The count reaches the user directly: `module_count = (!includes.is_empty()).then_some(includes.len())` with `module_label = "subprojects"`.

**Why it matters**: the About card reports a subproject count that does not match the project's actual layout, and the divergence is systematically upward — duplicates and conditionally-guarded includes both inflate. The identical concern on the Maven side (`<module>` entries) is bounded by XML structure; the Gradle DSL is not.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Include entries are normalised (leading ':' stripped, path separators canonicalised) and deduplicated before the count is taken
- [ ] #2 A settings.gradle declaring include ':app' and include 'app' yields module_count = 1; a test pins this
- [ ] #3 The treatment of an include nested inside a block is decided explicitly — either depth-gated like rootProject.name, or counted with the rationale recorded — and a test covers the chosen behaviour
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in wave TASK-2238: includes are depth-gated like rootProject.name (AC#3 — chosen treatment, rationale in parse_gradle_settings docs); entries dedup on a canonical path key (leading ":" stripped, "/" and "\\" canonicalised to ":") with first-writer-wins raw spelling kept (AC#1); ":app"+"app" collapses to one (AC#2, test parse_gradle_settings_dedupes_colon_and_bare_include_spellings); nested-block and separator-spelling tests added.
<!-- SECTION:NOTES:END -->
