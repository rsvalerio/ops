---
id: TASK-0041
title: >-
  CQ-10: extensions-java about/src/lib.rs mixes Maven and Gradle parsers in
  998-line god module
status: Done
assignee: []
created_date: '2026-04-14 20:14'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-quality
  - ARCH-1
  - ARCH-3
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions-java/about/src/lib.rs contains two entirely separate build-system implementations: Maven (lines 12-317, pom.xml parsing) and Gradle (lines 320-494, settings.gradle/gradle.properties/build.gradle parsing). These are independent concerns that share only java_about_fields() and dir_name(). At 998 lines the file exceeds the ARCH-1 500-line god-module threshold and violates ARCH-3 (organize by concern). Note: CQ-5 separately covers parse_pom_xml complexity; this finding addresses the architectural concern.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Maven and Gradle implementations split into separate submodules (e.g. maven.rs + gradle.rs) with shared helpers in lib.rs or a common.rs module
- [ ] #2 lib.rs reduced to module declarations, shared types, and extension registration — under 100 lines
<!-- AC:END -->
