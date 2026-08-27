---
id: TASK-1722
title: >-
  SEC-11: normalize_repo_url has no URL-scheme allowlist —
  javascript:/data:/file: reach the rendered repository field
status: Triage
assignee: []
created_date: '2026-08-27 11:11'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-node/about/src/repo_url.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/repo_url.rs:79-85`, `extensions-node/about/src/repo_url.rs:99-102`

**What**: `normalize_repo_url` recognises and rewrites a fixed set of shapes (npm shorthand, `ssh://`, `git+ssh://`, `git+<scheme>://`, `git://`, bare `owner/repo`) and then falls through, returning the input **verbatim** as the repository URL. There is no validation that the surviving value is an `http`/`https` URL at all. Three concrete escapes from an attacker-authored `package.json`:

1. Fall-through (`repo_url.rs:102`) — `"repository": "javascript:alert(document.domain)"` returns `javascript:alert(document.domain)` unchanged. Same for `data:text/html;base64,...`, `vbscript:`, `file:///etc/shadow`. `is_bare_github_shorthand` explicitly rejects anything containing `:` (`repo_url.rs:115`), so these never reach the shorthand rewrite and land on the verbatim fall-through.
2. `git+` branch (`repo_url.rs:79-84`) — `"repository": "git+javascript:alert(1)"`. `strip_prefix("git+")` leaves `javascript:alert(1)`; `scrub_full_url_path` finds no `://`, `scrub_authority_and_path` finds no `/`, and returns the body verbatim → `javascript:alert(1)`.
3. `git+file:///etc/passwd` → `scrub_full_url_path` splits on `://`, preserves the `file` scheme, and returns `file:///etc/passwd`.

The value flows into `PackageJson::repository` (`package_json.rs:130-145`), then `ParsedManifest`/`ProjectIdentity::repository`, and is rendered into the About card (`crates/core/src/project_identity/card.rs:101-103`) and emitted in `ops about --json`. The card renderer strips ANSI escapes but does not filter schemes.

**Why it matters**: This is the last unpatched member of the module's own documented threat model. Control characters (SEC-2 / TASK-1080, TASK-1165), path traversal (SEC-14 / TASK-0811, TASK-1111, TASK-1205), and hostless authorities (API / TASK-1256) were each closed by *dropping the field* — the module doc comment states the goal explicitly: the About card must surface "no link at all rather than a silently rewritten one". Scheme is the remaining hole: `package.json` is attacker-controlled data in any repository a user runs `ops about` inside, and the rendered/serialised `repository` value is operator-facing and consumed by downstream tooling reading `--json`. A `javascript:`/`data:` URI reaching any consumer that renders it as a hyperlink is a direct injection sink; a `file:` URI is a local-resource-disclosure lure.

**Fix shape**: after all rewrite branches, validate the result against an allowlist of `https://` and `http://` (the rewrite branches already only ever produce `https://`); anything else returns `""`. Callers (`package_json.rs:134-144`) already treat `""` as a missing field, so no call-site change is needed.

**Notes**: `scrub_full_url_path`'s no-`://` fallback (`repo_url.rs:228-234`) and `scrub_authority_and_path`'s no-`/` fallback (`repo_url.rs:218`) are the two lines that pass a non-URL body through untouched; neither has a test.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 normalize_repo_url returns an empty string for any input whose normalised result does not start with http:// or https://
- [ ] #2 Tests pin that javascript:alert(1), data:text/html;base64,AAA, vbscript:x, and file:///etc/passwd each normalise to the empty string
- [ ] #3 Tests pin that git+javascript:alert(1) and git+file:///etc/passwd each normalise to the empty string
- [ ] #4 Existing accepted shapes still round-trip: github:owner/repo, expressjs/express, git+ssh://git@github.com/o/r.git, git://github.com/o/r, https://github.com/o/r
- [ ] #5 parse_package_json drops the repository field (None) when the scheme check rejects the value
<!-- AC:END -->
