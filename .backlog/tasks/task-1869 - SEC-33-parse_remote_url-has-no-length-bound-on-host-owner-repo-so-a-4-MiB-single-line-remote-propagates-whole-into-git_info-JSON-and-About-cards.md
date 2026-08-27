---
id: TASK-1869
title: >-
  SEC-33: parse_remote_url has no length bound on host / owner / repo, so a 4
  MiB single-line remote propagates whole into git_info JSON and About cards
status: Triage
assignee: []
created_date: '2026-08-27 15:30'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/git/src/remote.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/remote.rs:42-65` (`parse_remote_url`), `remote.rs:116-130` (`is_valid_host`), `remote.rs:132-180` (`split_owner_repo` / `is_valid_path_segment`)

**What**: The `.git/config` read is capped at `MAX_GIT_CONFIG_BYTES` = 4 MiB (SEC-33 / TASK-0910), but that cap bounds the *file*, not the *value*. A single `url = https://<4 MiB of 'a'>/owner/repo` line passes every downstream gate:

- `is_valid_host` checks emptiness, leading/trailing `-`/`.`, empty labels, and a byte allowlist — no total length, no per-label length. DNS caps a name at 253 bytes and a label at 63; neither is enforced.
- `is_valid_path_segment` checks emptiness, all-dot, and a byte allowlist — no length.
- `split_owner_repo` deliberately preserves an arbitrarily deep owner path (TASK-0724), so `owner` can be megabytes of nested segments.
- `parse_remote_url` then builds `format!("{scheme}://{host}/{owner}/{repo}")`, allocating a second copy, and every field is cloned into `RemoteInfo` and again into `GitInfo`.

Result: ~4 MiB of attacker-chosen text is emitted as `git_info.host` / `owner` / `repo` / `remote_url` in the provider JSON and rendered by every consumer that displays repository identity.

**Why it matters**: SEC-33 asks for bounds on *values* derived from untrusted input, not only on the enclosing buffer. The realistic impact is not OOM (the file cap keeps it to a few MiB) but output flooding and rendering blow-up: an About card or a JSON consumer receives a multi-megabyte single-line "host", and any downstream layout code that pads, wraps, or column-aligns that string does so on a value 5 orders of magnitude larger than the ~40 bytes it was designed for. It is also the missing "range/size" layer of the layered validation the neighbouring host/segment validators otherwise implement carefully.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 is_valid_host rejects hosts longer than 253 bytes and labels longer than 63 bytes
- [ ] #2 is_valid_path_segment rejects segments beyond a documented maximum, and split_owner_repo rejects an owner path beyond a documented total length / segment count
- [ ] #3 the limits are named consts with a comment stating the rationale, in the style of MAX_GIT_CONFIG_BYTES
- [ ] #4 unit tests pin rejection just over each limit and acceptance just under it, and confirm a realistic nested GitLab subgroup ('a/b/c/d/repo') still parses
<!-- AC:END -->
