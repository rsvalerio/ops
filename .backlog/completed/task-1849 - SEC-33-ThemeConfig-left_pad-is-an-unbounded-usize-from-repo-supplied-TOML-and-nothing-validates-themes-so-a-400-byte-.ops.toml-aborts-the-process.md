---
id: TASK-1849
title: >-
  SEC-33: ThemeConfig::left_pad is an unbounded usize from repo-supplied TOML
  and nothing validates [themes], so a 400-byte .ops.toml aborts the process
status: Done
assignee:
  - TASK-1983
created_date: '2026-08-27 15:25'
updated_date: '2026-08-28 23:53'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - crates/core/src/config/theme_types.rs
  - crates/core/src/config/root.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/theme_types.rs:174-175` (`left_pad`), `crates/core/src/config/root.rs:75-82` (`Config::validate`)

**What**: `left_pad` is deserialized straight from `.ops.toml` with no bound:

```rust
    /// Number of spaces to prepend to all rendered output lines (left margin).
    #[serde(default = "default_left_pad")]
    pub left_pad: usize,
```

`Config::validate` — the only validation `load_config_at` runs (loader/mod.rs:227) — never touches `self.themes`:

```rust
    pub fn validate(&self) -> anyhow::Result<()> {
        for (name, spec) in &self.commands {
            if let CommandSpec::Exec(exec) = spec {
                exec.validate(name)?;
            }
        }
        Ok(())
    }
```

The value then sizes an allocation directly, in three consumers:

```rust
crates/theme/src/render.rs:19          let pad = " ".repeat(left_pad);
crates/theme/src/configurable.rs:69   // left_pad_str()
crates/runner/src/display/style.rs:28 let left_pad_str = " ".repeat(resolved_theme.left_pad());
```

Both failure modes reproduce on the shipped binary from a `.ops.toml` of roughly 400 bytes that defines a theme and selects it:

- `left_pad = 18446744073709551615` → `thread 'main' panicked at raw_vec/mod.rs: capacity overflow`
- `left_pad = 50000000000` → `memory allocation of 50000000000 bytes failed` (abort; with no ulimit this is an OOM or a swap-thrash of the host)

**Why it matters**: SEC-33. `.ops.toml` and `.ops.d/*.toml` are repo-supplied content in a tool explicitly designed to run inside third-party repositories, so this is a remote-ish crash and memory-exhaustion primitive triggered by `ops <anything>` in a hostile checkout. The existing SEC-33 defence for this input — the `ops_toml_max_bytes` read cap (TASK-0943) — does not help at all, because the payload is tiny; the amplification is entirely in the integer. Among the numeric knobs in `ThemeConfig` this one is unique: `columns` is bounded by `u16` and `stderr_tail_lines` only caps a ring buffer, so `left_pad` is the single unbounded allocation lever.

Fix direction: bound it at the type/serde level (a `u8`/`u16` field, or a `deserialize_with` that rejects anything past a sane maximum such as the terminal width), and/or add a `themes` pass to `Config::validate` so the `[themes]` table gets the same load-time screening `[commands]` already gets. A serde-level bound is preferable because it catches the value before it is ever stored.

<!-- scan confidence: verified by reading theme_types.rs:174-175 and root.rs:75-82, and reproduced against the built release binary for both the capacity-overflow and the allocation-failure shapes -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 left_pad from a .ops.toml cannot produce an allocation larger than a documented maximum; an out-of-range value is rejected at load with an error naming the theme and the field
- [x] #2 Config::validate (or the serde layer it delegates to) screens the [themes] table, so the only validated section is no longer [commands] alone
- [x] #3 Tests cover left_pad = usize::MAX and left_pad = 50_000_000_000 loading through the real load path and asserting a clean Err rather than a panic or an allocation failure
- [x] #4 The remaining numeric fields of ThemeConfig are audited in the same change and any other unbounded allocation lever is bounded or documented as bounded by its type
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Bounded at both layers, as the task's fix direction preferred.

- `MAX_LEFT_PAD = 1024` plus a `deserialize_with = "deserialize_left_pad"` on
  the field, so an out-of-range value is rejected during deserialization and can
  never be stored (AC #1). serde reports the `themes.<name>.left_pad` key path
  around the error.
- New `ThemeConfig::validate(&self, name)`, and `Config::validate` now iterates
  `self.themes` calling it — `[themes]` is no longer the one section nothing
  screens (AC #2). This is defence in depth for TOML-sourced themes and the only
  screen for programmatically-built ones; it is also what puts the theme name in
  the message.
- AC #3: `load_config_rejects_usize_max_left_pad`,
  `load_config_rejects_huge_left_pad` (50_000_000_000) both go through
  `load_config_at` and assert a clean `Err`;
  `load_config_accepts_ordinary_left_pad` guards against over-rejection, and
  `theme_validate_rejects_out_of_range_left_pad_and_names_the_theme` covers the
  non-serde path.
- AC #4 (audit, recorded in the `ThemeConfig::validate` rustdoc):
  `running_template_overhead: usize` is only ever `saturating_sub`ed from a
  `u16` column count — it shrinks a width and never sizes an allocation, so any
  value is inert. `OutputConfig::columns` is a `u16` and `stderr_tail_lines`
  caps a ring buffer; both are bounded by their own types and live outside
  `ThemeConfig`. `left_pad` was and remains the only allocation lever in the
  type.
<!-- SECTION:NOTES:END -->
