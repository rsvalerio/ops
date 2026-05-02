---
id: TASK-0943
title: >-
  SEC-33: .ops.toml config readers (loader.rs::read_config_file,
  edit.rs::read_ops_toml) have no byte cap
status: Done
assignee: []
created_date: '2026-05-02 16:02'
updated_date: '2026-05-02 16:17'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:124` (`read_config_file`); `crates/core/src/config/edit.rs` (`read_ops_toml`)

**What**: Both `.ops.toml` readers use `std::fs::read_to_string(path)` with no size limit. Project-local, ~/.config/ops global, and conf.d snippet readers all flow through these paths.

**Why it matters**: A multi-GiB or `/dev/zero`-symlinked `.ops.toml` (or a maliciously-crafted dotfile in a cloned repo) OOMs the CLI before TOML parsing can reject it. Sibling sweep gap relative to TASK-0910 (.git/config), TASK-0926 (Cargo.toml), TASK-0932 (manifests via for_each_trimmed_line). The central config readers were missed.

**Scanning guidance**: confirmed via grep `read_to_string` against config/*.rs; both call sites are non-test.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Both readers route through File::open + take(cap).read_to_string with a documented byte cap (≥256 KiB), surfacing a typed error rather than OOM on oversize
- [x] #2 Cap is overridable via an env var following the OPS_PLAN_JSON_MAX_BYTES pattern from TASK-0915
- [x] #3 Regression tests create a >cap .ops.toml and assert read fails with a bounded-read error rather than a successful unbounded read
<!-- AC:END -->
