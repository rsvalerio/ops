---
id: TASK-1593
title: 'API-5: config-checkers public report-returning functions lack #[must_use]'
status: Done
assignee:
  - TASK-1636
created_date: '2026-05-21 22:50'
updated_date: '2026-05-22 12:17'
labels:
  - code-review-rust
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/lib.rs`

**What**: Public functions and methods that return values whose entire purpose is to be inspected by the caller are not annotated `#[must_use]`:

- `run_check_json` (`lib.rs:92`) returns `anyhow::Result<CheckerReport>`
- `run_check_yaml` (`lib.rs:107`) returns `anyhow::Result<CheckerReport>`
- `CheckerReport::failed` (`lib.rs:86`) — pure predicate over the report
- `json::check_json` (`json.rs:5`) returns `Result<(), String>`
- `yaml::check_yaml` (`yaml.rs:7`) returns `Result<(), String>`

`CheckerOptions::with_allow_jsonc` already uses `#[must_use]`, so the pattern is established.

**Why it matters**: The CheckerReport drives the CLI's exit code; silently dropping the value (e.g., `run_check_json(&opts, &mut io)?;` without using the report) is the exact bug `#[must_use]` exists to catch. For the parse-only `check_json`/`check_yaml`, ignoring the `Result` defeats the validator's purpose.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 run_check_json, run_check_yaml, CheckerReport::failed, json::check_json, yaml::check_yaml annotated with #[must_use]
- [x] #2 cargo clippy -p ops-config-checkers --all-targets -- -D warnings stays clean
<!-- AC:END -->
