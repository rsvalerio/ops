---
id: TASK-0683
title: >-
  DUP-1: Maven parse_pom_xml does not route through
  manifest_io::read_optional_text
status: Done
assignee:
  - TASK-0736
created_date: '2026-04-30 05:15'
updated_date: '2026-04-30 08:03'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:74-88`

**What**: Maven's `parse_pom_xml` open-codes the `read_to_string` + `NotFound`/log-debug fallback that every sister parser (go_mod, go_work, package_json, pyproject, units variants) routes through `ops_about::manifest_io::read_optional_text`.

**Why it matters**: manifest_io.rs was created precisely to centralise this policy after a prior copy drifted (TASK-0467/0622); leaving Maven on its own copy means the next policy tweak (e.g. `kind`-tagged log fields, or upgrading severity) silently skips it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 parse_pom_xml calls ops_about::manifest_io::read_optional_text(&path, "pom.xml") and removes the local match
- [x] #2 Existing tests (parse_pom_missing_file, parse_pom_basic) keep passing
<!-- AC:END -->
