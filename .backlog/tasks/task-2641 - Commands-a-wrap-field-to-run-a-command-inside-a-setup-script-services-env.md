---
id: TASK-2641
title: 'Commands: a wrap field to run a command inside a setup script (services, env)'
status: To Do
assignee: []
created_date: '2026-10-26 22:12'
updated_date: '2026-10-31 23:55'
labels:
  - feature
  - config
dependencies: []
parent_task_id: 'TASK-2644'
modified_files: []
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Commands that need services are written as a wrapper script with the real command after --, which hides the real command and blocks clone.

dbsec: next-all is ./scripts/e2e-services.sh --pg --bao -- cargo nextest run --workspace --all-features --run-ignored all, i.e. the stack's next plus --run-ignored all inside a wrapper. release-tests is the same wrapper around bash -c "cargo nextest ... --run-ignored ignored-only && cargo run ... --example embedded", a composite flattened into a shell string because the services must outlive both steps.

Proposal: wrap = ["./scripts/e2e-services.sh", "--pg", "--bao", "--"] on an exec command, prepended at run time, so next-all becomes clone = "next", args = ["--run-ignored", "all"], wrap = [...]. Open question: wrapping a sequential composite, so one service lifetime spans several steps (release-tests), e.g. setup/teardown commands on a composite instead of a wrapper.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 wrap prepends to the resolved command line, after clone/extend args
- [ ] #2 ops --dry-run and ops explain show the wrapper and the wrapped command separately
- [ ] #3 A decision recorded on composite-level service lifetime (wrap or setup/teardown)
<!-- AC:END -->
