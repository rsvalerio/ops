---
id: TASK-2350
title: >-
  Foundation mise.toml template pins ops at an exact older version, so a repo
  running the current ops fails its own drift check
status: Triage
assignee: []
created_date: '2026-10-03 07:51'
labels:
  - foundation
  - bug
dependencies: []
priority: medium
ordinal: 163000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/foundation/templates/mise.toml` (`ops = "0.77.0"`), `extensions-rust/foundation/src/compare.rs`

**What**: the template embedded in ops 0.79.0 pins `ops = "0.77.0"`, and `ops init --rust --check` compares tool versions exactly. forge's rust-ci installs the caller's `mise.toml`, so the ops that runs the check is the one that file pins. A repo that pins the current release therefore fails that release's own check:

```
$ ops --version
ops 0.79.0
$ ops init --rust --check
drift   mise.toml:tools.ops: expected "0.77.0", found "0.79.0"
```

Pinning 0.77.0 instead passes only because 0.77.0 does not check `mise.toml` at all. The template's own comment says "ops must stay at 0.77.0 or later", which is a floor, not the equality the check enforces.

**Why it matters**: every consumer has to choose between running an old ops and carrying a `[foundation.waivers]` entry for `mise.toml:tools.ops`, and the template can never name the release that embeds it without a bump step that rewrites it. dbsec carries that waiver from its ops 0.79.0 upgrade.

**Origin**: found upgrading dbsec to ops 0.79.0, 2026-10-03.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A mise.toml that pins the running ops version passes ops init --rust --check with no waiver
- [ ] #2 An ops pin below the floor the template states (0.77.0, the first check-only verify) is still reported as drift
- [ ] #3 A test covers both, and docs/foundation.md says how the ops pin is compared
<!-- AC:END -->
