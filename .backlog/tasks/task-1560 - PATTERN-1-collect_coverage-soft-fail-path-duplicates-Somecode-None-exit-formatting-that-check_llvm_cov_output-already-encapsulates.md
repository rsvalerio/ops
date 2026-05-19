---
id: TASK-1560
title: >-
  PATTERN-1: collect_coverage soft-fail path duplicates Some(code)/None
  exit-formatting that check_llvm_cov_output already encapsulates
status: To Do
assignee:
  - TASK-1577
created_date: '2026-05-19 15:43'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - idioms
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs:330-336` vs `132-136`

**What**: `collect_coverage` (lines 330-336) formats the cargo exit code with a custom `Some(c).to_string() / None → "signal"` pattern duplicating the logic in `check_llvm_cov_output` (lines 132-136 — `Some(code) → status {code}`, `None → terminated by signal (exit_code = None)`). The duplication risks drift: TASK-1099 was filed for the helper to include `exit_code = None` for signal kills, but the parallel soft-fail formatter writes `signal` without the `exit_code = None` marker.

```rust
// collect_coverage soft-fail branch
let code_str = output.status.code().map(|c| c.to_string()).unwrap_or_else(|| \"signal\".to_string());
tracing::warn!(exit_code = %code_str, stderr_tail = %tail, ...);

// check_llvm_cov_output hard-fail branch
match output.status.code() {
    Some(code) => anyhow::bail!(\"... status {code}: {tail}\"),
    None => anyhow::bail!(\"... terminated by signal (exit_code = None): {tail}\"),
}
```

**Why it matters**: PATTERN-1 — two places formatting the same `Output -> human exit string` will drift; SIGKILL/OOM operators see different markers depending on whether the soft-fail or hard-fail path fires, breaking grep-on-logs (`exit_code = None`). Extract a single helper (`format_cargo_exit(status: &ExitStatus) -> Cow<str>`) used by both sites.

<!-- scan confidence: confirmed -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Single helper formats cargo exit status; both collect_coverage soft-fail and check_llvm_cov_output use it
- [ ] #2 SIGKILL marker (exit_code = None or signal) is identical across both code paths
<!-- AC:END -->
