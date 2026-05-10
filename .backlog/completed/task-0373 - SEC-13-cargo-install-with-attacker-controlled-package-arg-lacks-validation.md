---
id: TASK-0373
title: 'SEC-13: cargo install with attacker-controlled package arg lacks validation'
status: Done
assignee:
  - TASK-0419
created_date: '2026-04-26 09:37'
updated_date: '2026-04-27 10:55'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/install.rs:11`

**What**: install_cargo_tool(name, package) passes name and package directly into cargo install ... --bin name. While Command::args avoids shell interpolation, the values flow into a privileged operation; if either argument originates from a user-supplied config (ToolSpec) it should be validated.

**Why it matters**: Tool installs run with user privileges; an injected --config or --git argument (if name is ever an attacker-influenced string starting with --) would change install semantics.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Validate name and package against a regex (e.g. ^[A-Za-z0-9_.\\-]+$) before invocation; reject leading - to prevent flag injection
- [x] #2 Test coverage for rejection of names starting with - and other invalid characters
<!-- AC:END -->
