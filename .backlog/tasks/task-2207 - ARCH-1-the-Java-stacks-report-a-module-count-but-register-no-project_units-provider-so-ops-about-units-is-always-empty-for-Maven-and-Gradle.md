---
id: TASK-2207
title: 'ARCH-1: the Java stacks report a module count but register no project_units provider, so ''ops about units'' is always empty for Maven and Gradle'
status: Done
assignee: []
created_date: '2026-09-08 07:20'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - architecture
dependencies: []
parent_task_id: 'TASK-2239'
modified_files:
  - extensions-java/about/src/lib.rs
  - extensions-java/about/src/maven/mod.rs
  - extensions-java/about/src/gradle/mod.rs
priority: medium
ordinal: 119000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/lib.rs:44-84`, `extensions-java/about/src/maven/mod.rs:32-42`, `extensions-java/about/src/gradle/mod.rs:56-66`

**What**: both Java extensions declare `data_provider_name: Some("project_identity")` and register only `MavenIdentityProvider` / `GradleIdentityProvider`. Every other stack extension that ships an About card also registers a `project_units` provider — `extensions-rust/about/src/units.rs`, `extensions-go/about/src/modules.rs`, `extensions-node/about/src/units.rs`, `extensions-python/about/src/units.rs` (all `PROVIDER_NAME = "project_units"`). `extensions/about/src/units.rs::run_about_units_with` calls `load_or_default(.., PROJECT_UNITS_PROVIDER)` and prints `No project units found.` when the stack registers nothing.

Meanwhile the identity providers do publish a count and a label: Maven sets `module_label = "modules"` with `module_count = pom.modules.len()`, Gradle sets `module_label = "subprojects"` with `module_count = includes.len()`. So the About card claims "N modules" while the units subpage, reachable from the same command surface, insists there are none.

Cross-reference: the Go twin of this divergence — a module count that disagrees with the units list — is TASK-2178. This is the more basic version of the same defect: here the units list does not exist at all.

**Why it matters**: a user who sees "12 subprojects" on the About card and then opens the units page gets "No project units found", with nothing distinguishing "this stack was never wired up" from "this project genuinely has no units". Either the count or the page is lying, and both are stack-visible surfaces.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Either ops-about-java registers a project_units provider whose unit list matches the module_count published by the identity provider (Maven <module> entries, Gradle include entries), or the gap is closed the other way and the reason is recorded
- [x] #2 If a units provider is added, a test asserts units.len() equals the identity module_count for the same fixture project, for both Maven and Gradle
- [x] #3 The behaviour on a Java project with no modules/includes is distinguishable in output from an unwired stack
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed: both Java extensions now register a project_units provider. MavenUnitsProvider lists one ProjectUnit per <modules><module> (child pom read for name/version when in-tree; out-of-tree/absolute/.. entries are listed for count parity but never read, mirroring the Go stack's SEC-14 use-directive policy). GradleUnitsProvider lists one unit per include, normalising :a:b project paths to a/b for enrichment. AC #2 pinned by maven_units_length_equals_identity_module_count and gradle_units_length_equals_identity_module_count (units.len() == identity module_count on the same fixture). AC #3: extensions/about run_about_units_with now probes the registry — an unregistered stack prints "No units provider is registered for this stack." while a registered provider with an empty list keeps "No project units found."; the empty-registry test was updated and a registered-empty test added. cargo test -p ops-about-java -p ops-about: 226 passed; clippy pedantic clean.
<!-- SECTION:NOTES:END -->
