---
id: TASK-2248
title: 'code-review-plan-wave14'
status: Done
assignee: []
created_date: '2026-09-08 10:52'
updated_date: '2026-09-10 19:32'
labels:
  - code-review-wave
dependencies:
  - TASK-2070
  - TASK-2075
  - TASK-2088
  - TASK-2099
  - TASK-2101
  - TASK-2125
  - TASK-2136
  - TASK-2146
  - TASK-2147
  - TASK-2155
  - TASK-2169
  - TASK-2173
  - TASK-2182
  - TASK-2189
  - TASK-2191
  - TASK-2192
  - TASK-2197
  - TASK-2208
  - TASK-2216
  - TASK-2223
  - TASK-2228
  - TASK-2230
  - TASK-2233
modified_files:
  - crates/cli/src/args.rs
  - crates/extension/src/data.rs
  - crates/extension/src/error.rs
  - crates/extension/src/lib.rs
  - crates/runner/src/command/build.rs
  - crates/runner/src/command/exec.rs
  - crates/runner/src/command/mod.rs
  - crates/runner/src/command/results.rs
  - crates/runner/src/display.rs
  - crates/runner/src/display/finalize.rs
  - crates/runner/src/display/progress_state.rs
  - crates/runner/src/display/render_config.rs
  - extensions-go/about/src/go_mod.rs
  - extensions-go/about/src/go_syntax.rs
  - extensions-go/about/src/go_work.rs
  - extensions-go/about/src/lib.rs
  - extensions-go/about/src/modules.rs
  - extensions-java/about/src/gradle/lexer.rs
  - extensions-java/about/src/gradle/mod.rs
  - extensions-java/about/src/lib.rs
  - extensions-java/about/src/maven/mod.rs
  - extensions-java/about/src/maven/pom.rs
  - extensions-node/about/src/package_json.rs
  - extensions-node/about/src/package_manager.rs
  - extensions-node/about/src/repo_url.rs
  - extensions-node/about/src/units.rs
  - extensions-python/about/src/lib.rs
  - extensions-python/about/src/units.rs
  - extensions-rust/cargo-toml/src/lib.rs
  - extensions-rust/cargo-toml/src/tests/find_root.rs
  - extensions-rust/cargo-toml/src/workspace_root.rs
  - extensions-rust/cargo-update/src/lib.rs
  - extensions-rust/create-review-tasks/src/lib.rs
  - extensions-rust/create-review-tasks/src/provider.rs
  - extensions-rust/deps/src/format.rs
  - extensions-rust/deps/src/lib.rs
  - extensions-rust/deps/src/parse/deny.rs
  - extensions-rust/deps/src/parse/mod.rs
  - extensions-rust/deps/src/parse/upgrade.rs
  - extensions-rust/deps/src/test_support.rs
  - extensions-rust/loc/src/lib.rs
  - extensions-rust/loc/src/tests.rs
  - extensions-rust/loc/src/views.rs
  - extensions-rust/metadata/src/ingestor.rs
  - extensions-rust/metadata/src/lib.rs
  - extensions-terraform/about/src/lib.rs
  - extensions-terraform/plan/src/lib.rs
  - extensions-terraform/plan/src/model.rs
  - extensions-terraform/plan/src/render.rs
  - extensions/about/src/code.rs
  - extensions/about/src/lib.rs
  - extensions/about/src/loc.rs
  - extensions/about/src/providers.rs
  - extensions/duckdb/src/sql/ingest/sidecar.rs
  - extensions/duckdb/src/sql/mod.rs
  - extensions/run-before-commit/src/lib.rs
  - extensions/run-before-push/src/lib.rs
  - extensions/text-fixers/src/binary.rs
  - extensions/text-fixers/src/discovery.rs
  - extensions/text-fixers/src/lib.rs
  - extensions/text-fixers/src/report.rs
  - extensions/text-fixers/src/runner.rs
  - extensions/text-fixers/src/trailing.rs
ordinal: 154000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave14: Docs that narrate history instead of behaviour, and log-site field drift
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2247 wave13 (29 files: crates/extension/src/data.rs ...); TASK-2239 wave5 (8 files: extensions-go/about/src/lib.rs ...); TASK-2238 wave4 (7 files: extensions-go/about/src/go_mod.rs ...); TASK-2240 wave6 (7 files: extensions-go/about/src/lib.rs ...); TASK-2234 wave0 (6 files: extensions-rust/deps/src/parse/deny.rs ...); TASK-2241 wave7 (6 files: crates/extension/src/lib.rs ...); TASK-2242 wave8 (6 files: extensions-go/about/src/go_mod.rs ...); TASK-2235 wave1 (5 files: extensions-rust/cargo-toml/src/lib.rs ...); TASK-2246 wave12 (5 files: crates/extension/src/data.rs ...); TASK-2236 wave2 (3 files: extensions-node/about/src/package_json.rs ...); TASK-2237 wave3 (3 files: extensions/text-fixers/src/discovery.rs ...); TASK-2243 wave9 (3 files: crates/extension/src/data.rs ...); TASK-2244 wave10 (2 files: crates/runner/src/command/mod.rs ...); TASK-2245 wave11 (1 file: crates/extension/src/lib.rs)

Branch: code-review/TASK-2248

<!-- SECTION:NOTES:END -->
