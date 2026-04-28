---
id: TASK-0494
title: 'PERF-2: java_about_fields rebuilds the field list on every about_fields() call'
status: Done
assignee:
  - TASK-0531
created_date: '2026-04-28 06:09'
updated_date: '2026-04-28 07:25'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/lib.rs:72-76`

**What**: java_about_fields() calls base_about_fields() then insert_homepage_field() each time it is invoked. Both Maven and Gradle providers call this from the per-invocation about_fields() trait method, so each provide cycle rebuilds the same Vec<AboutFieldDef>.

**Why it matters**: The result is invariant for the lifetime of the process. Caching it in a OnceLock<Vec<AboutFieldDef>> avoids the repeated Vec allocation and any per-field cloning inside insert_homepage_field.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 java_about_fields backed by a OnceLock or equivalent so the Vec is built once
- [ ] #2 Maven and Gradle providers' about_fields() return clones from the cached value (or a borrowed slice if the trait permits)
- [ ] #3 Existing maven_provider_about_fields / gradle_provider_about_fields tests still pass
<!-- AC:END -->
