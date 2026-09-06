---
id: TASK-1749
title: >-
  ARCH-11: ops-about-java declares ops-git and anyhow but neither is referenced
  anywhere in the crate
status: Done
assignee:
  - TASK-1990
created_date: '2026-08-27 11:14'
updated_date: '2026-08-28 15:48'
labels:
  - code-review-rust
  - structure-readability
dependencies: []
modified_files:
  - extensions-java/about/Cargo.toml
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/Cargo.toml:9-16`

**What**: The `[dependencies]` table lists eight crates; two of them have zero references in the crate's source:

```
ops-git = { workspace = true }   # no `ops_git` / `ops-git` token anywhere under src/
anyhow  = { workspace = true }   # no `anyhow` token anywhere under src/
```

Verified with `grep -rn "ops_git\|ops-git\|anyhow" extensions-java/about/src/` — no hits. The other six are all live:

- `ops-core` — `project_identity`, `text::for_each_trimmed_line`
- `ops-extension` — `Context`, `DataProvider`, `impl_extension!`
- `ops-about` — `identity::provide_identity_from_manifest`, `manifest_io::read_optional_text`
- `serde_json` — provider return type
- `tracing` — `tracing::debug!` in `gradle/lexer.rs`
- `linkme` — required by the `impl_extension!` expansion, which emits `#[linkme::distributed_slice(…)]` (`crates/extension/src/macros.rs:86`), so it must stay a direct dependency even though the crate source never names it

**Why it matters**: ARCH-11 — unused members inflate the build graph and, more importantly, misrepresent the crate's coupling: `ops-git` in the manifest implies this extension touches the repository, which it does not. It also means a future CVE or version bump on those crates lands on this crate's rebuild path for nothing.

**Fix**: remove the two entries and confirm the workspace still builds. Worth pairing with a `cargo machete` / `cargo +nightly udeps` pass over the workspace, since `linkme` shows why a plain grep alone is not a safe rule — macro-expanded paths count as real uses.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ops-git and anyhow are removed from extensions-java/about/Cargo.toml [dependencies]
- [x] #2 cargo build --all-targets and cargo clippy --all-targets --workspace -- -D warnings both pass afterwards
- [x] #3 linkme is retained (required by the impl_extension! expansion) and the reason is not re-litigated by the same grep
<!-- AC:END -->
