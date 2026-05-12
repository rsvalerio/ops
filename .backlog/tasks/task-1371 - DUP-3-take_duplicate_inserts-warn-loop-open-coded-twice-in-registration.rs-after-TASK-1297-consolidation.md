---
id: TASK-1371
title: >-
  DUP-3: take_duplicate_inserts() warn loop open-coded twice in registration.rs
  after TASK-1297 consolidation
status: Done
assignee:
  - TASK-1383
created_date: '2026-05-12 21:35'
updated_date: '2026-05-12 23:16'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/registration.rs:200-209, 238-247`

**What**: TASK-1297 (Done) collapsed the *cross-extension* collision pipeline into the shared `classify_and_warn_collision` helper, but the *in-extension* duplicate-emit pipeline is still open-coded in both `register_extension_commands` and `register_extension_data_providers`. Each function has its own near-identical 10-line block:

```rust
for dup in local.take_duplicate_inserts() {
    tracing::warn!(
        target: "ops::registry",
        kind = <AUDIT>.kind_field,
        key = %dup,
        extension = ext.name(),
        "{}",
        <AUDIT>.in_ext_duplicate,
    );
}
```

Only the `<AUDIT>` constant (`COMMAND_AUDIT` vs `DATA_PROVIDER_AUDIT`) differs — exactly the same parameterisation the cross-extension audit already uses. A future audit-policy change (extra field, different target, structured event) requires touching two sites; the symmetry comment at the top of the module promises the opposite.

**Why it matters**: DUP-3 — same shape, two sites, parameterised only by a constant the module already plumbs everywhere else. Risk: drift between the command and data-provider in-extension warns (different `target`, different field set, different message phrasing) the next time either is touched. Low severity because the symmetry is preserved *today*, but the consolidation TASK-1297 explicitly aimed for is not finished.

**Candidates to inspect**:
- `crates/cli/src/registry/registration.rs:200-209` — `register_extension_commands` in-ext-duplicate loop
- `crates/cli/src/registry/registration.rs:238-247` — `register_extension_data_providers` in-ext-duplicate loop
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 the in-extension take_duplicate_inserts() warn block is extracted into a shared helper (or inlined into classify_and_warn_collision) so adding a structured field touches one site
- [ ] #2 no behavioural change: the existing target/kind/key/extension fields are still emitted for both registries with the policy-appropriate message
<!-- AC:END -->
