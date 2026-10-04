---
id: TASK-2364
title: 'CL-3: Maven pom parser treats every element outside a few skipped containers as a project-level scalar, so dependency, repository and profile children leak into version, url and modules'
status: To Do
assignee: []
created_date: '2026-10-04 14:10'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - CL
dependencies: []
parent_task_id: 'TASK-2424'
modified_files:
  - extensions-java/about/src/maven/pom.rs
priority: medium
ordinal: 1000
dedup_key: 'CL-3:extensions-java/about/src/maven/pom.rs:match_section_open'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:411-445` (`match_section_open`, `SKIP_SECTIONS`), `pom.rs:447-458` (`parse_top_level`)

**What**: The parser assumes that any line seen in `PomSection::TopLevel` that is not a known section opener is a direct child of `<project>`. `SKIP_SECTIONS` only lists organization, parent, issueManagement, ciManagement and distributionManagement. Containers that routinely carry `<version>`, `<url>`, `<name>`, `<description>` and `<modules>` children are not skipped: `<dependencies>`, `<dependencyManagement>`, `<build>`/`<plugins>`, `<repositories>`/`<pluginRepositories>`, `<profiles>`, `<reporting>`, `<properties>`. Consequences (first-writer-wins `try_set_once` makes them stick):
- A POM that inherits its version from `<parent>` (no own `<version>`) reports the first `<dependency>`/`<plugin>` `<version>` as the project version.
- A POM with no top-level `<url>` reports the first `<repository><url>` as the project homepage.
- `<modules>` nested in `<profiles><profile>` is matched as the top-level `<modules>` section, inflating the module count and units (profile modules may never build).
- `<name>`/`<description>` inside a `<plugin>` or `<dependency>` can fill the project name/description when the project omits them.

**Why it matters**: The About card shows a confidently wrong version/homepage/module count for ordinary multi-module and parent-inheriting POMs, which is the common real-world shape; the crate docs list known limits but not this one. The existing depth gating in the Gradle parser (TASK-2215) shows the intended policy.

Candidates to verify: add the container tags above to `SKIP_SECTIONS` (or track element depth under `<project>` so only depth-1 children are matched).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A POM with a <parent> and no own <version> reports no version (or the parent's by explicit policy), never a dependency or plugin version
- [ ] #2 A <repository><url> never populates the project homepage
- [ ] #3 <modules> inside <profiles> is not counted in module_count or project_units
- [ ] #4 Regression tests cover dependencies, dependencyManagement, build/plugins, repositories and profiles
<!-- AC:END -->
