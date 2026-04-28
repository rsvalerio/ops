---
id: TASK-0484
title: >-
  DUP-1: Five about identity providers repeat the same
  parse-and-build_identity_value scaffold
status: To Do
assignee:
  - TASK-0534
created_date: '2026-04-28 05:49'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions-go/about/src/lib.rs (and analogues in extensions-python/about, extensions-node/about, extensions-java/about)

**What**: parse_go_work in lib.rs is a one-line wrapper around go_work::parse_use_dirs whose only purpose is to repackage the Vec into a GoWork { use_dirs } struct that has no other field. Separately, the four refactored providers (Go/Python/Node/Maven/Gradle identity) each open with the same shape: 'let cwd = ctx.working_directory.clone(); let X = parse_X(&cwd).unwrap_or_default(); ... build_identity_value(ParsedManifest{...})'.

**Why it matters**: The GoWork newtype-around-Vec adds an indirection with no semantic value. The repeated provider boilerplate is prime territory for a 'provide_identity_from_manifest(ctx, parser)' helper in ops_about::identity so each stack contributes only the parser.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Inline the GoWork wrapper (use the Vec from parse_use_dirs directly in GoIdentityProvider::provide)
- [ ] #2 Sketch a shared helper in ops_about::identity that takes a closure (Path -> ParsedManifest) and returns Result<Value, DataProviderError>, then migrate at least one provider as proof-of-concept
<!-- AC:END -->
