---
id: TASK-1078
title: >-
  PATTERN-1: tools probe parse_active_toolchain rejects any token containing
  ':', blocking rustup-link / Windows path toolchains
status: Done
assignee: []
created_date: '2026-05-07 21:20'
updated_date: '2026-05-07 23:18'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe.rs:103`

**What**: `parse_active_toolchain` rejects tokens containing `:`. A custom toolchain registered via `rustup toolchain link` may contain `:` in its name, and on Windows `rustup show active-toolchain` may print `C:\path\...` shapes. The blanket reject downgrades to "no active toolchain" and `install_tool` then bails with "could not determine active toolchain".

**Why it matters**: Edge-case correctness for developers with linked / path toolchains; cross-platform regression on Windows.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Reject only tokens that match rustup's diagnostic prefixes (error:/warning:/info:/note:) by full-word check, not blanket contains
- [ ] #2 Accept other ':'-containing tokens
- [ ] #3 Test linked-toolchain shape and a Windows-style path token
<!-- AC:END -->
