---
id: TASK-2204
title: >-
  PATTERN-1: ops-about-java advertises a Homepage about field it can never fill,
  while the POM's homepage <url> is emitted as the repository
status: To Do
assignee:
  - TASK-2239
created_date: '2026-09-08 07:19'
updated_date: '2026-09-08 10:55'
labels:
  - code-review-rust
  - pattern
dependencies: []
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
- [ ] #1 PomData carries the top-level <url> in a field distinct from <scm><url> (e.g. project_url vs scm_url), and the Maven provider maps the former to ParsedManifest::homepage and the latter to repository
- [ ] #2 A POM declaring only a top-level <url> yields homepage = that URL and leaves repository to the git-remote fallback; a test pins this
- [ ] #3 A POM declaring both <url> and <scm><url> yields homepage from <url> and repository from <scm><url>, in either source order; a test pins this
- [ ] #4 Either the Gradle provider populates homepage from an available source, or java_about_fields stops inserting the homepage field for the Gradle stack so no permanently-empty row is rendered
<!-- AC:END -->
