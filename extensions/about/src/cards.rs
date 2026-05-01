//! Card rendering and grid layout for project unit display.
//!
//! Stack-agnostic. Fed by `ops_core::project_identity::ProjectUnit` values
//! returned from stack-specific `project_units` data providers.

use ops_core::output::display_width;
use ops_core::project_identity::ProjectUnit;
use ops_core::style::{cyan, dim, grey, white};

use crate::text_util::{
    format_number, get_terminal_width, pad_to_width_plain, truncate_to_width, tty_style, wrap_text,
};

/// Layout constants for about pages.
///
/// READ-4 (TASK-0527): kept crate-private. Out-of-crate consumers do not
/// need to compose card layouts; the struct exists only as a namespace for
/// the constants below.
struct CardLayoutConfig;

impl CardLayoutConfig {
    /// Width of each unit card in characters.
    ///
    /// READ-5 (TASK-0470): minimum supported value is 4 — `render_card`
    /// passes `CARD_WIDTH - 2` to `truncate_to_width`, which uses
    /// `max_width.saturating_sub(1)` and produces a single-ellipsis card
    /// when `inner_width < 2`. The compile-time assertion below pins this.
    const CARD_WIDTH: usize = 32;
    /// Maximum lines for description in a card.
    const CARD_DESC_LINES: usize = 3;
    /// Spacing between cards horizontally.
    const CARD_SPACING: usize = 2;
    /// Minimum terminal width to show 3 cards per row.
    const MIN_WIDTH_3_CARDS: usize = 105;
    /// Minimum terminal width to show 2 cards per row.
    const MIN_WIDTH_2_CARDS: usize = 70;
}

const _: () = assert!(
    CardLayoutConfig::CARD_WIDTH >= 4,
    "CARD_WIDTH must be >= 4: render_card relies on inner_width >= 2 \
     to keep truncate_to_width from emitting a pure-ellipsis card",
);

/// Capitalize the last path segment of a member-style string (e.g. "crates/foo" → "Foo").
pub fn format_unit_name(member: &str) -> String {
    let name = member
        .strip_prefix("**/")
        .unwrap_or(member)
        .split('/')
        .next_back()
        .unwrap_or(member);

    let mut chars = name.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

pub fn build_card_stats_line(unit: &ProjectUnit) -> Option<String> {
    let parts: Vec<String> = [
        unit.loc.map(|loc| format!("{} loc", format_number(loc))),
        unit.file_count
            .map(|f| format!("{} file{}", format_number(f), if f != 1 { "s" } else { "" })),
        unit.dep_count
            .map(|deps| format!("{} deps", format_number(deps))),
    ]
    .into_iter()
    .flatten()
    .collect();

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" \u{00b7} "))
    }
}

pub fn render_card(unit: &ProjectUnit, is_tty: bool) -> Vec<String> {
    use std::borrow::Cow;

    let inner_width = CardLayoutConfig::CARD_WIDTH - 2;

    // Borrow `unit.name` directly when no version suffix is needed (PERF-3 /
    // OWN-8): the prior code cloned the name into an owned String even when
    // the format! call was unreachable.
    let title: Cow<'_, str> = match &unit.version {
        Some(v) => Cow::Owned(format!("{} v{}", unit.name, v)),
        None => Cow::Borrowed(unit.name.as_str()),
    };

    let title_truncated: Cow<'_, str> = if display_width(&title) > inner_width {
        Cow::Owned(truncate_to_width(&title, inner_width))
    } else {
        title
    };

    let path_truncated: Cow<'_, str> = if display_width(&unit.path) > inner_width {
        Cow::Owned(truncate_to_width(&unit.path, inner_width))
    } else {
        Cow::Borrowed(unit.path.as_str())
    };

    let desc_lines = wrap_text(
        unit.description.as_deref().unwrap_or(""),
        inner_width,
        CardLayoutConfig::CARD_DESC_LINES,
    );

    let top_border = format!("\u{256d}{}\u{256e}", "\u{2500}".repeat(inner_width));
    let bottom_border = format!("\u{2570}{}\u{256f}", "\u{2500}".repeat(inner_width));

    let mut lines = vec![top_border];
    lines.push(format!(
        "\u{2502}{}\u{2502}",
        tty_style(
            &pad_to_width_plain(&title_truncated, inner_width),
            cyan,
            is_tty
        )
    ));
    lines.push(format!(
        "\u{2502}{}\u{2502}",
        tty_style(
            &pad_to_width_plain(&path_truncated, inner_width),
            grey,
            is_tty
        )
    ));

    let empty_line = " ".repeat(inner_width);
    if let Some(stats_text) = build_card_stats_line(unit) {
        lines.push(format!(
            "\u{2502}{}\u{2502}",
            tty_style(&pad_to_width_plain(&stats_text, inner_width), dim, is_tty)
        ));
    } else {
        lines.push(format!("\u{2502}{}\u{2502}", empty_line));
    }

    for i in 0..CardLayoutConfig::CARD_DESC_LINES {
        let desc_line = desc_lines.get(i).map(|s| s.as_str()).unwrap_or("");
        let content = if desc_line.is_empty() {
            empty_line.clone()
        } else {
            tty_style(&pad_to_width_plain(desc_line, inner_width), white, is_tty)
        };
        lines.push(format!("\u{2502}{}\u{2502}", content));
    }

    lines.push(bottom_border);
    lines
}

/// Lay out cards using a width probed from stdout (TTY size, then `COLUMNS`,
/// then a hard-coded 120-column fallback).
///
/// ERR-1 (TASK-0784): the 120-column fallback fires silently when stdout is
/// not a TTY and `COLUMNS` is unset — piped invocations get layout sized for
/// a wide terminal regardless of caller intent. Reserve this entry point for
/// direct stdout renders. Buffer-writing callers must use
/// [`layout_cards_in_grid_with_width`] and supply a width that reflects the
/// destination they are rendering into.
pub fn layout_cards_in_grid(cards: &[Vec<String>]) -> Vec<String> {
    layout_cards_in_grid_with_width(cards, get_terminal_width())
}

/// Lay out a slice of pre-rendered cards into a grid sized for `term_width`.
///
/// READ-5 (TASK-0590) — Render contract:
///
/// The grid switches between 3, 2, and 1 cards per row at the
/// [`CardLayoutConfig::MIN_WIDTH_3_CARDS`] and
/// [`CardLayoutConfig::MIN_WIDTH_2_CARDS`] thresholds, but each card is
/// always rendered at fixed [`CardLayoutConfig::CARD_WIDTH`] (32) columns —
/// the underlying [`render_card`] is width-agnostic.
///
/// Minimum supported terminal width is therefore CARD_WIDTH + 2 (the leading
/// indent), i.e. 34 columns. Below that, single-card rows still render at
/// 32 columns and visibly overflow narrower terminals; the layout itself is
/// preserved (no truncated borders or mangled rows). The grid does not
/// further narrow CARD_WIDTH because render_card pre-computes its content
/// against a fixed inner-width and reflowing it would invalidate the
/// pre-rendered card lines.
pub fn layout_cards_in_grid_with_width(cards: &[Vec<String>], term_width: usize) -> Vec<String> {
    if cards.is_empty() {
        return vec![];
    }

    let cards_per_row = if term_width >= CardLayoutConfig::MIN_WIDTH_3_CARDS {
        3
    } else if term_width >= CardLayoutConfig::MIN_WIDTH_2_CARDS {
        2
    } else {
        1
    };

    let mut result = Vec::new();
    let spacing = " ".repeat(CardLayoutConfig::CARD_SPACING);

    for chunk in cards.chunks(cards_per_row) {
        let max_lines = chunk.iter().map(|c| c.len()).max().unwrap_or(0);

        for line_idx in 0..max_lines {
            // Borrow card lines as &str instead of cloning per cell; the
            // resulting `[&str].join(&str)` allocates once for the row String.
            let row_parts: Vec<&str> = chunk
                .iter()
                .map(|card| card.get(line_idx).map(String::as_str).unwrap_or(""))
                .collect();
            result.push(format!("  {}{}", row_parts.join(spacing.as_str()), spacing));
        }

        result.push(String::new());
    }

    if result.last().map(|s| s.is_empty()).unwrap_or(false) {
        result.pop();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(name: &str, path: &str) -> ProjectUnit {
        ProjectUnit {
            name: name.to_string(),
            path: path.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn format_unit_name_simple() {
        assert_eq!(format_unit_name("crate1"), "Crate1");
    }

    #[test]
    fn format_unit_name_with_path() {
        assert_eq!(format_unit_name("crates/aggregate"), "Aggregate");
    }

    #[test]
    fn format_unit_name_with_glob_prefix() {
        assert_eq!(format_unit_name("**/my-crate"), "My-crate");
    }

    #[test]
    fn format_unit_name_empty() {
        assert_eq!(format_unit_name(""), "");
    }

    #[test]
    fn render_card_with_loc() {
        let mut u = unit("My-lib", "crates/my-lib");
        u.version = Some("0.1.0".to_string());
        u.description = Some("A shared library".to_string());
        u.loc = Some(4231);
        let card = render_card(&u, false);
        assert!(card[3].contains("4,231 loc"));
    }

    #[test]
    fn render_card_without_stats() {
        let mut u = unit("My-lib", "crates/my-lib");
        u.version = Some("0.1.0".to_string());
        let card = render_card(&u, false);
        let inner = &card[3][3..card[3].len() - 3];
        assert!(inner.trim().is_empty());
    }

    #[test]
    fn build_card_stats_line_all_fields() {
        let mut u = unit("test", "test");
        u.loc = Some(1000);
        u.file_count = Some(10);
        u.dep_count = Some(3);
        let result = build_card_stats_line(&u).unwrap();
        assert!(result.contains("1,000 loc"));
        assert!(result.contains("10 files"));
        assert!(result.contains("3 deps"));
    }

    #[test]
    fn build_card_stats_line_file_count_singular() {
        let mut u = unit("t", "t");
        u.file_count = Some(1);
        assert_eq!(build_card_stats_line(&u).unwrap(), "1 file");
    }

    #[test]
    fn build_card_stats_line_none_when_empty() {
        assert!(build_card_stats_line(&unit("t", "t")).is_none());
    }

    #[test]
    fn render_card_no_version() {
        let u = unit("My-lib", "crates/my-lib");
        let card = render_card(&u, false);
        assert!(card[1].contains("My-lib"));
        assert!(!card[1].contains(" v"));
    }

    #[test]
    fn render_card_long_title_truncated() {
        let mut u = unit(&"A".repeat(40), "crates/long");
        u.version = Some("1.0.0".to_string());
        let card = render_card(&u, false);
        assert!(card[1].contains("\u{2026}"));
    }

    #[test]
    fn render_card_line_count() {
        let u = unit("Test", "test");
        let card = render_card(&u, false);
        assert_eq!(card.len(), 8);
    }

    #[test]
    fn layout_cards_empty() {
        assert!(layout_cards_in_grid(&[]).is_empty());
    }

    #[test]
    fn layout_cards_single() {
        let card = vec!["line1".to_string(), "line2".to_string()];
        let result = layout_cards_in_grid(&[card]);
        assert!(result.iter().any(|l| l.contains("line1")));
    }

    /// READ-5 (TASK-0590): below the minimum supported terminal width the
    /// grid stays in single-card mode and renders each card at the fixed
    /// CARD_WIDTH. Pin this so a future "responsive" refactor that picks
    /// 0-cards-per-row or panics on small terminals fails the test rather
    /// than the user.
    #[test]
    fn layout_cards_in_24col_terminal_uses_single_card_mode() {
        let card = render_card(&unit("T", "t"), false);
        let result = layout_cards_in_grid_with_width(std::slice::from_ref(&card), 24);
        assert!(!result.is_empty(), "must not return empty for narrow term");
        // Borders are preserved at fixed CARD_WIDTH (=32 inner cols + 2
        // border chars) regardless of terminal width.
        assert!(
            result.iter().any(|l| l.contains('\u{256d}')),
            "top border missing: {result:?}"
        );
        // Single-card mode: no two cards on the same line.
        let top_border_lines = result
            .iter()
            .filter(|l| l.matches('\u{256d}').count() > 0)
            .count();
        let cards_with_top = card.iter().filter(|l| l.contains('\u{256d}')).count();
        assert_eq!(top_border_lines, cards_with_top);
    }

    /// PERF-3 (TASK-0722): rendering 100+ cards must complete in trivial
    /// time. The pre-fix layout cloned every card line into an owned String
    /// per cell; this smoke test pins the borrow-friendly hot path.
    #[test]
    fn layout_cards_handles_large_workspace() {
        let cards: Vec<Vec<String>> = (0..150)
            .map(|i| {
                let name = format!("unit-{i}");
                render_card(&unit(&name, &format!("crates/{name}")), false)
            })
            .collect();
        let start = std::time::Instant::now();
        let result = layout_cards_in_grid_with_width(&cards, 120);
        let elapsed = start.elapsed();
        assert!(!result.is_empty());
        assert!(
            elapsed < std::time::Duration::from_millis(250),
            "layout_cards_in_grid_with_width should be fast for 150 units; took {elapsed:?}"
        );
    }

    #[test]
    fn layout_cards_multiple_cards() {
        let c1 = vec!["a1".to_string(), "a2".to_string()];
        let c2 = vec!["b1".to_string(), "b2".to_string()];
        let joined = layout_cards_in_grid(&[c1, c2]).join("\n");
        assert!(joined.contains("a1"));
        assert!(joined.contains("b1"));
    }
}
