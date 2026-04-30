# `ops plan` — Terraform plan summary subcommand

## Goal

Add a `plan` subcommand to `ops` (`github.com/rsvalerio/ops`) that either (a) runs `terraform plan` in the current directory and summarizes it, or (b) reads an existing `terraform show -json` payload from a file or stdin and prints the same summary. Scoped to **tables only** for the first iteration, inspired by [tfprettyplan](https://github.com/ao/tfprettyplan) and [tf-summarize](https://github.com/dineshba/tf-summarize), but Rust-native and aligned with this repo’s crates and table stack.

## Scope

In scope:

- **Three input modes** (first implementation):
  1. **Default** — detect `terraform` on PATH (spawn `terraform` by name; on `NotFound`, print a clear error). Run `terraform plan` → binary plan → `terraform show -json` → parse → tables.
  2. **File** — `--json-file <path>` reads UTF-8 JSON plan text from that path (no `terraform` subprocess for plan/show).
  3. **Stdin** — `--json-file -` (or equivalent documented spelling) reads the same JSON from stdin until EOF.
- Parse the JSON plan and print two tables: per-action summary, and categorized resource changes.
- Exit codes that are scriptable.

Out of scope (for this iteration):

- Per-resource attribute diff tables.
- HTML / markdown export.
- Auto-approve / apply.
- `tofu` support (note as a future `--engine` toggle only).
- Extra color crates beyond what [`comfy_table::Color`](https://docs.rs/comfy-table) already provides via existing [`OpsTable`](../../crates/core/src/table.rs).

## Project alignment (do this, not the old crate list)

- **Reuse `ops-core`**: parsing, classification, and table rendering live under `crates/core/src/` (e.g. `terraform_plan/` with `model.rs`, `render.rs`, and a small `mod.rs` exporting a single entry like `summarize_from_json_str` / `run_plan_pipeline`). This crate already has `comfy-table`, `terminal_size`, `serde`, `anyhow`, and `shellexpand`.
- **Reuse [`OpsTable`](../../crates/core/src/table.rs)** for TTY-aware cells and column width caps (same pattern as the rest of the CLI output story); use `terminal_size::terminal_size()` where you need viewport width, not a new dependency.
- **Path / `~` expansion**: use existing `shellexpand` (already in `ops-core`) for default `--out` / `--json-out` paths — do **not** add `dirs`.
- **Resolve `terraform`**: use `std::process::Command::new("terraform")` (PATH resolution is OS behavior). Do **not** add `which`.
- **Thin CLI**: add `crates/cli/src/plan_cmd.rs` and wire `CoreSubcommand::Plan { ... }` in [`args.rs`](../../crates/cli/src/args.rs) + [`main.rs`](../../crates/cli/src/main.rs) dispatch, same style as [`theme_cmd.rs`](../../crates/cli/src/theme_cmd.rs) / [`about_cmd.rs`](../../crates/cli/src/about_cmd.rs) (no nested `src/cmd/plan/` tree unless you split files for size only).
- **Dependencies**: add **`serde_json = { workspace = true }`** to [`crates/core/Cargo.toml`](../../crates/core/Cargo.toml) only. Avoid new workspace crates (`which`, `dirs`, `owo-colors`, `insta`, etc.).

## CLI shape

Invocation from a directory where Terraform should run (for the default mode); for `--json-file`, CWD only matters if you later add relative paths inside the JSON — today it does not.

Flags (minimal set):

| Flag                  | Default                         | Behavior |
| --------------------- | ------------------------------- | -------- |
| `--json-file <path>`  | unset                           | If set, skip `terraform plan` / `terraform show`; read plan JSON from path. Use `-` for stdin. Conflicts with forwarding `terraform plan` args if you pass any after `--` (document: passthrough only in default mode). |
| `--out <path>`        | e.g. `~/.terraform/tfplan.binary` | Binary plan from `terraform plan -out=…` (default mode only). Expand with `shellexpand`. |
| `--json-out <path>`   | e.g. `~/.terraform/tfplan.json` | Where JSON from `terraform show -json` is written (default mode only). |
| `--keep-plan`         | `false`                         | If false, delete temp binary/json after a successful summary (tune default to match whether a future `ops apply` will reuse artifacts). |
| `--no-color`          | `false`                         | Force non-TTY styling for tables (`OpsTable::with_tty(false)`). |
| `--detailed-exitcode` | `false`                         | Forward `-detailed-exitcode` to `terraform plan` and map exit codes (default mode only). |
| `-- <args>`           | —                               | Only in default mode: passed through to `terraform plan`. |

Examples:

```sh
ops plan
ops plan --no-color
ops plan --json-file ./plan.json
terraform show -json tfplan.binary | ops plan --json-file -
ops plan -- -var-file=prod.tfvars -target=module.api
ops plan --detailed-exitcode
```

## Behavior

### 0. Input mode

- If `--json-file` is **absent**: run sections 1–3 (terraform), then 4–7 on the captured JSON string.
- If `--json-file` is **present**:
  - `-` → read all of stdin to a `String` (or `Vec<u8>` + UTF-8 validate).
  - Otherwise → `std::fs::read_to_string` after `shellexpand` on the path parent if needed.
  - Skip sections 1–3; on empty input, fail with a clear message and exit `1`.

### 1. Pre-flight (default mode only)

1. Ensure `terraform` can be started: first failure with “not found” style → exit `2` with:
   ```
   error: `terraform` binary not found on PATH.
   install it from https://developer.hashicorp.com/terraform/install
   ```
2. Resolve `--out` / `--json-out` with `shellexpand`, `mkdir -p` parents.

### 2. Run `terraform plan` (default mode only)

```
terraform plan -out=<binary-plan-path> -input=false -no-color <passthrough-args>
```

- Always `-input=false`; always `-no-color` on the child (CLI does its own coloring).
- Stream **stderr** through to the user; suppress **stdout** by default (optional `--verbose` later to tee stdout).

Non-zero exit: unless `--detailed-exitcode` mapping applies, print a one-line error and propagate the code.

### 3. Render JSON (default mode only)

```
terraform show -json <binary-plan-path>
```

Capture stdout → write `--json-out` if keeping artifacts; always parse from memory for summary.

### 4. Parse the plan JSON

Same minimal structs as before (`Plan`, `ResourceChange`, `Change`, `OutputChange`); `serde` + `serde_json` in `ops-core`.

Reference: [Terraform JSON format — resource_changes](https://developer.hashicorp.com/terraform/internals/json-format#resource-change-representation).

### 5. Classify each change

Same `Action` enum and mapping rules as in the previous revision (`no-op`, `read`, replace detection via both `create` and `delete`, etc.).

### 6–7. Print tables

Use **`OpsTable`** + `comfy_table::Color` for action coloring; `--no-color` forces non-TTY tables.

Summary and “Resource Changes” tables: same columns and sorting rules as the earlier spec. For narrow terminals, use `terminal_size` + `OpsTable::set_max_width` on the address column.

### 8. Outputs section

Optional `--show-outputs` (default off); same as before.

### 9. Exit codes

Same table as the previous revision; for `--json-file` mode, omit terraform-specific rows (parse failure → `1`, success → `0` / `2` only if you define detailed semantics for “changes present” without terraform — optional: always `0` on successful parse in JSON-only mode unless you add `--detailed-exitcode` interpreting parsed counts).

## Testing (no `insta`)

- **`ops-core`**: unit tests on `classify(actions: &[&str]) -> Action` in `model.rs` (or next to it under `#[cfg(test)]`).
- **Render**: golden strings or substring asserts with `OpsTable::with_tty(false)` — same style as [`crates/core/src/table.rs`](../../crates/core/src/table.rs) tests, not snapshot crates.
- **Fixtures**: commit trimmed `terraform show -json` samples under `crates/core/tests/fixtures/` or `crates/core/src/terraform_plan/fixtures/` and test parse → classify → render end-to-end.
- **Runner**: optional `#[ignore]` integration test with a tiny `null_resource` stack; not required for CI if too heavy.

## Notes / open questions

- **`-out` vs JSON**: unchanged — binary from `plan`, JSON from `show -json`.
- **Concurrency**: `~/.terraform/` can stomp across stacks; prefer project-local `.ops/` or cwd-hash names later.
- **OpenTofu**: future `--engine` flag; not implemented now.
- **Stack gating**: optional follow-up — hide `plan` in help when `Stack` ≠ Terraform (mirror `deps` / `tools`), noting [`dist-workspace.toml`](../../dist-workspace.toml) uses `all-features = true` for releases so `stack-terraform` is available there.

## Definition of done

- `ops plan` default mode in a valid `.tf` tree prints both tables (or the “no changes” line).
- `ops plan --json-file <fixture>` and `… | ops plan --json-file -` produce identical summaries to parsing the same bytes in-process.
- `ops plan` with no terraform on PATH exits `2` with the install hint (default mode).
- `ops plan -- -var-file=foo.tfvars` forwards args (default mode).
- `ops verify` and `ops qa` pass after any `*.rs` changes.
