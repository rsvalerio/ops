---
id: TASK-0932
title: >-
  SEC-33: text::for_each_trimmed_line reads manifests with no byte cap (DoS via
  oversized go.mod/gradle/requirements)
status: Done
assignee: []
created_date: '2026-05-02 15:50'
updated_date: '2026-05-02 16:53'
labels:
  - code-review-rust
  - security
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/text.rs:55-72`

**What**: `for_each_trimmed_line` calls `std::fs::read_to_string(path)` with no upper bound on the file size before iterating its lines. This helper is the shared skeleton for line-based manifest parsers across stacks (go.mod, go.work, gradle.properties, requirements.txt — see TASK-0321 / TASK-0442). A hostile or accidentally-corrupt manifest of arbitrary size is slurped wholly into memory before any line iteration, with the entire content held in `String` for the duration of the callback.

**Why it matters**: Sibling gap to the TASK-0831 (extension manifest readers), TASK-0926 (Cargo.toml in extensions-rust), TASK-0910 (.git/config), TASK-0915/0924 (terraform plan), TASK-0927 (.git/HEAD) cap series. Every line-based manifest reader that funnels through this helper inherits the same DoS shape: a 10 GB requirements.txt blows the process resident set before the first `f(line.trim())` callback runs. The cap series treats this as a high-priority pattern; this central helper is the highest-leverage fix point because every Go/Python/Gradle/Ansible parser in `extensions-*` calls into it.

**Acceptance criteria**: introduce a documented byte cap (e.g. `OPS_MANIFEST_MAX_BYTES` mirroring `OPS_PLAN_JSON_MAX_BYTES`) with a sensible default (4 MiB feels generous for a manifest), enforced via `Read::take(cap)` or equivalent before string allocation. On overflow, log at warn and return `None` (matching the existing IO-error pass-through) plus surface a clear diagnostic message naming the path and effective cap. Add a regression test that writes a `cap + 1` byte file and asserts the helper does not allocate the full content.

<!-- scan confidence: high -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Introduce byte cap (default ~4 MiB, env-overridable) on read path before full content allocation
- [ ] #2 Cap overflow logged at warn with path + effective cap; helper returns None
- [ ] #3 Regression test asserts cap+1 byte file does not OOM the helper and surfaces the diagnostic
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added crates/core/src/text.rs::read_capped_to_string + manifest_max_bytes (default 4 MiB, OPS_MANIFEST_MAX_BYTES env override). for_each_trimmed_line now routes through it; oversize logged at warn with cap field, returns None without invoking callback. Three regression tests added (oversize None, capped error, at-cap success). ops verify + full test suite pass.
<!-- SECTION:NOTES:END -->
