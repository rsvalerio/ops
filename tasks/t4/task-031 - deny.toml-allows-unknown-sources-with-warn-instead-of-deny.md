---
id: TASK-031
title: 'deny.toml allows unknown registries and git sources with warn instead of deny'
status: To Do
assignee: []
created_date: '2026-04-08 16:00:00'
labels:
  - rust-security
  - RS
  - SEC-28
  - low
  - effort-S
dependencies: []
ordinal: 30000
---

## Description

**Location**: `deny.toml:36-37`
**Anchor**: `[sources]`
**Impact**: The `[sources]` section sets `unknown-registry = "warn"` and `unknown-git = "warn"`, which allows dependencies from unverified registries or git sources to be added without blocking the build. A compromised or malicious dependency from an unknown source would only produce a warning, not a build failure. OWASP: A06 (Vulnerable Components), A08 (Software and Data Integrity Failures).

**Notes**:
Current configuration:
```toml
[sources]
unknown-registry = "warn"
unknown-git = "warn"
```

Suggested fix — tighten to `deny` with explicit exceptions for known-good sources:
```toml
[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-git = ["https://github.com/quinn-rs/quinn"]  # RUSTSEC-2026-0037 patch
```

The `quinn-rs/quinn` allowlist entry is needed for the existing `[patch.crates-io]` entry that pins `quinn-proto` to the DoS-fix commit. Severity is Low because this is a local developer tool with a small dependency tree and active `cargo-deny` / `cargo-audit` usage, but `deny` is the safer default for supply chain hardening.
