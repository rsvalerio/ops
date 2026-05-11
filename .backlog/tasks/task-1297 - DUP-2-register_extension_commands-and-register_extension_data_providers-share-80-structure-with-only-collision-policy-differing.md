---
id: TASK-1297
title: >-
  DUP-2: register_extension_commands and register_extension_data_providers share
  80% structure with only collision policy differing
status: To Do
assignee:
  - TASK-1304
created_date: '2026-05-11 16:19'
updated_date: '2026-05-11 16:48'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: \`crates/cli/src/registry/registration.rs:83\` (commands) and \`crates/cli/src/registry/registration.rs:149\` (data providers)

**What**: The two registration functions are structurally near-identical: both call \`snapshot_initial_owners\`, iterate \`extensions\`, register into a per-extension scratch registry, drain duplicate-insert audits via \`take_duplicate_inserts\` and warn, then iterate the local entries and classify each via the \`owners\` map, warning on cross-extension and pre-existing collisions. The only behavioural differences are:

1. Collision-resolution policy: commands use last-write-wins (\`owners.insert\` + always-register), data providers use first-write-wins (\`Entry::Vacant\` + register-only-if-new).
2. The shape of \`take_duplicate_inserts\` warn message (\"shadows the earlier within this extension\" vs. \"first-write-wins keeps the earlier registration\").

The structural duplication (DUP-2) means any future audit-pipeline change has to be made in two places. The asymmetric policies are intentional and well-documented in the module header — so the natural refactor is a generic helper parameterised by an \`InsertPolicy\` enum (or a closure) and a per-policy warn-message provider, with the two public functions becoming thin shells that pick the policy.

**Why it matters**: This is the source of TASK-1288 (register_extension_data_providers FN-1) and TASK-1280 (test-with-no-assertion) — the duplication makes each function long and tempts shortcuts in tests. Unifying the audit pipeline removes both downstream findings and prevents drift between the two paths (e.g. a future change to in-extension dedup wording landing on only one side).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Shared helper (e.g. register_extension_entries) drives both commands and data-provider registration, parameterised by an InsertPolicy enum that captures last-write-wins vs. first-write-wins
- [ ] #2 register_extension_commands and register_extension_data_providers shrink to <=20 lines each, with the asymmetric policy expressed at the call site (not by re-implementing the audit loop)
- [ ] #3 Cross-extension and pre-existing collision warn messages remain distinct per policy (the existing module-doc rationale stays accurate); existing tests in registry/tests.rs continue to pass with no message regressions
<!-- AC:END -->
