---
id: TASK-0043
title: 'RS-2: rand 0.8.5/0.9.2 advisory RUSTSEC-2026-0097 — unsound with custom logger'
status: Done
assignee: []
created_date: '2026-04-14 20:16'
updated_date: '2026-04-15 09:56'
labels:
  - rust-security
  - dependency
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
cargo audit reports RUSTSEC-2026-0097 for both rand 0.8.5 (transitive via tera, rust_decimal/duckdb, phf_generator) and rand 0.9.2 (transitive via quinn-proto). The advisory flags unsoundness when using rand::rng() with a custom logger. This project does not use a custom logger with rand, so there is no exploitable path today. However, updating to a patched version when available removes the theoretical risk. OWASP: A06 (Vulnerable Components). SEC-27.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 rand dependency updated to a version that resolves RUSTSEC-2026-0097, or advisory confirmed as not-applicable and documented
<!-- AC:END -->
