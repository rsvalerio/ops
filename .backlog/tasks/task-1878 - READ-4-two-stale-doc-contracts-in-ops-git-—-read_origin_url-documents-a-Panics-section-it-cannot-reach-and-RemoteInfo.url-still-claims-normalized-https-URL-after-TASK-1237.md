---
id: TASK-1878
title: >-
  READ-4: two stale doc contracts in ops-git — read_origin_url documents a #
  Panics section it cannot reach, and RemoteInfo.url still claims 'normalized
  https URL' after TASK-1237
status: Triage
assignee: []
created_date: '2026-08-27 15:32'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions/git/src/config.rs
  - extensions/git/src/remote.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:146-149`, `extensions/git/src/remote.rs:31`

**What**: Two rustdoc contracts no longer describe the code beneath them.

1. `read_origin_url` (`config.rs:146-149`) carries:
   ```
   /// # Panics
   ///
   /// If the URL bytes fail `String::from_utf8` after having already been
   /// validated as UTF-8 — an internal invariant violation.
   ```
   The function cannot panic. ERR-1 / TASK-1244 replaced the fallible decode with `String::from_utf8(bytes)` + a `String::from_utf8_lossy` fallback on the `Err` arm (`config.rs:206-218`), and the signature is `-> Option<RedactedUrl>` with no `unwrap`/`expect` anywhere in the body. The section is a leftover from the pre-TASK-1244 shape. It matters more than a normal stale comment because the workspace denies `clippy::expect_used` / `panic_in_result_fn`: a `# Panics` section is the documented signal callers use to decide whether a call needs isolating, and this one points at a panic that cannot happen.

2. `RemoteInfo.url` (`remote.rs:31`) says `/// Normalized https URL (no `.git` suffix, no credentials).` — but PATTERN-1 / TASK-1237 changed the field to *preserve* the input scheme (`https`/`http`/`ssh`/`git`), and the struct-level doc twelve lines above now documents exactly that. The field doc directly contradicts the type doc, and it is the one a consumer sees on hover / in the generated docs for the field they are about to read.

**Why it matters**: these are the two docs a caller actually reads before trusting `git_info` output. One tells them to guard against a panic that does not exist; the other tells them the URL is always `https://`, which is the precise misreading TASK-1237 was filed to prevent ("audit/policy code that distinguishes scheme can mistake it for TLS-fronted"). Per the skill's design philosophy, documentation that no longer matches the code is worse than no documentation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 the # Panics section is removed from read_origin_url, or rewritten to describe an actually reachable panic
- [ ] #2 RemoteInfo.url's field doc states that the input scheme is preserved (https/http/ssh/git, with scp-style normalised to ssh) and stops claiming https
- [ ] #3 a scan of the crate's remaining doc comments confirms no other post-TASK-1237 'https' or panic claim survives
<!-- AC:END -->
