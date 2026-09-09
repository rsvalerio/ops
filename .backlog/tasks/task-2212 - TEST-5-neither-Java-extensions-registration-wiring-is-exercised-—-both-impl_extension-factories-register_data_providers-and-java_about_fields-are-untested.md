---
id: TASK-2212
title: 'TEST-5: neither Java extension''s registration wiring is exercised — both impl_extension! factories, register_data_providers and java_about_fields are untested'
status: Done
assignee: []
created_date: '2026-09-08 07:20'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - test
dependencies: []
parent_task_id: 'TASK-2240'
modified_files:
  - extensions-java/about/src/lib.rs
priority: medium
ordinal: 123000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/lib.rs` (whole file — no `#[cfg(test)]` module)

**What**: `lib.rs` has no tests at all, and a repo-wide grep for `AboutMavenExtension` / `AboutGradleExtension` finds them only in `lib.rs` itself and in `crates/cli/src/main.rs`'s `extern crate ops_about_java;`. Untested surface:

- both `ops_extension::impl_extension!` invocations, including `MAVEN_ABOUT_FACTORY` / `GRADLE_ABOUT_FACTORY` and their `linkme` distributed-slice registration;
- the two `register_data_providers` closures — nothing asserts that `"project_identity"` actually lands in the registry, nor that the Maven extension registers the Maven provider and the Gradle extension the Gradle one (the two closures are near-identical and a copy-paste swap would be invisible);
- the `stack: Some(Stack::JavaMaven)` / `Some(Stack::JavaGradle)` gating — with both extensions in one crate and both `stack-java-maven` and `stack-java-gradle` cargo features pointing at it (`crates/cli/Cargo.toml:22-23`), a wrong stack tag would register the Gradle provider for a Maven project;
- `java_about_fields()`, whose `OnceLock` + `insert_homepage_field` composition is never asserted (no test that `homepage` is present, or that it sits before `coverage`).

The provider `provide`/`name`/`about_fields` methods themselves *are* covered (`maven/mod.rs` tests, `gradle/tests.rs:588-660`) — the gap is strictly the extension/registration layer above them. `ops-extension`'s `test-support` feature is already a dev-dependency of this crate, so the harness for this exists.

Twin: TASK-2184 files the same gap for the Go extension's provider registration.

**Why it matters**: the wiring is the only thing that connects a working parser to the CLI. A mis-tagged stack or a swapped provider in a `register_data_providers` closure compiles cleanly, passes every existing test, and ships an About card built from the wrong stack's parser.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test builds each extension through its factory and asserts name, shortname, description, types and stack
- [ ] #2 A test runs each register_data_providers closure against a test DataRegistry and asserts the 'project_identity' provider present is the matching stack's provider (Maven for AboutMavenExtension, Gradle for AboutGradleExtension), not merely that some provider registered
- [ ] #3 A test asserts java_about_fields() contains a 'homepage' field positioned immediately before 'coverage'
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed in wave TASK-2240. lib.rs gained a tests module (previously none): factories_build_extensions_with_declared_metadata (AC1), each_extension_registers_its_own_stack_identity_provider (AC2, provider identified by stack_detail Maven/Gradle over real fixtures), java_about_fields_places_homepage_immediately_before_coverage (AC3).
<!-- SECTION:NOTES:END -->
