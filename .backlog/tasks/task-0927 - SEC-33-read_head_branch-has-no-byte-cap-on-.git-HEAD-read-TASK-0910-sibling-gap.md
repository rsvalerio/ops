---
id: TASK-0927
title: >-
  SEC-33: read_head_branch has no byte cap on .git/HEAD read (TASK-0910 sibling
  gap)
status: Done
assignee: []
created_date: '2026-05-02 15:32'
updated_date: '2026-05-02 16:09'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:256`

**What**: `read_head_branch` calls `std::fs::read_to_string(&head_path)` with no `Read::take` cap, while its sibling `read_origin_url` (same file, same crate) was hardened in TASK-0910 with `MAX_GIT_CONFIG_BYTES`. An adversarial repo can drop a multi-GB or `/dev/zero`-symlinked HEAD and force unbounded allocation in any `ops about` / `git_info` invocation that walks past it. Real `.git/HEAD` is ~30 bytes, so a tight cap is trivial; the only reason this slipped is the TASK-0910 fix targeted only the config reader.

**Why it matters**: Same DoS class as TASK-0910 / TASK-0831 — an unprivileged user-supplied repo path can OOM the CLI. `ops about` is run interactively and inside CI containers with constrained memory; a hostile checkout (cloned for inspection, or mounted from a third-party pipeline) can crash the host process.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 read_head_branch reads at most MAX_HEAD_BYTES (<=4 KiB is plenty for ref: refs/heads/<longname>); oversized HEAD returns None with tracing::warn! mirroring the read_origin_url shape.
- [x] #2 New unit test writes a HEAD payload one byte over the cap and asserts read_head_branch returns None without slurping the file.
- [x] #3 Existing head_branch_* tests still pass; the cap value is a pub const so callers can introspect the policy.
<!-- AC:END -->
