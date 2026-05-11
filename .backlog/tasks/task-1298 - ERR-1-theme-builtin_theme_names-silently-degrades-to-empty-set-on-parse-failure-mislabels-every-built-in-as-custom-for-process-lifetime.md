---
id: TASK-1298
title: >-
  ERR-1: theme builtin_theme_names silently degrades to empty set on parse
  failure, mislabels every built-in as (custom) for process lifetime
status: Done
assignee:
  - TASK-1306
created_date: '2026-05-11 16:19'
updated_date: '2026-05-11 19:16'
labels:
  - code-review-rust
  - err
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: \`crates/cli/src/theme_cmd.rs:32\`

**What**: \`builtin_theme_names()\` caches the result of \`parse_default_config()\` in a \`OnceLock\`. On parse failure it logs one \`tracing::warn!\` and stores an empty \`HashSet\`, which means every subsequent call (for the entire process lifetime) returns the same empty set. Downstream, \`collect_theme_options\` then labels every built-in theme as \`(custom)\` in \`run_theme_list\` output because \`!builtins.contains(name.as_str())\` is always true.

The one-shot warn is fired before any user-visible UI runs (\`run_theme_list\` writes only the option list to stdout); operators who don't have \`RUST_LOG=ops=warn\` enabled see no indication that the (custom) marker is wrong.

**Why it matters**: The embedded \`default_ops_toml()\` is compiled into the binary so the parse-failure branch is unreachable in normal operation — but the silent-degrade pattern itself is the finding: a permanently-cached empty-set fallback hides a regression in the embedded TOML (e.g. someone adds a new field to ThemeConfig without bumping the deserializer, or adds a syntactically-invalid theme block). The user-visible symptom is wrong UI labels, with no error surface and no way to re-attempt the parse without restarting the process. Either fail loud (the embedded config is a compile-time invariant — \`expect\` it and let a broken default crash startup), or surface the parse error through the public theme APIs (\`Result<...>\` propagation) so \`run_theme_list\` can render a one-line error instead of mislabeling rows.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 On embedded-config parse failure, either: (a) builtin_theme_names panics with expect("embedded default config must parse") so a broken default crashes startup, or (b) parse_default_config errors propagate through builtin_theme_names/collect_theme_options and run_theme_list renders a user-visible diagnostic line instead of mislabeling built-ins as (custom)
- [x] #2 Unit test pins the chosen behaviour (panic with named message, or propagated Err carrying the toml::de::Error context)
<!-- AC:END -->
