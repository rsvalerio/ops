---
id: TASK-2344
title: 'Scaffold and drift-check mise.toml from the Rust foundation templates'
status: Triage
assignee: []
created_date: '2026-09-29 18:03'
updated_date: '2026-09-29 19:28'
labels:
  - foundation
dependencies: []
modified_files:
  - extensions-rust/foundation/templates/mise.toml
  - mise.toml
  - docs/foundation.md
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
forge TASK-0050 (2026-09-29) made mise the one install method for the tools a Rust repo's gates and pipelines run, on CI and laptops; forge docs/foundation.md "Pipeline tools" lists them with their mise.toml entries. Nothing writes or checks a repo's mise.toml today, so laptop pins drift from CI.

Add a mise.toml template to extensions-rust/foundation/templates/ carrying the gate tools at the versions forge's mise.toml pins: [tool_alias] ops = "github:rsvalerio/ops", cargo-nextest = "github:nextest-rs/nextest", cargo-machete = "github:bnjbvr/cargo-machete", cargo-edit = "cargo:cargo-edit"; [tools] ops, cargo-deny, cargo-machete, cargo-nextest (version_prefix = "cargo-nextest-"), cargo-edit, trivy, and "aqua:taiki-e/cargo-llvm-cov". `ops init --rust` writes it when missing; `ops init --rust --check` compares it semantically like the other templates (a tool the template pins must keep its version; tools the repo adds are its own; a deliberate divergence goes under [foundation.waivers]).

**Origin**: forge TASK-0050 AC#4.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The template ships in ops and ops init --rust writes mise.toml when missing
- [ ] #2 ops init --rust --check reports a drifted or missing tool pin and honours [foundation.waivers]
- [ ] #3 ops's own mise.toml matches the template
- [ ] #4 ops docs/foundation.md lists the file
- [ ] #5 The template pins rust (with rustfmt,clippy components) and ops at or above 0.77.0, the first check-only verify (forge rust-ci relies on callers pinning it)

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-09-29: forge rust-ci now installs from the caller's mise.toml (forge #21), so this template is what keeps callers' CI pins current.
<!-- SECTION:NOTES:END -->
