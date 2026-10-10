---
id: TASK-2608
title: 'code-review-plan-wave54'
status: To Do
assignee: []
created_date: '2026-10-10 21:12'
updated_date: '2026-10-10 21:13'
labels:
  - code-review-wave
dependencies:
  - TASK-2450
  - TASK-2445
  - TASK-2475
  - TASK-2476
  - TASK-2479
  - TASK-2481
  - TASK-2495
  - TASK-2504
  - TASK-2525
  - TASK-2545
  - TASK-2546
  - TASK-2547
  - TASK-2552
  - TASK-2554
  - TASK-2555
  - TASK-2562
  - TASK-2576
  - TASK-2578
  - TASK-2585
  - TASK-2588
  - TASK-2590
modified_files:
  - crates/extension/Cargo.toml
  - crates/theme/Cargo.toml
  - extensions/config-checkers/Cargo.toml
  - extensions/create-review-tasks/Cargo.toml
  - extensions/git/Cargo.toml
  - extensions-go/about/Cargo.toml
  - extensions-java/about/Cargo.toml
  - extensions-node/about/Cargo.toml
  - extensions-python/about/Cargo.toml
  - extensions/run-before-commit/Cargo.toml
  - extensions/run-before-push/Cargo.toml
  - extensions-rust/cargo-toml/Cargo.toml
  - extensions-rust/loc/Cargo.toml
  - extensions-terraform/plan/Cargo.toml
  - extensions/text-fixers/Cargo.toml
  - extensions/tokei/Cargo.toml
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave54
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Single concern: bump every crate still on edition 2021 to 2024 and declare the inherited rust-version; all one-line Cargo.toml edits across crates
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2620 (extensions/create-review-tasks/Cargo.toml); TASK-2621 (extensions-node/about/Cargo.toml); TASK-2624 (extensions/git/Cargo.toml)
<!-- SECTION:NOTES:END -->
