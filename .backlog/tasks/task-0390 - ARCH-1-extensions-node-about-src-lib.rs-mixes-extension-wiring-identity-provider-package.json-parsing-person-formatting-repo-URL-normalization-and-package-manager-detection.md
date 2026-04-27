---
id: TASK-0390
title: >-
  ARCH-1: extensions-node/about/src/lib.rs mixes extension wiring, identity
  provider, package.json parsing, person formatting, repo URL normalization, and
  package-manager detection
status: To Do
assignee:
  - TASK-0417
created_date: '2026-04-26 09:40'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/lib.rs:1`

**What**: 454 lines containing: extension macro wiring, the DataProvider impl, four Deserialize enums, parse_package_json, format_person, normalize_repo_url, detect_package_manager, plus large test module.

**Why it matters**: God-module pattern: changes to npm shorthand normalization, packageManager semantics, or workspace detection all touch the same file.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Move parse_package_json + RawPackage/LicenseField/RepositoryField/PersonField/Engines/format_person/normalize_repo_url into package_json.rs; move detect_package_manager into package_manager.rs
- [ ] #2 After the split, lib.rs is <120 lines containing only the extension macro, the NodeIdentityProvider impl, and re-exports
<!-- AC:END -->
