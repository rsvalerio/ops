---
id: TASK-0155
title: >-
  READ-2: read_origin_url_from returns first remote url it sees regardless of
  section nesting
status: Done
assignee: []
created_date: '2026-04-22 21:22'
updated_date: '2026-04-23 07:42'
labels:
  - rust-code-review
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:26-44`

**What**: `read_origin_url_from` is a simple line scanner with a boolean `in_origin`. It picks up the first line starting with `url` inside any section whose header matches `[remote "origin"]`. It does not defend against: (1) continuation lines or escaped quotes in git-config, (2) a `url` that is *commented out* with a leading `#` after trim (the current code only strips leading whitespace on trim, not `#` lines — a line `# url = ...` will match because the substring check is `strip_prefix("url")` which accepts lines starting with `url`… actually `#` will fail strip_prefix, that is fine, but `insteadOf = https://...` inside `[url "..."]` would also not match because the inner check is on section, but adjacent sections with unusual whitespace will).

More concretely: an indented `[remote "origin"]` subkey like `pushurl = X` followed by `url = Y` works fine, but git config supports `url.<base>.insteadOf` rewrites that are **ignored** here — so a config that rewrites origin via `insteadOf` returns the wrong URL. Also: git-config keys are case-insensitive (`URL = ...` is valid); the current scanner is case-sensitive.

**Why it matters**: Parser correctness on hand-edited configs. Prefer `gix-config` or a minimal case-insensitive match on the key.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Document the parser limitations or switch to gix-config
- [x] #2 Treat url/URL key case-insensitively
<!-- AC:END -->
