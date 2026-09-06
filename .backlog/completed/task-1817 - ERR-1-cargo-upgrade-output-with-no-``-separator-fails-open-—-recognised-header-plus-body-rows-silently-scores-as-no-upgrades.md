---
id: TASK-1817
title: >-
  ERR-1: cargo-upgrade output with no `====` separator fails open — recognised
  header plus body rows silently scores as "no upgrades"
status: Done
assignee:
  - TASK-1997
created_date: '2026-08-27 11:33'
updated_date: '2026-08-28 20:24'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions-rust/deps/src/parse/upgrade.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/upgrade.rs:159-208` (`parse_upgrade_table_inner`), `:44-71` (`interpret_upgrade_output`), `:73-108` (`check_header_drift` / `check_row_shape_drift`)

**What**: `parse_upgrade_table_inner` only aligns columns from a `====` separator row. When cargo-edit emits body rows but no separator, `columns` stays `None`, no row is ever handed to `parse_upgrade_row`, and the function returns an empty `Vec` plus `diag.saw_separator == false`. Both drift guards are gated on `saw_separator`:

- `check_header_drift` requires `diag.saw_separator && !diag.saw_recognised_header`
- `check_row_shape_drift` requires `diag.saw_recognised_header && diag.saw_separator && …`

so neither fires. The only reaction is the `tracing::warn!("TASK-1026: …")` at `upgrade.rs:198-200`, and `interpret_upgrade_output` returns `Ok(vec![])`. `run_deps` then renders "Compatible Upgrades: None / Breaking Upgrades: None" and `has_issues` passes the gate.

This is the same fail-open class the crate has already hardened twice — TASK-1074 (header renamed, separator present) and TASK-1202 (separator present, every row fails the 5-column shape) both `anyhow::bail!` rather than score green. The third permutation (separator dropped entirely, e.g. cargo-edit switching to a box-drawing or ANSI-styled table, or piping through a wrapper that strips it) is the only one still silent, and it is also the *most* likely rendering change since the separator is pure decoration upstream.

Note the current warn-only behaviour is pinned by `parse/upgrade/table_tests.rs::parse_upgrade_table_warns_on_missing_separator`, which asserts `entries.is_empty()` and only checks for the warn — that test needs to move to the `interpret_upgrade_output` level (asserting the bail) when this is fixed, mirroring how `interpret_upgrade_output_bails_on_unrecognised_header_with_separator` is written.

**Why it matters**: the deps gate is meant to be authoritative about dependency freshness. A cargo-edit table-rendering change turns `ops deps` into a permanently green "no upgrades available" report with no error and no non-zero exit — the warn goes to `tracing`, which `ops deps` does not surface to the operator. The failure is silent and indefinite: nobody notices that upgrades stopped being reported.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 interpret_upgrade_output returns Err when stdout contained body/content lines but no `====` separator row was seen, instead of Ok(vec![])
- [x] #2 The bail message names the missing-separator drift case and is distinguishable from the header-drift (TASK-1074) and row-shape-drift (TASK-1202) messages
- [x] #3 The existing TASK-1026 tracing::warn! is kept or moved so the drift stays observable in logs
- [x] #4 A test drives interpret_upgrade_output(Some(0), <recognised header + body rows, no separator>, b"") and asserts it errs
- [x] #5 parse_upgrade_table_warns_on_missing_separator is updated (or superseded) so it no longer pins the warn-only fail-open behaviour
<!-- AC:END -->
