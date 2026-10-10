---
id: TASK-2629
title: 'READ-13: residual RULE/TASK provenance tags across extensions-rust/about item and test docs'
status: Triage
assignee: []
created_date: '2026-10-10 21:38'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions-rust/about/src/coverage_provider.rs
  - extensions-rust/about/src/deps_provider.rs
  - extensions-rust/about/src/identity/metrics.rs
  - extensions-rust/about/src/identity/mod.rs
  - extensions-rust/about/src/manifest.rs
  - extensions-rust/about/src/workspace_root_cache.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: multiple — see the per-file candidate list below.

**What**: After wave TASK-2614 cleaned the seven member sites (journey narration in coverage_provider, deps_provider, identity/{metrics,mod,resolver}, manifest, workspace_root_cache), `extensions-rust/about/src` still carries RULE-ID / TASK-XXXX provenance tag prefixes on item and test docs. Candidate sites (line numbers from the pre-cleanup tree, shifts possible):

- `identity/mod.rs`: 72, 222, 308
- `identity/metrics.rs`: 56, 122, 136
- `manifest.rs`: 20, 26, 28, 40, 45, 59, 78, 107, 127-129, 137, 143, 169, 185, 207, 209, 211, 220, 238, 275, 305, 307, 314, 370, 430, 472, 507, 581
- `coverage_provider.rs`: 17, 24, 32, 54, 60, 78, 110, 118, 157, 164, 174, 177, 181, 185, 220, 236, 280, 329, 339, 417, 424, 461, 481, 520, 676, 721, 755, 801, 830-831, 871, 899
- `deps_provider.rs`: 583, 602, 618, 647, 735, 797, 922, 935, 953, 964, 996, 1021
- `workspace_root_cache.rs`: 23, 34, 50, 54-55, 87, 185, 220

Not every hit is a finding: some name an enduring contract by its rule ID (e.g. "the CONC-7 contract spelled out in manifest_cache") and may survive as cross-references; the tag-as-prefix sites ("TEST-5 / TASK-2154 AC #1: ...") are the clear READ-13 shape, matching the style.rs/sgr.rs test-doc tags already filed.

**Why it matters**: READ-13 — docs must describe the end state; RULE/TAG prefixes are process artifacts that rot as the code moves on while looking authoritative.

**Origin**: discovered during TASK-2614 (code-review-plan-wave60) while fixing TASK-2459/2460/2461/2464/2465/2466/2467; the member ACs scoped each fix to its narrating region, so these out-of-scope tags were left rather than swept up.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every in-scope site either states the end state in present-tense prose with no RULE/TASK prefix, or is a deliberate cross-reference to a named contract
<!-- AC:END -->
