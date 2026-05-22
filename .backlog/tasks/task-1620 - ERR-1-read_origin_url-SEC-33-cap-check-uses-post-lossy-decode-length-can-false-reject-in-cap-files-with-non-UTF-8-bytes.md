---
id: TASK-1620
title: >-
  ERR-1: read_origin_url SEC-33 cap check uses post-lossy-decode length, can
  false-reject in-cap files with non-UTF-8 bytes
status: Done
assignee:
  - TASK-1639
created_date: '2026-05-22 07:00'
updated_date: '2026-05-22 13:27'
labels:
  - code-review-rust
  - error-handling
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:166-198`

**What**: `read_origin_url` reads at most `MAX_GIT_CONFIG_BYTES + 1` raw bytes via `File::take(limit).read_to_end(&mut bytes)`, then lossy-decodes the bytes into `content`, and finally checks `content.len() as u64 > MAX_GIT_CONFIG_BYTES` to enforce the SEC-33 size cap. Lossy UTF-8 decoding replaces each invalid byte with U+FFFD, which is 3 bytes when re-encoded as UTF-8 inside `String`. Therefore `content.len()` can exceed `bytes.len()` whenever the file contains invalid UTF-8. A `.git/config` whose raw size is at or just under `MAX_GIT_CONFIG_BYTES` but contains many non-UTF-8 bytes (legitimately: latin-1 commit-template, hostile injection in an unrelated `[user]` section — exactly the scenario `read_origin_url_survives_non_utf8_byte_in_unrelated_section` was added to support) will inflate above the cap on the lossy path and be rejected with the SEC-33 warn, surfacing as `remote_url = None`.

The intent of the cap is "do not slurp an unbounded-size config"; the `take(limit)` already enforces that at the IO boundary. The post-decode length check is the wrong tape measure — once `take(MAX+1)` has cleanly returned ≤ MAX bytes, the file is by definition within cap.

**Why it matters**: The pinned-behaviour test `read_origin_url_survives_non_utf8_byte_in_unrelated_section` documents that a single stray non-UTF-8 byte must NOT zero out remote detection. The current cap check re-introduces exactly that fail-open-then-fail-closed surface at the size boundary: an operator with a near-cap config plus any non-UTF-8 byte sees `remote_url = None` with a misleading SEC-33 warn ("config exceeds byte cap") even though the file fits. The boundary is unlikely in practice (configs are tiny) but the warn message would misdirect a debugging operator.

**Suggested fix**: Replace `content.len() as u64 > MAX_GIT_CONFIG_BYTES` with `bytes.len() as u64 > MAX_GIT_CONFIG_BYTES`, moved to *before* the UTF-8 conversion. Same warn semantics, no false positives.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cap check operates on raw bytes.len() rather than post-decode content.len()
- [ ] #2 Move the cap check above the lossy-UTF-8 conversion so the SEC-33 warn never fires on in-cap files containing non-UTF-8 bytes
- [ ] #3 Add a regression test: a config whose raw size is ≤ MAX_GIT_CONFIG_BYTES but contains an invalid UTF-8 byte must still surface the [remote "origin"] url
<!-- AC:END -->
