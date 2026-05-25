---
id: TASK-1036
title: >-
  PATTERN-1: cargo-toml find_workspace_root_with_depth re-uses canonical start
  for the entire walk, bypassing per-step canonicalisation across ancestors
status: Done
assignee: []
created_date: '2026-05-07 20:24'
updated_date: '2026-05-07 23:15'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:362-407`

**What**: After canonicalising `start` once (line 344), the loop walks via `current.parent()` without re-canonicalising at each ancestor. The doc-comment at lines 316-323 acknowledges this as a documented TOCTOU window: "if a parent directory is replaced with a symlink between canonicalisation and `manifest_declares_workspace`, the walk reads through the new symlink".

The deeper consequence not called out: when an intermediate ancestor IS itself a symlink to a different filesystem location at the time of the walk, `current.parent()` returns a path *as if* the symlink were a real directory, and subsequent `.parent()` calls walk the symlink-perceived path — which can resolve to a Cargo.toml that lives outside the user's intended workspace. The depth cap bounds the damage but a malicious workspace can plant a fake `Cargo.toml` 1-2 ancestors above and have it picked up as the workspace root.

**Why it matters**: Low severity given the documented mitigation (depth cap, "best-effort symlink-safe"), but the SEC-25 framing in the comment understates the case: this isn't only a TOCTOU, it's a pre-existing shape too. Filing so a future hardening pass can decide whether to re-canonicalise per ancestor (cost: N extra stat() calls per walk) or accept the documented gap explicitly.

**Suggested fix**: option A — re-canonicalize at each ancestor with the discovered Cargo.toml (`fs::canonicalize(cargo_toml).and_then(|p| p.parent().map(...))`). Option B — document the threat model in the public function-level docs (today only the inline comment captures it).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either: each ancestor's Cargo.toml is canonicalised before being trusted as the workspace root; OR the public docs of find_workspace_root explicitly state the symlink threat model so callers can opt for stronger enforcement
- [ ] #2 If keeping current behaviour, add a unit test that pins the documented behaviour for a symlink-replaced ancestor so a future change cannot silently regress it
<!-- AC:END -->
