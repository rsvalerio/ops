---
id: TASK-0387
title: >-
  DUP-2: Identity providers across stacks duplicate the same
  parse-then-build-ProjectIdentity skeleton
status: Done
assignee:
  - TASK-0417
created_date: '2026-04-26 09:39'
updated_date: '2026-04-27 20:11'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/lib.rs:55` (also Python, Go, Maven, Gradle providers)

**What**: NodeIdentityProvider::provide, PythonIdentityProvider::provide (extensions-python:56), GoIdentityProvider::provide (extensions-go:49), MavenIdentityProvider::provide (extensions-java/maven.rs:22), and GradleIdentityProvider::provide (extensions-java/gradle.rs:22) follow an almost identical structure.

**Why it matters**: Five copies drift independently — Python and Maven do not call insert_homepage_field consistently, Maven swallows homepage entirely, Gradle drops authors/license. Bug fixes need to be applied 5 times.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Introduce a shared IdentityBuilder/helper in ops_about that takes a stack-agnostic ParsedManifest and produces ProjectIdentity + serde_json::Value, including git-fallback repository resolution
- [ ] #2 Each *IdentityProvider::provide reduced to call its parser, build canonical ParsedManifest, call shared helper. Each shrinks to <30 lines and behavioural divergences are eliminated
<!-- AC:END -->
