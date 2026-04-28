---
id: TASK-0482
title: 'PATTERN-1: extract_assignment requires = and misses Groovy bare-method form'
status: Done
assignee:
  - TASK-0531
created_date: '2026-04-28 05:48'
updated_date: '2026-04-28 07:25'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions-java/about/src/gradle.rs:125-133

**What**: extract_assignment requires an = between the key and value, so the Groovy-DSL form 'description "My project"' (a method-call-as-property, idiomatic in build.gradle) returns None. Only the Kotlin DSL form 'description = "..."' and the explicit-equals Groovy form succeed.

**Why it matters**: Among real-world Spring/Apache Groovy build.gradle files the bare-method form is at least as common; result is GradleBuild.description is None for many genuine projects, masking a project description in the about card.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Accept the bare-method form (no =, possibly more whitespace) in addition to the current = form, while still rejecting random lines that merely begin with the key
- [ ] #2 Add tests for the bare-method form returning Some(value), and a test that descriptionTask {} does not match
<!-- AC:END -->
