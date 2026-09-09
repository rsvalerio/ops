---
id: TASK-2204
title: 'PATTERN-1: ops-about-java advertises a Homepage about field it can never fill, while the POM''s homepage <url> is emitted as the repository'
status: Done
assignee: []
created_date: '2026-09-08 07:19'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - pattern
dependencies: []
parent_task_id: 'TASK-2239'
modified_files:
  - extensions-java/about/src/lib.rs
  - extensions-java/about/src/maven/pom.rs
  - extensions-java/about/src/maven/mod.rs
  - extensions-java/about/src/gradle/mod.rs
priority: medium
ordinal: 117000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/lib.rs:88` (`java_about_fields`), `extensions-java/about/src/maven/pom.rs:~360` (`parse_top_level`), `extensions-java/about/src/maven/mod.rs:32`, `extensions-java/about/src/gradle/mod.rs:56`

**What**: `java_about_fields()` calls `ops_core::project_identity::insert_homepage_field`, so both the Maven and the Gradle About cards declare a `homepage` / "Homepage" field. Neither provider ever assigns `m.homepage` — the Maven `ParsedManifest::build` closure sets name/version/description/license/authors/repository only, and the Gradle one sets even fewer. `homepage` is therefore structurally always `null` for every Java project; `maven_provider_provide_shape` even asserts `result["homepage"].is_null()` as if that were the intended contract. The sibling stacks that insert the same field do populate it (`extensions-node/about/src/lib.rs:94`, `extensions-python/about/src/lib.rs:103`, `extensions-rust/about/src/identity/mod.rs:67`).

The data to fill it is being thrown at the wrong field: in `parse_top_level`, a **top-level** `<url>` is written into `PomData::scm_url` via `try_set_once(&mut data.scm_url, line, "<url>", "</url>")`, and `maven/mod.rs` then maps `pom.scm_url` to `m.repository`. In the Maven POM schema the top-level `<url>` is the *project homepage*; the repository URL is `<scm><url>`. So a POM carrying both a homepage and an SCM section is fine only because `<scm>` usually parses first (`try_set_once` is first-writer-wins), while a POM with only a top-level `<url>` reports its homepage as the project's repository and leaves Homepage blank.

**Why it matters**: every Java About card renders a permanently empty Homepage row, and projects whose POM declares only `<url>` get that URL mislabelled as the repository — a wrong value is worse than a missing one, and the git-remote fallback in `build_identity_value` is suppressed because `repository` is already `Some`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 PomData carries the top-level <url> in a field distinct from <scm><url> (e.g. project_url vs scm_url), and the Maven provider maps the former to ParsedManifest::homepage and the latter to repository
- [x] #2 A POM declaring only a top-level <url> yields homepage = that URL and leaves repository to the git-remote fallback; a test pins this
- [x] #3 A POM declaring both <url> and <scm><url> yields homepage from <url> and repository from <scm><url>, in either source order; a test pins this
- [x] #4 Either the Gradle provider populates homepage from an available source, or java_about_fields stops inserting the homepage field for the Gradle stack so no permanently-empty row is rendered
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed: PomData gained project_url (top-level <url>) distinct from scm_url (<scm><url>); parse_top_level writes the former. Maven provider maps project_url -> homepage and scm_url -> repository. java_about_fields split: maven_about_fields (inserts homepage) and gradle_about_fields (base only) — the Gradle card no longer declares a row it can never fill, pinned by gradle_provider_about_fields. Tests: parser-level parse_pom_top_level_url_is_the_project_homepage and parse_pom_url_and_scm_url_are_captured_independently_in_either_order; provider-level maven_provider_top_level_url_is_homepage_not_repository (AC #2, repository left to git fallback) and maven_provider_homepage_and_repository_come_from_distinct_elements (AC #3, both orders). parse_pom_duplicate_scm_opener_deterministic and parse_pom_scm_takes_precedence_over_url updated to the new field semantics (the latter folded into the either-order test). cargo test -p ops-about-java: 90 passed; clippy pedantic clean.
<!-- SECTION:NOTES:END -->
