---
id: TASK-2207
title: >-
  ARCH-1: the Java stacks report a module count but register no project_units
  provider, so 'ops about units' is always empty for Maven and Gradle
status: To Do
assignee:
  - TASK-2239
created_date: '2026-09-08 07:20'
updated_date: '2026-09-08 10:55'
labels:
  - code-review-rust
  - architecture
dependencies: []
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
- [ ] #1 Either ops-about-java registers a project_units provider whose unit list matches the module_count published by the identity provider (Maven <module> entries, Gradle include entries), or the gap is closed the other way and the reason is recorded
- [ ] #2 If a units provider is added, a test asserts units.len() equals the identity module_count for the same fixture project, for both Maven and Gradle
- [ ] #3 The behaviour on a Java project with no modules/includes is distinguishable in output from an unwired stack
<!-- AC:END -->
