---
id: TASK-1608
title: >-
  ERR-2: validate_cargo_tool_arg over-rejects rustup version-pinned toolchain
  identifiers containing '.'
status: Done
assignee:
  - TASK-1638
created_date: '2026-05-22 06:46'
updated_date: '2026-05-22 13:20'
labels:
  - code-review-rust
  - error-handling
  - security
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: \`extensions-rust/tools/src/install.rs:37-60, :141\`

**What**: \`install_rustup_component_with_timeout\` validates the toolchain argument with \`validate_cargo_tool_arg(toolchain, "rustup toolchain")\`. After TASK-1199 the validator's allow-set is \`[A-Za-z0-9][A-Za-z0-9_-]*\` — \`.\` is explicitly rejected. The validator's own doc-comment acknowledges this and states it deliberately rejects \`1.70.0-x86_64-apple-darwin\`.

That shape is, however, a perfectly valid rustup toolchain identifier (version-pinned channel + host triple). \`parse_active_toolchain\` (extensions-rust/tools/src/probe/rustup.rs:48) explicitly accepts it: it only requires the token to contain one of \`-./:\`. So:

1. User pins their workspace to e.g. \`1.70.0-x86_64-apple-darwin\` (via rust-toolchain.toml or \`rustup default 1.70.0\`).
2. \`get_active_toolchain()\` returns \`Some("1.70.0-x86_64-apple-darwin")\`.
3. \`install_tool\` calls \`install_rustup_component(component, &toolchain)\`.
4. \`validate_cargo_tool_arg\` bails: \`rustup toolchain "1.70.0-x86_64-apple-darwin" contains invalid character '.'\`.

The same validator over-restricts \`spec.rustup_component()\` values too — none of the in-tree components carry \`.\`, but the contract is "rustup component / toolchain", and rustup grammars don't share crates.io's no-dot constraint.

**Why it matters**: Blocks \`ops tools install\` for any user on a version-pinned rustup toolchain (a very common pattern for reproducible builds). The error message points at the validator's policy rather than the actual cargo/rustup invocation, sending the operator chasing a phantom policy violation.

**Suggested fix**: Either (a) introduce a separate \`validate_rustup_toolchain\` that allows \`.\` (rustup's grammar is broader than crates.io's), or (b) carve out a \`{allow_dot: bool}\` flag on \`validate_cargo_tool_arg\` and pass it for the toolchain/component label call sites. Keep the no-leading-\`-\` guard either way (still defense-in-depth against arg-as-flag).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 validate_cargo_tool_arg (or its toolchain-specific successor) accepts '1.70.0-x86_64-apple-darwin' as a valid toolchain
- [x] #2 Leading '-' is still rejected on toolchain/component arguments (defense-in-depth against flag injection)
- [x] #3 A unit test pins the version-pinned-toolchain accept case and the leading-dash reject case
- [x] #4 install_tool succeeds end-to-end against an active toolchain whose name contains '.'
<!-- AC:END -->
