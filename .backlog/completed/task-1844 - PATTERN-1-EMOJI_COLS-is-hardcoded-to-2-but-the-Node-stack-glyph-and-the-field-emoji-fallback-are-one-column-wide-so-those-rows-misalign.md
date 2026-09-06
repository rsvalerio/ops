---
id: TASK-1844
title: >-
  PATTERN-1: EMOJI_COLS is hardcoded to 2, but the Node stack glyph and the
  field-emoji fallback are one column wide, so those rows misalign
status: Done
assignee:
  - TASK-1984
created_date: '2026-08-27 15:24'
updated_date: '2026-08-29 00:36'
labels:
  - code-review-rust
  - correctness
dependencies: []
modified_files:
  - crates/core/src/project_identity/card.rs
  - crates/core/src/project_identity/format.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity/card.rs:262-284` (column constants + `continuation_indent`), `crates/core/src/project_identity/card.rs:296-320` (`render_field`), `crates/core/src/project_identity/format.rs:27-70` (`field_emoji` / `stack_emoji`)

**What**: The about-card layout is documented and computed as a fixed-width column stack:

```rust
//   `"  " (LEADING) + emoji (EMOJI_COLS) + " " (KEY_SEP) + padded_key
//      (max_key_len + KEY_PAD) + " " (VALUE_SEP) + value`
const LEADING_COLS: usize = 2;
const EMOJI_COLS: usize = 2;
```

`continuation_indent` sums those constants, and `render_field` interpolates the glyph without measuring it:

```rust
let mut out = vec![format!("  {} {} {}", emoji, padded_key, styled(&sanitised(first)))];
for cont in value_lines {
    out.push(format!("{}{}", cont_indent, styled(&sanitised(cont))));
}
```

The key is padded by *measured* display width (`pad_to_display_width`, the DUP-3 / TASK-1390 and PERF-3 / TASK-1220 work); the emoji is not. Measured against the workspace's pinned `unicode-width 0.2`, two of the glyphs `field_emoji` can return are **one** column wide, not two:

| glyph | codepoint | width |
|---|---|---|
| ⬢ (Node/JavaScript stack) | `U+2B22` | **1** |
| ▸ (`_` fallback arm) | `U+25B8` | **1** |
| 📦 📝 🏷️ 📜 🔗 👤 🧩 🧪 🌐 🦀 📚 | — | 2 |

Both are reachable in shipped code, not hypothetically:

1. **Node projects.** `stack_emoji` returns `"\u{2b22}"` for `"Node" | "JavaScript"` (format.rs:59), and `extensions-node/about/src/lib.rs:96` sets `stack_label = "Node"`. So on every `ops about` in a Node repo the `stack` row starts one column to the left of the `project` / `packages` / `codebase` / `repository` rows.
2. **The fallback arm.** `field_emoji`'s catch-all returns `▸` for any key not in its list. `extensions/about/src/identity.rs:188` sets `m.module_label = "crate"` — **singular** — which does not match the `"crates" | "packages" | "modules" | "subprojects"` arm, so that row takes the width-1 fallback too.

The misalignment compounds on multi-line values: `cont_indent` is built from the constant `2`, so a width-1 emoji row's continuation lines land one column to the *right* of that row's own first value line. The Node `stack` value is exactly this shape — `compose_stack_value` appends the stack detail (e.g. "ESM") as a second line.

Existing coverage does not catch it: `render_field_aligns_multi_byte_key_by_display_width` exercises two rows that both take the `▸` fallback, so their equal (and equally wrong) emoji widths cancel out.

**Why it matters**: The about card is the crate's primary rendered artifact, and column alignment is the one property it exists to provide. This is the same defect class the `display_width` / `pad_to_display_width` work closed for *keys* (TASK-1220, TASK-1187, TASK-1390) — solved everywhere except the one column still written as a literal. The fix is to derive the emoji column from `display_width(emoji)` (padding the glyph to a chosen width, or threading the measured width into `continuation_indent`) rather than asserting it.

<!-- scan confidence: verified — widths measured by compiling against unicode-width 0.2, the pinned workspace version; both reachable callers cited above were read -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The emoji column is sized from the glyph's measured display width (or every glyph field_emoji can return is padded to a single documented width), so every row's key column starts at the same terminal column
- [x] #2 continuation_indent for a row is derived from that row's actual emoji width, so a multi-line value's continuation lines align with its own first value line
- [x] #3 A test renders a card whose rows mix a width-2 glyph and a width-1 glyph (Node stack, or the ▸ fallback) and asserts both rows' value columns start at the same display column
- [x] #4 A test covers a multi-line value on a width-1-emoji row and asserts the continuation line aligns with the first value line
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in wave TASK-1984. `render_field` now pads the glyph with `pad_to_display_width(field_emoji(..), EMOJI_COLS)`, so every row occupies the same measured two-cell emoji slot regardless of whether the glyph is width-1 (Node ⬢ U+2B22, fallback ▸ U+25B8) or width-2. Because every row now matches EMOJI_COLS exactly, `continuation_indent` — derived from the same constant — matches each row own emoji width. The EMOJI_COLS rustdoc records that the constant describes the rendered column rather than asserting a property of the glyph set. Tests: render_field_aligns_rows_mixing_width_1_and_width_2_glyphs (asserts the width-1/width-2 premise, then equal value columns for license/stack/crate) and multi_line_value_on_width_1_emoji_row_aligns_its_continuation.
<!-- SECTION:NOTES:END -->
