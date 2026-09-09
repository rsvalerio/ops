---
id: TASK-2247
title: 'code-review-plan-wave13'
status: To Do
assignee: []
created_date: '2026-09-08 10:52'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-wave
dependencies:
  - TASK-2071
  - TASK-2072
  - TASK-2073
  - TASK-2080
  - TASK-2097
  - TASK-2098
  - TASK-2110
  - TASK-2112
  - TASK-2114
  - TASK-2124
  - TASK-2128
  - TASK-2130
  - TASK-2135
  - TASK-2138
  - TASK-2139
  - TASK-2145
  - TASK-2149
  - TASK-2152
  - TASK-2163
  - TASK-2164
  - TASK-2167
  - TASK-2174
  - TASK-2175
  - TASK-2185
  - TASK-2186
  - TASK-2187
  - TASK-2198
  - TASK-2218
  - TASK-2219
  - TASK-2221
  - TASK-2224
  - TASK-2232
modified_files:
  - crates/cli/src/run_cmd/dry_run.rs
  - crates/core/src/lib.rs
  - crates/extension/src/data.rs
  - crates/extension/src/extension.rs
  - crates/runner/src/command/mod.rs
  - extensions-go/about/src/go_mod.rs
  - extensions-go/about/src/go_syntax.rs
  - extensions-go/about/src/go_work.rs
  - extensions-go/about/src/lib.rs
  - extensions-go/about/src/modules.rs
  - extensions-java/about/src/lib.rs
  - extensions-node/about/src/lib.rs
  - extensions-node/about/src/package_json.rs
  - extensions-node/about/src/repo_url.rs
  - extensions-node/about/src/units.rs
  - extensions-rust/about/src/lib.rs
  - extensions-rust/about/src/members.rs
  - extensions-rust/about/src/units.rs
  - extensions-rust/cargo-toml/src/inheritance.rs
  - extensions-rust/cargo-toml/src/lib.rs
  - extensions-rust/cargo-toml/src/types.rs
  - extensions-rust/cargo-toml/src/workspace_root.rs
  - extensions-rust/cargo-update/src/lib.rs
  - extensions-rust/create-review-tasks/src/lib.rs
  - extensions-rust/create-review-tasks/src/provider.rs
  - extensions-rust/deps/src/lib.rs
  - extensions-rust/deps/src/parse/upgrade.rs
  - extensions-rust/deps/src/types.rs
  - extensions-rust/loc/src/counter.rs
  - extensions-rust/loc/src/ingestor.rs
  - extensions-rust/loc/src/lib.rs
  - extensions-rust/loc/src/views.rs
  - extensions-rust/test-coverage/src/ingestor.rs
  - extensions-rust/test-coverage/src/lib.rs
  - extensions-rust/test-coverage/src/parse.rs
  - extensions-rust/test-coverage/src/provider.rs
  - extensions-rust/test-coverage/src/subprocess.rs
  - extensions-rust/test-coverage/src/views.rs
  - extensions-terraform/about/src/lib.rs
  - extensions-terraform/plan/src/lib.rs
  - extensions-terraform/plan/src/model.rs
  - extensions/about/src/cards.rs
  - extensions/about/src/code.rs
  - extensions/about/src/coverage.rs
  - extensions/about/src/deps.rs
  - extensions/about/src/lib.rs
  - extensions/about/src/lru.rs
  - extensions/about/src/manifest_cache.rs
  - extensions/about/src/text_util.rs
  - extensions/about/src/units.rs
  - extensions/config-checkers/src/lib.rs
  - extensions/config-checkers/src/options.rs
  - extensions/config-checkers/src/report.rs
  - extensions/config-checkers/src/runner.rs
  - extensions/create-review-tasks/src/lib.rs
  - extensions/duckdb/src/error.rs
  - extensions/duckdb/src/ingestor.rs
  - extensions/duckdb/src/lib.rs
  - extensions/duckdb/src/sql/ingest/sql.rs
  - extensions/duckdb/src/sql/query/helpers.rs
  - extensions/duckdb/src/sql/query/loc.rs
  - extensions/duckdb/src/sql/validation.rs
  - extensions/git/src/config.rs
  - extensions/git/src/lib.rs
  - extensions/git/src/provider.rs
  - extensions/hook-common/src/lib.rs
  - extensions/run-before-commit/src/lib.rs
  - extensions/run-before-push/src/lib.rs
  - extensions/text-fixers/src/report.rs
  - extensions/text-fixers/src/runner.rs
  - extensions/tokei/src/ingestor.rs
  - extensions/tokei/src/lib.rs
  - extensions/tokei/src/views.rs
ordinal: 153000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave13: Missing doc summaries, duplicate public paths, and weak must_use
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2248 wave14 (29 files: crates/extension/src/data.rs ...); TASK-2236 wave2 (7 files: extensions-node/about/src/package_json.rs ...); TASK-2240 wave6 (7 files: extensions-go/about/src/lib.rs ...); TASK-2234 wave0 (6 files: extensions-rust/deps/src/parse/upgrade.rs ...); TASK-2237 wave3 (5 files: extensions/config-checkers/src/lib.rs ...); TASK-2242 wave8 (5 files: extensions-go/about/src/go_mod.rs ...); TASK-2246 wave12 (5 files: crates/extension/src/data.rs ...); TASK-2235 wave1 (4 files: extensions-rust/cargo-toml/src/lib.rs ...); TASK-2238 wave4 (4 files: extensions-go/about/src/go_mod.rs ...); TASK-2239 wave5 (4 files: extensions-go/about/src/lib.rs ...); TASK-2241 wave7 (4 files: extensions-terraform/plan/src/lib.rs ...); TASK-2243 wave9 (4 files: crates/extension/src/data.rs ...); TASK-2244 wave10 (4 files: crates/runner/src/command/mod.rs ...); TASK-2249 wave15 (1 file: extensions/create-review-tasks/src/lib.rs)
<!-- SECTION:NOTES:END -->
