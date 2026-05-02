---
id: TASK-0872
title: >-
  TRAIT-4: Config derives Default but Default is documented as
  'tests-and-extensions only'
status: Triage
assignee: []
created_date: '2026-05-02 09:22'
labels:
  - code-review-rust
  - traits
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/mod.rs:46-66`

**What**: The doc comment says Config::default is "intended for tests and downstream extension wiring" but the type is a pub workspace type with derive(Default), so any caller can construct one. This is the same TRAIT-4 nuance the project already files (e.g. backlog task-0750).

**Why it matters**: A buggy CLI path can silently fall back to Config::default() (no commands, default theme strings, etc.) instead of going through load_config_or_default, which is the supported degradation path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Hide the Default impl behind cfg(any(test, feature = test-support)), or rename to Config::empty() + remove derive(Default)
- [ ] #2 Audit production call sites (config::Config::default()) and migrate to load_config_or_default
- [ ] #3 Document the rationale on the type
<!-- AC:END -->
