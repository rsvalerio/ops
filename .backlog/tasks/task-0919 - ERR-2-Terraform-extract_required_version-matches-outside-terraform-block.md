---
id: TASK-0919
title: 'ERR-2: Terraform extract_required_version matches outside terraform block'
status: Done
assignee: []
created_date: '2026-05-02 10:12'
updated_date: '2026-05-02 15:00'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:99`

**What**: extract_required_version scans every non-comment line for a leading `required_version =` token without verifying it appears inside a `terraform { ... }` block. A `module` block or a custom variable named required_version (legal HCL) yields a spurious match, and the first match wins.

**Why it matters**: Producers occasionally place required_version inside provider/module blocks or custom locals; the About card then advertises a stack version that is not the projects terraform constraint.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Parser tracks a minimal block-state machine and only accepts required_version inside a terraform block
- [x] #2 Test covers a .tf file where a non-terraform block declares required_version and asserts it is ignored
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added a minimal block-stack state machine to extract_required_version: only accept the key when block_stack == ["terraform"] (depth 1 inside terraform { }). block_open_ident parses HCL block openers `<ident> [labels…] {` and rejects assignment-shaped lines (`x = {`). Three new tests pin: ignores_non_terraform_blocks, returns_none_when_only_inside_non_terraform_block, rejects_nested_inside_terraform. Existing trailing-comment / quote-strict / cap tests were updated to wrap their content in `terraform { … }` to satisfy the new contract.
<!-- SECTION:NOTES:END -->
