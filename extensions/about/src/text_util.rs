//! Text utilities: formatting, padding, truncation, and wrapping.
//!
//! Stack-agnostic helpers used by about subpages across stacks.

use ops_core::output::{detect_terminal_width, display_width};
pub use ops_core::text::format_number;
use std::io::IsTerminal;
use unicode_segmentation::UnicodeSegmentation;

#[must_use]
pub fn get_terminal_width() -> usize {
    // ARCH-2 / TASK-0667: when stdout is a TTY, ask the OS for the real
    // window size first; `COLUMNS` is unset in many shells until the user
    // resizes once, and the previous 120-column fallback wrapped badly on
    // narrow terminals and under-utilised wide ones.
    if std::io::stdout().is_terminal() {
        if let Some(width) = detect_terminal_width() {
            return width;
        }
    }
    parse_terminal_width(std::env::var("COLUMNS").ok().as_deref())
}

/// Parse a COLUMNS-style width source.
///
/// Extracted from `get_terminal_width` so tests can exercise the parser
/// without mutating process-global env, which otherwise races with any
/// parallel test reading COLUMNS.
#[must_use]
pub fn parse_terminal_width(raw: Option<&str>) -> usize {
    raw.and_then(|s| s.parse().ok()).unwrap_or(120)
}

/// Trim an `Option<String>`'s body and drop empty results.
///
/// DUP-3 / TASK-1258: about-node and about-python both reimplemented this
/// helper verbatim (their per-site copies cite ERR-2 / TASK-0563 and
/// TASK-0566). Lifting it onto `ops_about` collapses the three drift surfaces
/// onto one policy — tightening the trim semantics (e.g. dropping
/// whitespace-only Unicode controls) lands once.
#[must_use]
pub fn trim_nonempty(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Detect ASCII / Unicode control characters in an attacker-controllable text
/// value (manifest URLs, repository fields, package metadata).
///
/// `char::is_control` matches the whole Unicode `Cc` category — C0
/// (`U+0000..=U+001F`), DEL (`U+007F`) **and** C1 (`U+0080..=U+009F`) — so it
/// is the complete test on its own; no separate DEL clause is required.
///
/// SEC-2 / TASK-1165 / TASK-1207: any control byte in such a value is treated
/// as evidence of tampering and the caller drops the field entirely rather
/// than stripping it. Stripping silently concatenates the attacker-controlled
/// tail (`https://demo.dev\nINJECT` → `https://demo.devINJECT`) into a
/// clickable URL; dropping surfaces the field as missing.
///
/// DUP-3 / TASK-1758: about-node and about-python each carried a verbatim copy
/// of this predicate for the same policy. Lifting it here keeps the
/// sanitisation boundary pinned at one source location, so a tightening lands
/// for every stack at once.
#[must_use]
pub fn contains_control_chars(raw: &str) -> bool {
    raw.chars().any(char::is_control)
}

/// URL schemes an About-card link may carry.
///
/// Deliberately minimal: these are the only two schemes that render as a safe
/// clickable link across the terminal (OSC 8 auto-linkification), markdown,
/// HTML and JSON surfaces the About card feeds.
const ALLOWED_URL_SCHEMES: [&str; 2] = ["https://", "http://"];

/// Whether `raw` carries an allowlisted URL scheme (`https://` or `http://`).
///
/// SEC-11 / TASK-1755 / TASK-1722: manifest URL fields are untrusted input —
/// `ops about` is routinely run against third-party checkouts. Without an
/// allowlist, `javascript:fetch(…)`, `data:text/html;base64,…`, `vbscript:x`
/// and `file:///etc/shadow` reach the rendered homepage / repository fields
/// and become clickable XSS or local-file-exfiltration links in any markdown
/// or HTML surface.
///
/// Scheme-less values (`example.com/x`, `//host/x`) are rejected too: guessing
/// a scheme for them would fabricate a link the manifest never declared.
/// Callers drop the field to `None` on rejection, extending the SEC-2 /
/// TASK-1207 drop-not-strip policy to the scheme axis — a partial or rewritten
/// URL is never emitted.
///
/// Scheme comparison is ASCII-case-insensitive, per RFC 3986 §3.1.
#[must_use]
pub fn has_allowed_url_scheme(raw: &str) -> bool {
    let trimmed = raw.trim_start();
    ALLOWED_URL_SCHEMES.iter().any(|scheme| {
        trimmed
            .get(..scheme.len())
            .is_some_and(|head| head.eq_ignore_ascii_case(scheme))
    })
}

#[must_use]
pub fn pad_to_width_plain(s: &str, width: usize) -> String {
    // PATTERN-1 / TASK-1001: delegate to `display_width` so emoji ZWJ
    // sequences (`👨‍👩‍👧`), regional-indicator flag pairs, and variation
    // selectors are accounted for at the cluster level. Char-summing
    // over-counted joiners / VS-16 glyphs and produced misaligned About
    // cards for unit names containing emoji.
    let current_width = display_width(s);
    if current_width >= width {
        s.to_string()
    } else {
        // `current_width < width` on this branch, so `saturating_sub` is
        // exactly `width - current_width`.
        format!("{}{}", s, " ".repeat(width.saturating_sub(current_width)))
    }
}

/// Truncate `s` to at most `max_width` display columns, appending a
/// `\u{2026}` (HORIZONTAL ELLIPSIS) when anything was cut.
///
/// READ-6 / TASK-1730: measures **grapheme clusters** with the same
/// cluster-aware [`display_width`] that [`pad_to_width_plain`] and
/// [`wrap_text`] use. It previously summed `unicode_width` per `char`, which
/// gave this module two disagreeing definitions of a column on the same
/// strings — `cards.rs` guards with `display_width` and then truncates with
/// this function. For a ZWJ sequence such as `👨‍👩‍👧` the per-`char` sum is
/// 2+0+2+0+2 = 6 where `display_width` reports 2, so two things went wrong:
///
/// 1. **Over-truncation.** A title the guard measured as fitting was cut at
///    up to a third of the available width.
/// 2. **Cluster splitting.** The per-`char` loop could break *inside* a ZWJ
///    sequence, emitting a lone `👨` plus a dangling joiner whose rendered
///    width is not the width the loop accounted for — re-introducing on the
///    truncation side exactly the card misalignment TASK-1001 fixed on the
///    padding side.
///
/// Iterating clusters fixes both at once: a cluster is kept whole or dropped
/// whole, and its width is measured the way the terminal renders it. ASCII is
/// unaffected — every ASCII `char` is its own one-column cluster.
#[must_use]
pub fn truncate_to_width(s: &str, max_width: usize) -> String {
    let mut result = String::new();
    let mut width: usize = 0;

    for cluster in s.graphemes(true) {
        let cluster_width = display_width(cluster);
        // `width` never exceeds `max_width - 1` (the accumulate below only runs
        // when this test is false), so the sum is exact for every `max_width`
        // short of `usize::MAX`; at that extreme saturation still takes the
        // same branch the checked sum would.
        if width.saturating_add(cluster_width) > max_width.saturating_sub(1) {
            result.push('\u{2026}');
            break;
        }
        result.push_str(cluster);
        width = width.saturating_add(cluster_width);
    }

    result
}

// TEST-15 / TASK-1664: counts the characters `wrap_text` hands to
// `display_width`.
//
// The measurement contract used to be pinned by a wall-clock ratio — a 10x
// input had to wrap in under 20x the time. That assertion was load-dependent in
// a debug build, and it could not have caught the regression it guarded twice
// over: `max_lines` breaks the word loop after at most that many lines, so both
// input sizes did nearly the same work; and the regression itself is a constant
// factor rather than a growth change (see the seam below), which no ratio
// bound can see. Counting the characters measured pins the property directly.
//
// **Thread-local**, not a process-wide counter, for the reason given at
// `runner::command::resolve::STORE_WALKS`: test binaries run their tests in
// parallel threads, so a global counter is incremented by unrelated tests
// between a reader's two observations.
//
// Documented with `//` rather than `///`: a doc comment on a macro invocation
// is dropped by rustdoc, which `-D unused-doc-comments` rejects.
#[cfg(test)]
thread_local! {
    static WIDTH_CHARS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Single seam for every width measurement [`wrap_text`] makes, so a
/// regression that re-measures the accumulated line once per word — the shape
/// TASK-0709 fixed — shows up in the count above.
fn measure_width(s: &str) -> usize {
    #[cfg(test)]
    // Test-only tally over in-memory strings; the count cannot approach
    // `usize::MAX`, so saturation is unreachable.
    WIDTH_CHARS.with(|c| c.set(c.get().saturating_add(s.chars().count())));
    display_width(s)
}

/// Read the characters measured since the last call and reset the counter.
#[cfg(test)]
fn take_width_chars() -> usize {
    WIDTH_CHARS.with(|c| c.replace(0))
}

/// Wrap `text` into at most `max_lines` lines whose `display_width` is at
/// most `max_width`.
///
/// # Truncation behaviour
///
/// PATTERN-1 / TASK-1105: when wrapping exceeds `max_lines`, the function
/// **does not silently drop** the trailing content. The last emitted line
/// has a `\u{2026}` (HORIZONTAL ELLIPSIS) appended (replacing its tail if
/// necessary to stay within `max_width`) so callers can tell at a glance
/// that the rendered description is truncated. An unbreakable word wider
/// than `max_width` is still truncated by `truncate_to_width` per the
/// READ-5 / TASK-0550 contract; the ellipsis policy here applies to the
/// "ran out of lines" case.
///
/// Empty input or `max_lines == 0` returns an empty vector unchanged.
#[must_use]
pub fn wrap_text(text: &str, max_width: usize, max_lines: usize) -> Vec<String> {
    if text.is_empty() || max_lines == 0 {
        return vec![];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();
    // Track current line width incrementally rather than re-scanning
    // `current_line` via display_width on every iteration. Scanning cost up to
    // `max_width` columns per word — bounded, so the growth stayed linear, but
    // the constant was 5.6x on the shape `wrap_text_measures_each_word_and_
    // line_exactly_once` exercises (TASK-1664 measured it; the original
    // TASK-0709 note called this O(N^2), which overstated it).
    let mut current_width: usize = 0;
    // PATTERN-1 / TASK-1105: track whether the input had more words than
    // `max_lines` could accommodate, so the post-loop fixup can append an
    // ellipsis to the last emitted line. Without this, the trailing word
    // pushed into `current_line` after the `max_lines` cap was reached was
    // dropped silently.
    let mut truncated = false;

    for word in text.split_whitespace() {
        let word_width = measure_width(word);

        if current_line.is_empty() {
            current_line.push_str(word);
            current_width = word_width;
        } else if current_width.saturating_add(1).saturating_add(word_width) <= max_width {
            // Reached only when the sum above fits in `max_width`, so this
            // accumulation is exact.
            current_line.push(' ');
            current_line.push_str(word);
            current_width = current_width.saturating_add(1).saturating_add(word_width);
        } else {
            lines.push(std::mem::take(&mut current_line));
            current_line.push_str(word);
            current_width = word_width;

            if lines.len() >= max_lines {
                // The just-pushed word lives in `current_line` but cannot
                // be emitted as a new line — record that tail content was
                // dropped so the last line gets an ellipsis below.
                truncated = true;
                break;
            }
        }
    }

    if !current_line.is_empty() && lines.len() < max_lines {
        lines.push(current_line);
    } else if !current_line.is_empty() {
        // Non-empty `current_line` with no room left: we are about to drop
        // it, which is exactly the silent-truncation case TASK-1105 fixes.
        truncated = true;
    }

    lines.truncate(max_lines);

    // PATTERN-1 / TASK-1105: mark the last emitted line as truncated. We
    // append `\u{2026}` and trim from the end if the resulting display
    // width would exceed max_width, so the contract (every line <=
    // max_width) is preserved. `truncate_to_width` already implements that
    // shape — feed it the line-plus-ellipsis "intent" and it produces a
    // single trailing ellipsis at most max_width columns wide.
    if truncated {
        if let Some(last) = lines.last_mut() {
            // Avoid double-ellipsis if the line already ends in U+2026
            // (e.g. an unbreakable word that was truncated mid-emit).
            if !last.ends_with('\u{2026}') {
                let with_ellipsis = format!("{last}\u{2026}");
                *last = if measure_width(&with_ellipsis) <= max_width {
                    with_ellipsis
                } else {
                    truncate_to_width(last, max_width)
                };
            }
        }
    }

    // READ-5 (TASK-0550): an unbreakable word wider than max_width is pushed
    // verbatim when current_line is empty and may land on an intermediate
    // line, so truncating only the last line lets earlier lines exceed the
    // contract. Enforce display_width(line) <= max_width on every line.
    for line in &mut lines {
        if measure_width(line) > max_width.saturating_sub(1) {
            *line = truncate_to_width(line, max_width);
        }
    }

    lines
}

pub fn tty_style(
    text: &str,
    styler: fn(&str) -> std::borrow::Cow<'_, str>,
    is_tty: bool,
) -> String {
    if is_tty {
        styler(text).into_owned()
    } else {
        text.to_string()
    }
}

/// Pad `left` and `right` with spaces so they span a content area of
/// `target_content_width` columns (right-aligned right string, one trailing space).
#[must_use]
pub fn pad_header(left: &str, right: &str, target_content_width: usize) -> String {
    let left_display = display_width(left);
    let right_display = display_width(right);
    // PATTERN-1 / TASK-1115: when `left + right + 1` exceeds the target content
    // width, `saturating_sub` pins padding at 0 and the formatted result loses
    // its whitespace separator entirely (`<left><right> `), making two
    // adjacent identifiers look like a single concatenated token. Floor the
    // separator at one space so the contract — "right-aligned right string,
    // one trailing space" — at minimum keeps the halves visually distinct
    // even when the card is narrower than the header content.
    let padding = target_content_width
        .saturating_sub(left_display.saturating_add(right_display).saturating_add(1))
        .max(1);
    format!("{}{}{} ", left, " ".repeat(padding), right)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::output::display_width;

    /// DUP-3 / TASK-1758: the per-provider copies of this predicate carried a
    /// redundant `|| c == '\u{007f}'` clause, implying `char::is_control` had
    /// a gap at DEL. It does not — `Cc` spans C0, DEL and C1 — so dropping
    /// the clause must not change behaviour at any of those three points.
    #[test]
    fn contains_control_chars_covers_c0_del_and_c1() {
        assert!(contains_control_chars("https://demo.dev\nINJECT"));
        assert!(contains_control_chars("https://demo.dev\u{1b}[31m"));
        assert!(
            contains_control_chars("https://demo.dev\u{007f}"),
            "DEL must still be rejected without the explicit clause"
        );
        assert!(
            contains_control_chars("https://demo.dev\u{0085}"),
            "C1 (NEL) must be rejected"
        );
        assert!(!contains_control_chars("https://demo.dev/owner/repo"));
    }

    /// SEC-11 / TASK-1755: only `http(s)` reach a rendered About link.
    #[test]
    fn has_allowed_url_scheme_allowlists_only_http_and_https() {
        assert!(has_allowed_url_scheme("https://demo.dev"));
        assert!(has_allowed_url_scheme("http://demo.dev"));
        assert!(has_allowed_url_scheme("HTTPS://demo.dev"));
        assert!(!has_allowed_url_scheme(
            "javascript:fetch('https://evil.tld')"
        ));
        assert!(!has_allowed_url_scheme("data:text/html;base64,AAAA"));
        assert!(!has_allowed_url_scheme("file:///etc/shadow"));
        assert!(!has_allowed_url_scheme("vbscript:x"));
        assert!(!has_allowed_url_scheme("ftp://demo.dev"));
    }

    /// A scheme-less value must be rejected rather than guessed at.
    #[test]
    fn has_allowed_url_scheme_rejects_scheme_less_and_short_values() {
        assert!(!has_allowed_url_scheme("example.com/x"));
        assert!(!has_allowed_url_scheme("//example.com/x"));
        assert!(!has_allowed_url_scheme(""));
        assert!(!has_allowed_url_scheme("http"));
        // Multi-byte prefix shorter than the scheme must not panic on slicing.
        assert!(!has_allowed_url_scheme("é"));
    }

    #[test]
    fn truncate_to_width_short_string() {
        assert_eq!(truncate_to_width("hello", 10), "hello");
    }

    #[test]
    fn truncate_to_width_exact_fit() {
        assert_eq!(truncate_to_width("hello", 5), "hell\u{2026}");
    }

    #[test]
    fn truncate_to_width_needs_truncation() {
        assert_eq!(truncate_to_width("hello world", 6), "hello\u{2026}");
    }

    #[test]
    fn truncate_to_width_very_short_max() {
        assert_eq!(truncate_to_width("hello", 1), "\u{2026}");
    }

    #[test]
    fn truncate_to_width_empty() {
        assert_eq!(truncate_to_width("", 10), "");
    }

    #[test]
    fn wrap_text_single_line() {
        assert_eq!(wrap_text("hello world", 20, 2), vec!["hello world"]);
    }

    #[test]
    fn wrap_text_multiple_lines() {
        let result = wrap_text("one two three four five", 10, 3);
        assert!(result.len() <= 3);
        for line in &result {
            assert!(display_width(line) <= 10);
        }
    }

    #[test]
    fn wrap_text_respects_max_lines() {
        let result = wrap_text("one two three four five six seven eight", 5, 2);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn wrap_text_empty() {
        assert!(wrap_text("", 10, 2).is_empty());
    }

    #[test]
    fn wrap_text_max_lines_zero() {
        assert!(wrap_text("hello world", 20, 0).is_empty());
    }

    #[test]
    fn wrap_text_long_word_exceeds_width() {
        let result = wrap_text("superlongword short", 5, 3);
        assert!(!result.is_empty());
    }

    /// PATTERN-1 / TASK-1105: when `max_lines` runs out mid-iteration the
    /// previous implementation silently dropped the word that would have
    /// started the next line. Pin the new policy: the last emitted line
    /// ends with `\u{2026}` so callers can tell content was truncated.
    ///
    /// `max_width` is set to 8 here (bigger than the longest word, 5) so
    /// the READ-5 / TASK-0550 "intermediate-line >= max_width-1" pass does
    /// not also truncate the otherwise-unchanged words and obscure the
    /// signal we are pinning.
    #[test]
    fn wrap_text_truncates_with_ellipsis_when_max_lines_exceeded() {
        let result = wrap_text("alpha beta gamma delta", 8, 2);
        assert_eq!(
            result,
            vec!["alpha".to_string(), "beta\u{2026}".to_string()],
            "TASK-1105: trailing word must drive an ellipsis on the last line"
        );
        for line in &result {
            assert!(display_width(line) <= 8);
        }
    }

    /// PATTERN-1 / TASK-1105: when the input fits within `max_lines` exactly
    /// no ellipsis must be appended — only signal truncation when tail
    /// content was actually dropped.
    #[test]
    fn wrap_text_no_ellipsis_when_input_fits() {
        let result = wrap_text("alpha beta", 8, 2);
        assert_eq!(result, vec!["alpha", "beta"]);
        assert!(result.iter().all(|l| !l.ends_with('\u{2026}')));
    }

    /// PERF-1 (TASK-0709) + TEST-15 (TASK-1044, TASK-1152, TASK-1664):
    /// wrapping must measure each word once and each emitted line once, and
    /// must never re-measure the line it is accumulating. Pre-fix,
    /// `display_width(&current_line)` was called for every word.
    ///
    /// TASK-1664 replaces the wall-clock ratio this used to assert (a 10x
    /// input under 20x the time), which was wrong twice over. It timed a debug
    /// build, so it failed under CPU contention — the normal state of a shared
    /// runner. And it could not have caught the regression it guarded:
    /// `max_lines` breaks the word loop after 50 lines, so the 1k-word and
    /// 10k-word inputs consumed the same ~800 words and the ratio sat near 1
    /// whatever the implementation did.
    ///
    /// Note the old `O(N^2)` framing overstated the bug. `current_line` is
    /// capped at `max_width`, so re-scanning it per word costs a bounded
    /// constant per word — the growth stays linear and only the constant
    /// moves (measured: 5.6x the characters). That is precisely the class of
    /// regression a growth-ratio assertion cannot see, whatever its bound, so
    /// the count is asserted exactly rather than compared between two sizes.
    /// Re-introducing the call reports 80,621 characters against 14,300.
    #[test]
    fn wrap_text_measures_each_word_and_line_exactly_once() {
        // 16 four-character words fit an 80-column line (16*4 + 15 spaces =
        // 79), so both sizes below are multiples of 16 and pack into whole
        // lines. `max_lines` is set past the resulting line count so neither
        // run truncates — truncation caps the work and would hide a
        // regression behind the early break.
        fn assert_exact_ledger(words: usize) {
            let text = std::iter::repeat_n("word", words)
                .collect::<Vec<_>>()
                .join(" ");
            let _ = take_width_chars();
            let lines = wrap_text(&text, 80, 10_000);
            assert_eq!(
                lines.len(),
                words / 16,
                "input must pack into whole lines without truncating"
            );

            let measured = take_width_chars();
            // Every word, once (the per-word width), plus every emitted line,
            // once (the READ-5 over-width sweep). Derived from the actual
            // output rather than hardcoded, so the ledger stays honest if the
            // packing changes.
            let expected = words * 4 + lines.iter().map(|l| l.chars().count()).sum::<usize>();
            assert_eq!(
                measured, expected,
                "wrap_text measured {measured} characters for {words} words \
                 where {expected} accounts for every word and every emitted \
                 line exactly once. Measuring the accumulated line per word — \
                 the TASK-0709 regression — costs up to `max_width` extra \
                 characters per word."
            );
        }

        assert_exact_ledger(1_600);
        assert_exact_ledger(16_000);
    }

    #[test]
    fn wrap_text_truncates_intermediate_overlong_lines() {
        let result = wrap_text(
            "https://example.com/very/long/path-that-overflows aa bb",
            10,
            3,
        );
        for line in &result {
            assert!(
                display_width(line) <= 10,
                "line {line:?} exceeds max_width 10 (got {})",
                display_width(line)
            );
        }
    }

    #[test]
    fn pad_to_width_adds_padding() {
        assert_eq!(pad_to_width_plain("hi", 5).len(), 5);
    }

    /// PATTERN-1 / TASK-1001: a string with an emoji ZWJ sequence
    /// (`👨‍👩‍👧`) must be padded based on `display_width` (cluster-aware),
    /// matching how the rest of the `about/text_util` module measures text.
    /// Char-summing over-counted the joiner / VS-16 glyphs and produced
    /// off-by-one cards under TTY rendering.
    #[test]
    fn pad_to_width_uses_display_width_for_zwj_sequence() {
        let s = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}"; // 👨‍👩‍👧
        let target = display_width(s) + 4;
        let padded = pad_to_width_plain(s, target);
        assert_eq!(
            display_width(&padded),
            target,
            "padded display_width must equal target; got padded={padded:?}"
        );
        assert!(padded.starts_with(s));
    }

    #[test]
    fn pad_to_width_already_wide() {
        assert_eq!(pad_to_width_plain("hello", 3), "hello");
    }

    /// READ-6 / TASK-1730: the truncation counterpart of
    /// `pad_to_width_uses_display_width_for_zwj_sequence`. A ZWJ family emoji
    /// is one two-column cluster; the old per-`char` sum measured it as six
    /// columns and could break between the joiner and its neighbour.
    #[test]
    fn truncate_to_width_keeps_zwj_cluster_whole() {
        const FAMILY: &str = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
        // Sanity: the two width models genuinely disagree on this input, or
        // the test is not covering the regression.
        assert_eq!(display_width(FAMILY), 2);
        assert_eq!(FAMILY.chars().count(), 5);

        for max_width in 1..=12 {
            let out = truncate_to_width(&format!("ab{FAMILY}cd"), max_width);
            assert!(
                display_width(&out) <= max_width,
                "truncate_to_width({max_width}) produced {out:?} of width {}",
                display_width(&out)
            );
            // Kept whole or dropped whole: the cluster's interior code points
            // must never appear without the full sequence around them.
            let joiners = out.matches('\u{200D}').count();
            assert!(
                out.contains(FAMILY) || joiners == 0,
                "truncate_to_width({max_width}) split the ZWJ cluster: {out:?}"
            );
        }
    }

    /// READ-6 / TASK-1730: with enough room the whole cluster survives, and
    /// with room for the prefix but not the cluster it is dropped entirely
    /// rather than cut in half.
    #[test]
    fn truncate_to_width_drops_or_keeps_the_whole_cluster() {
        const FAMILY: &str = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
        let input = format!("ab{FAMILY}");
        assert_eq!(display_width(&input), 4);

        // The function always reserves one column for a possible ellipsis, so
        // content is kept up to `max_width - 1`. At 5 the whole 4-column
        // input fits inside that budget and comes back untouched — this is
        // where the old char-summing model failed, counting the cluster as 6
        // columns and cutting after "a".
        assert_eq!(truncate_to_width(&input, 5), input);

        // At 4 the budget is 3: "ab" fits, the 2-column cluster does not, and
        // it is dropped whole rather than cut open.
        let tight = truncate_to_width(&input, 4);
        assert_eq!(tight, "ab\u{2026}");
        assert!(!tight.contains('\u{200D}'), "cluster must not be split");
        assert!(display_width(&tight) <= 4);
    }

    #[test]
    fn tty_style_applies_when_tty() {
        // READ-9/TASK-0950: ops_core::style helpers self-gate on
        // stdout TTY + NO_COLOR, so tests that need to observe the
        // styled-branch behaviour of tty_style use a local styler.
        fn force_styler(s: &str) -> std::borrow::Cow<'_, str> {
            std::borrow::Cow::Owned(format!("[{s}]"))
        }
        let styled = tty_style("hello", force_styler, true);
        assert_eq!(styled, "[hello]");
    }

    #[test]
    fn tty_style_passthrough_when_not_tty() {
        fn force_styler(s: &str) -> std::borrow::Cow<'_, str> {
            std::borrow::Cow::Owned(format!("[{s}]"))
        }
        assert_eq!(tty_style("hello", force_styler, false), "hello");
    }

    #[test]
    fn parse_terminal_width_default_when_unset() {
        assert_eq!(parse_terminal_width(None), 120);
    }

    #[test]
    fn parse_terminal_width_default_when_unparseable() {
        assert_eq!(parse_terminal_width(Some("not-a-number")), 120);
    }

    #[test]
    fn parse_terminal_width_uses_explicit_value() {
        assert_eq!(parse_terminal_width(Some("80")), 80);
    }

    #[test]
    fn pad_header_balances_left_and_right() {
        let result = pad_header("Left", "Right", 100);
        assert!(result.starts_with("Left"));
        assert!(result.ends_with("Right "));
        assert!(result.len() <= 100);
    }

    #[test]
    fn pad_header_long_strings() {
        let left = "A".repeat(60);
        let right = "B".repeat(60);
        let result = pad_header(&left, &right, 100);
        assert!(result.contains(&left));
        assert!(result.contains(&right));
    }

    /// PATTERN-1 / TASK-1115: when `left + right + 1` overflows
    /// `target_content_width`, `pad_header` previously returned
    /// `<left><right> ` with no whitespace between the two halves, making
    /// two adjacent identifiers look concatenated. Guarantee at least one
    /// space separator regardless of width.
    #[test]
    fn pad_header_preserves_separator_on_overflow() {
        let left = "Foo";
        let right = "BarValue";
        // target_content_width is far smaller than left+right+1, forcing the
        // overflow / saturating-sub branch.
        let result = pad_header(left, right, 4);
        let between = result
            .get(left.len()..result.len() - right.len() - 1)
            .expect("result is the left half, a separator, then the right half");
        assert!(
            between.contains(' '),
            "expected at least one space between left and right halves; got {result:?}"
        );
        assert!(result.starts_with(left));
        assert!(result.ends_with("BarValue "));
    }
}
