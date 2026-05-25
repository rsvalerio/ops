---
id: TASK-1207
title: >-
  SEC-2: pyproject.toml [project.urls] flow into About cards/logs without ASCII
  control-byte stripping
status: Done
assignee:
  - TASK-1259
created_date: '2026-05-08 08:17'
updated_date: '2026-05-08 13:34'
labels:
  - code-review-rust
  - sec
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:260-354`

**What**: extract_urls / pick_url return [project.urls] values trimmed only — no control-character scrub. An adversarial pyproject.toml with `Homepage = "https://demo.dev\\nINJECTED"` or an embedded ANSI escape flows verbatim into ProjectIdentity.homepage / repository, which is rendered into About cards (markdown + HTML), serialised into JSON output, and surfaced in operator-facing log lines.

**Why it matters**: Sister policy in extensions-node/about/src/repo_url.rs::strip_control_chars (SEC-2 / TASK-1080) explicitly stripped C0 + DEL from package.json::repository; the Python provider was never updated to match. The pyproject [project.urls] table is parsed from a file the user runs ops about against, the same threat surface.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 pick_url (or a new shared helper in ops_about) strips is_control() chars and U+007F before returning the URL, mirroring extensions-node strip_control_chars.
- [x] #2 Two new tests pin the contract: Homepage with embedded newline yields URL without '\n' or '\r'; embedded ANSI escape is stripped from Repository.
<!-- AC:END -->
