//! Shared name+description row renderer for CLI list views.
//!
//! `tools_cmd::render_tools_list` and `theme_cmd::run_theme_list_to` both
//! rendered the same shape inline — a
//! cyan name padded to a column width, then a dim description, then a dim
//! trailing marker. Centralising here keeps the two list surfaces' colour /
//! padding policy in lock-step when it tightens.
//!
//! The two callers differ only in: the optional leading prefix (status icon
//! for tools, none for themes) and the gap between name and description (two
//! vs three spaces). Both are passed through; the styling (cyan name, dim
//! description, dim suffix) and the display-width-aware padding are shared.
//!
//! `extension_cmd` is intentionally NOT a caller: its list surface renders
//! through `OpsTable`, a higher-level abstraction with its own column policy.

use std::io::Write;

use ops_core::output::pad_to_display_width;
use ops_core::style::{cyan, dim};

/// A single styled list row.
///
/// `leading` is emitted verbatim (already styled — e.g. an already-coloured
/// status glyph plus its trailing space, or just the row indent). `name` is
/// padded to `name_width` display columns and rendered cyan; `description`
/// and `suffix` are rendered dim. `gap` is the separator between the padded
/// name and the description.
pub(crate) struct ListRow<'a> {
    pub leading: &'a str,
    pub name: &'a str,
    pub name_width: usize,
    pub gap: &'a str,
    pub description: &'a str,
    pub suffix: &'a str,
}

pub(crate) fn write_list_row(w: &mut dyn Write, row: ListRow<'_>) -> std::io::Result<()> {
    let padded = pad_to_display_width(row.name, row.name_width);
    writeln!(
        w,
        "{}{}{}{}{}",
        row.leading,
        cyan(&padded),
        row.gap,
        dim(row.description),
        dim(row.suffix),
    )
}
