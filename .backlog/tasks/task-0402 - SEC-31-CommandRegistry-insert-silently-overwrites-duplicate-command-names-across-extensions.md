---
id: TASK-0402
title: >-
  SEC-31: CommandRegistry insert silently overwrites duplicate command names
  across extensions
status: To Do
assignee:
  - TASK-0420
created_date: '2026-04-26 09:52'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/extension.rs:55` (type alias `CommandRegistry = IndexMap<CommandId, CommandSpec>`); call sites: `crates/cli/src/registry.rs:117-119` (`register_extension_commands`), `crates/runner/src/command/mod.rs:189-197` (`CommandRunner::register_commands`)

**What**: `CommandRegistry` is `IndexMap<CommandId, CommandSpec>`. Both registration paths call `.insert()` on it for every extension-provided command. If two extensions register the same command id (e.g. both an `about` extension and a stack extension contribute a `coverage` command), the second insertion silently replaces the first with no warning, no debug log, and no error.

**Why it matters**: This is the same defect already filed against `DataRegistry::register` in TASK-0350 (SEC-31). Extensions are linked statically via `EXTENSION_REGISTRY` distributed slice — duplicate names can collide on a feature-flag combination the developer never tested locally, and the user observes a silently-shadowed command without any log signal to diagnose. For commands the impact is higher than for data providers because end-users invoke them by name and any disagreement between `extension list` output and what actually runs is a confusion / supply-chain footgun.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 register_extension_commands and CommandRunner::register_commands detect duplicate keys before insert and either warn (preferred) or return an error
- [ ] #2 Behaviour matches whatever is decided for DataRegistry under TASK-0350 so the two registries stay symmetric
- [ ] #3 Test added that registers two extensions with overlapping command names and asserts the chosen behaviour
<!-- AC:END -->
