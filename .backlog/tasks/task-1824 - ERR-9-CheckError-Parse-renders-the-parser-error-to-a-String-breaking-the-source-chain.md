---
id: TASK-1824
title: >-
  ERR-9: CheckError::Parse renders the parser error to a String, breaking the
  source() chain
status: To Do
assignee:
  - TASK-2004
created_date: '2026-08-27 11:33'
updated_date: '2026-08-28 14:15'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - extensions/config-checkers/src/lib.rs
  - extensions/config-checkers/src/json.rs
  - extensions/config-checkers/src/yaml.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/lib.rs:80-83` (`CheckError::Parse(String)`), `lib.rs:95-102` (`Error::source`), constructed at `json.rs:20`, `json.rs:24`, `yaml.rs:18`

**What**: the three parser errors are stringified at the construction site (`.map_err(|e| CheckError::Parse(e.to_string()))`) and the original is dropped, so `source()` returns `None` for the variant that carries the actual cause:

```rust
fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
        Self::InvalidUtf8(e) => Some(e),
        Self::Parse(_) => None,
    }
}
```

The doc comment gives the reason — "the concrete parser error types (`serde_json::Error`, `json5::Error`, `saphyr::ScanError`) diverge, so the message is stored as a rendered string" — which is a real constraint but not one that requires discarding the source. `Box<dyn std::error::Error + Send + Sync + 'static>` erases the divergence while keeping the chain, and all three types satisfy it.

What is lost: `serde_json::Error` carries `line()`, `column()` and `classify()`; `saphyr::ScanError` carries a `Marker`. Once flattened to `String`, a caller that wants to sort failures by line, group by error class, or emit a machine-readable report has to re-parse the rendered English.

**Why it matters**: ERR-9 (implement `source()` so the chain is walkable) and ERR-10 (do not represent errors as strings). Severity is Low, not higher: this is a dormant concern today because the only consumer is `run_checker`, which immediately calls `.to_string()` anyway, and the rendered messages already contain the position text a human needs. It becomes real the moment anything wants structured output — a `--format json` mode, editor integration, or per-class counting.

**Fix shape**: `Parse(Box<dyn std::error::Error + Send + Sync + 'static>)`, return `Some(&**e)` from `source()`, and keep `Display` delegating to the boxed error so the current messages are byte-identical. Note the interaction with ERR-9's other half: whatever `Display` says must not simply re-interpolate the source, or chain-walking printers repeat it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 CheckError::Parse retains the underlying parser error (boxed) instead of a rendered String
- [ ] #2 Error::source() returns Some for the Parse variant, so the chain is walkable
- [ ] #3 The rendered messages emitted by run_checker are unchanged, and Display does not duplicate the source text
<!-- AC:END -->
