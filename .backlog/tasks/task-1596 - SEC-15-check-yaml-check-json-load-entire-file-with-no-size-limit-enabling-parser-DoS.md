---
id: TASK-1596
title: >-
  SEC-15: check-yaml/check-json load entire file with no size limit, enabling
  parser DoS
status: Done
assignee:
  - TASK-1636
created_date: '2026-05-21 22:52'
updated_date: '2026-05-22 12:17'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/lib.rs:147`, `src/yaml.rs:9`, `src/json.rs:7-14`

**What**: `run_checker` reads each candidate file fully into memory via `std::fs::read(&path)` with no upper bound on file size, then hands the bytes to `serde_json` / `json5` / `saphyr::Yaml::load_from_str` to be parsed in full. No size cap, no streaming, no per-file timeout. A multi-GB `*.yaml` or `*.json` in the discovery tree — accidental (a vendored fixture, a checked-in lockfile, a generated artifact) or adversarial (a malicious PR adding a crafted YAML file with deep nesting / repeated anchors) — will be slurped into RAM and parsed.

For YAML specifically, `saphyr` does not, to my knowledge, ship anchor-expansion limits comparable to PyYAML's safe loader, so a "billion laughs"-style file (`&a [1,1,...]` repeatedly aliased) can blow up memory even at small file sizes.

**Why it matters**: The checker is invoked from pre-commit and from CI runners, both of which are resource-constrained. A single 2 GB `.yaml` left in `node_modules/` or a tarball-extracted fixture will OOM the runner. In hostile-PR contexts (open-source repos, untrusted contributors), this is also a cheap DoS: attacker commits a crafted YAML/JSON, and every CI run for that PR thereafter falls over inside check-yaml. Standard hardening for "validate-untrusted-files" tooling is to apply a max-bytes cap (e.g., 16 MiB) plus, for YAML, a depth/alias-expansion guard, and to skip-with-warning rather than parse files that exceed the cap.

OWASP: A05:2021 (Security Misconfiguration — resource limits) / A04:2021 (Insecure Design — missing rate/size limits).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CheckerOptions gains a configurable max-bytes cap (with a sensible default such as 16 MiB) applied before fs::read or via a length-limited reader
- [x] #2 Files exceeding the cap are skipped and recorded on CheckerReport (e.g., a files_skipped counter or a dedicated 'too large' failure entry) rather than parsed
- [x] #3 YAML anchor-expansion / alias-recursion behavior of saphyr is verified; if it lacks a guard, either switch to a limiter or wrap saphyr with an alias-count cap before parsing untrusted input
- [x] #4 Unit test exercises a file exceeding the cap and verifies the file is skipped without OOM and without parsing
<!-- AC:END -->
