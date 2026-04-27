---
id: TASK-0397
title: 'TEST-12: Maven parse_pom_basic and maven_provider_provide_success overlap'
status: To Do
assignee:
  - TASK-0417
created_date: '2026-04-26 09:40'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven.rs:257`

**What**: parse_pom_basic exercises every parser branch. maven_provider_provide_success (line 392) and maven_provider_provide_with_modules (line 441) re-test artifactId/version/description/modules through the provider, duplicating coverage of the parser without exercising new provider-specific behavior.

**Why it matters**: Test redundancy adds maintenance cost without catching new bugs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Trim provider-level tests to cases not covered by parser tests: empty-modules-yields-None module_count, homepage absent in JSON output, stack_detail==Maven always, name-fallback to dir
- [ ] #2 Keep parse_pom_basic as the single full-coverage parser test
<!-- AC:END -->
