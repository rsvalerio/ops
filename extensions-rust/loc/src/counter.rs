//! Rust-aware line classification via `proc_macro2` (lexing) and `syn`
//! (structure).
//!
//! # Why not a hand-rolled scanner
//!
//! Rust's lexer discards `//` and `/* */` comments entirely, so anything
//! *between* two tokens is only whitespace or a comment. That is a
//! guarantee of the grammar, not a heuristic. By tokenising with
//! `proc_macro2` and treating the gaps as comment/blank, raw strings
//! (`r#"// not a comment"#`), nested block comments, and `//` inside
//! string literals are handled by the upstream lexer instead of by us.
//!
//! Test attribution comes from `syn`, which parses `#[cfg(...)]`
//! predicates properly. A textual matcher has to string-compare
//! `"#[cfg(test)]"` and therefore misses `#[cfg(all(test, unix))]`,
//! `#[cfg_attr(test, ...)]`, and `#[cfg( test )]`.
//!
//! # Known limits
//!
//! - Tests generated *by* a macro (e.g. `test_create_sql_validation!`)
//!   are attributed to the invocation line, which is main code. No tool
//!   in this space can see through macro expansion; `ops` itself has
//!   several such sites, so its own numbers under-report test lines.
//! - Only top-level items and items inside inline `mod` blocks are
//!   inspected for test gates. A `#[cfg(test)]` on a statement inside a
//!   function body is attributed to the enclosing region.
//! - A file `proc_macro2` cannot lex, or `syn` cannot parse, falls back
//!   to [`count_fallback`]: blank vs non-blank only, all attributed to
//!   the file-level region.

use std::path::Path;
use std::str::FromStr;

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;

/// How a single source line is classified.
///
/// Ordered so that `max` implements the precedence rule: a line holding
/// both code and a trailing comment counts as code, matching the
/// convention used by `tokei` and every other counter.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum LineKind {
    Blank,
    Comment,
    Doc,
    Code,
}

/// Which bucket a line's counts land in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Region {
    Main,
    Test,
    Example,
}

impl Region {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Test => "test",
            Self::Example => "example",
        }
    }
}

/// Line counts for one region of one file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Locs {
    pub code: u64,
    pub docs: u64,
    pub comments: u64,
    pub blanks: u64,
}

impl Locs {
    #[must_use]
    pub const fn lines(&self) -> u64 {
        self.code + self.docs + self.comments + self.blanks
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.lines() == 0
    }

    fn add(&mut self, kind: LineKind) {
        match kind {
            LineKind::Code => self.code += 1,
            LineKind::Doc => self.docs += 1,
            LineKind::Comment => self.comments += 1,
            LineKind::Blank => self.blanks += 1,
        }
    }
}

/// Per-file counts, split by region.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct FileCounts {
    pub main: Locs,
    pub test: Locs,
    pub example: Locs,
}

impl FileCounts {
    fn bucket_mut(&mut self, region: Region) -> &mut Locs {
        match region {
            Region::Main => &mut self.main,
            Region::Test => &mut self.test,
            Region::Example => &mut self.example,
        }
    }

    /// Iterate the non-empty regions, for row emission.
    pub fn non_empty(&self) -> impl Iterator<Item = (Region, Locs)> {
        [
            (Region::Main, self.main),
            (Region::Test, self.test),
            (Region::Example, self.example),
        ]
        .into_iter()
        .filter(|(_, locs)| !locs.is_empty())
    }
}

/// Classify a file's base region from its path.
///
/// `syn` resolves `#[cfg(test)]` *within* a file; it cannot know that
/// `src/tests.rs` is reached through a `#[cfg(test)] mod tests;`
/// declaration in the parent. Path convention covers that gap, matching
/// cargo's own layout rules.
#[must_use]
pub fn region_from_path(path: &Path) -> Region {
    for component in path.components() {
        let std::path::Component::Normal(name) = component else {
            continue;
        };
        if name == "examples" {
            return Region::Example;
        }
        if name == "tests" || name == "benches" || name == "tests.rs" {
            return Region::Test;
        }
    }
    Region::Main
}

/// Classify every line of `src`, attributing to `base` unless `syn`
/// marks a span as test-gated.
///
/// Never fails: unparseable input degrades to [`count_fallback`] rather
/// than dropping the file, so a single nightly-only syntax file cannot
/// zero out a whole crate's numbers.
#[must_use]
pub fn count_source(src: &str, base: Region) -> FileCounts {
    let lines: Vec<&str> = src.lines().collect();
    let line_count = lines.len();
    if line_count == 0 {
        return FileCounts::default();
    }

    let Ok(stream) = TokenStream::from_str(src) else {
        return count_fallback(src, base);
    };

    let mut kinds = vec![LineKind::Blank; line_count];
    mark_tokens(&stream, &lines, &mut kinds);

    // Any line still Blank was not covered by a token, so it lies in a
    // gap: whitespace or a comment, nothing else is possible.
    for (idx, kind) in kinds.iter_mut().enumerate() {
        if *kind == LineKind::Blank && !lines[idx].trim().is_empty() {
            *kind = LineKind::Comment;
        }
    }

    let mut regions = vec![base; line_count];
    if base == Region::Main {
        if let Ok(file) = syn::parse_file(src) {
            mark_test_items(&file.items, &mut regions);
        }
    }

    let mut counts = FileCounts::default();
    for (idx, kind) in kinds.into_iter().enumerate() {
        counts.bucket_mut(regions[idx]).add(kind);
    }
    counts
}

/// Degraded counting for input the lexer or parser rejects.
#[must_use]
pub fn count_fallback(src: &str, base: Region) -> FileCounts {
    let mut counts = FileCounts::default();
    let bucket = counts.bucket_mut(base);
    for line in src.lines() {
        if line.trim().is_empty() {
            bucket.blanks += 1;
        } else {
            bucket.code += 1;
        }
    }
    counts
}

// -- token walking --

/// Mark every line touched by a token as [`LineKind::Code`], except doc
/// comments, which the lexer rewrites into `#[doc = "..."]` attributes
/// and which are reclassified as [`LineKind::Doc`].
fn mark_tokens(stream: &TokenStream, lines: &[&str], kinds: &mut [LineKind]) {
    let mut iter = stream.clone().into_iter().peekable();

    while let Some(tt) = iter.next() {
        // A doc comment reaches us as `#` `[` `doc` `=` lit `]` (or with
        // a leading `!` for inner docs). Every synthesised token carries
        // the span of the original comment, so we detect it by looking
        // at what the source actually says at the `#`.
        if let TokenTree::Punct(ref punct) = tt {
            if punct.as_char() == '#' && starts_comment(punct.span(), lines) {
                let start = punct.span().start().line;
                if let Some(end) = consume_doc_attr(&mut iter) {
                    mark_range(start, end, kinds, LineKind::Doc);
                } else {
                    mark_span_line_range(punct.span(), kinds, LineKind::Code);
                }
                continue;
            }
        }

        match tt {
            TokenTree::Group(group) => {
                // A group's own span covers its entire contents, so
                // using it directly would swallow comments nested
                // inside the braces. Mark only the delimiters and
                // recurse.
                mark_span_line_range(group.span_open(), kinds, LineKind::Code);
                mark_tokens(&group.stream(), lines, kinds);
                mark_span_line_range(group.span_close(), kinds, LineKind::Code);
            }
            other => mark_span_line_range(other.span(), kinds, LineKind::Code),
        }
    }
}

/// Does the source at `span`'s start begin a comment?
///
/// Distinguishes a real `///` doc comment from a hand-written
/// `#[doc = "..."]`, which should count as code.
fn starts_comment(span: proc_macro2::Span, lines: &[&str]) -> bool {
    let start = span.start();
    let Some(line) = lines.get(start.line.saturating_sub(1)) else {
        return false;
    };
    // `LineColumn::column` counts characters, not bytes.
    let mut chars = line.chars().skip(start.column);
    chars.next() == Some('/') && matches!(chars.next(), Some('/' | '*'))
}

/// Consume the `[!] [ doc = "..." ]` tail of a doc-comment attribute,
/// returning the last line it covers.
fn consume_doc_attr(
    iter: &mut std::iter::Peekable<proc_macro2::token_stream::IntoIter>,
) -> Option<usize> {
    if let Some(TokenTree::Punct(punct)) = iter.peek() {
        if punct.as_char() == '!' {
            iter.next();
        }
    }
    match iter.peek() {
        Some(TokenTree::Group(group)) if group.delimiter() == proc_macro2::Delimiter::Bracket => {
            let end = group.span_close().end().line;
            iter.next();
            Some(end)
        }
        _ => None,
    }
}

fn mark_span_line_range(span: proc_macro2::Span, kinds: &mut [LineKind], kind: LineKind) {
    mark_range(span.start().line, span.end().line, kinds, kind);
}

/// Apply `kind` to every line in the inclusive, 1-based range, keeping
/// the strongest classification already recorded.
fn mark_range(start_line: usize, end_line: usize, kinds: &mut [LineKind], kind: LineKind) {
    let (Some(start), Some(end)) = (start_line.checked_sub(1), end_line.checked_sub(1)) else {
        return;
    };
    let end = end.min(kinds.len().saturating_sub(1));
    for slot in kinds.iter_mut().take(end + 1).skip(start) {
        *slot = (*slot).max(kind);
    }
}

// -- test-span detection --

fn mark_test_items(items: &[syn::Item], regions: &mut [Region]) {
    for item in items {
        if item_is_test_gated(item) {
            if let Some((start, end)) = item_line_range(item) {
                let last = end.min(regions.len());
                for slot in regions.iter_mut().take(last).skip(start.saturating_sub(1)) {
                    *slot = Region::Test;
                }
            }
            continue;
        }
        // Not gated: an inner module may still hold gated items.
        if let syn::Item::Mod(module) = item {
            if let Some((_, nested)) = &module.content {
                mark_test_items(nested, regions);
            }
        }
    }
}

fn item_is_test_gated(item: &syn::Item) -> bool {
    item_attrs(item).is_some_and(|attrs| attrs.iter().any(attr_is_test_gate))
}

/// Is this attribute a test gate?
///
/// Accepts `#[test]` and any framework variant whose final path segment
/// is `test` (`#[tokio::test]`), plus `#[cfg(..)]` / `#[cfg_attr(..)]`
/// whose predicate mentions the bare `test` ident.
///
/// Scanning for a bare ident is deliberately not the same as a
/// substring match: `cfg(feature = "test")` carries `test` as a string
/// *literal*, not an ident, so it correctly does not match. A `not(..)`
/// group is skipped for the same reason — `#[cfg(not(test))]` gates code
/// *out* of test builds, so it is production code.
fn attr_is_test_gate(attr: &syn::Attribute) -> bool {
    let path = attr.path();
    if path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "test")
    {
        return true;
    }
    if path.is_ident("cfg") || path.is_ident("cfg_attr") {
        return stream_has_test_ident(&attr.to_token_stream());
    }
    false
}

fn stream_has_test_ident(stream: &TokenStream) -> bool {
    let mut iter = stream.clone().into_iter().peekable();
    while let Some(tt) = iter.next() {
        match tt {
            TokenTree::Ident(ident) => {
                if ident == "not" {
                    // Consume the inverted predicate wholesale.
                    if matches!(iter.peek(), Some(TokenTree::Group(_))) {
                        iter.next();
                    }
                } else if ident == "test" {
                    return true;
                }
            }
            TokenTree::Group(group) => {
                if stream_has_test_ident(&group.stream()) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Inclusive 1-based line range spanned by an item, attributes included.
///
/// Computed by folding over the item's own tokens rather than via
/// `syn::spanned::Spanned`, which relies on `Span::join` and returns
/// only the first token's span when joining is unavailable.
fn item_line_range(item: &syn::Item) -> Option<(usize, usize)> {
    let mut range: Option<(usize, usize)> = None;
    fold_span_range(&item.to_token_stream(), &mut range);
    range
}

fn fold_span_range(stream: &TokenStream, range: &mut Option<(usize, usize)>) {
    for tt in stream.clone() {
        match tt {
            TokenTree::Group(group) => {
                extend(range, group.span_open());
                fold_span_range(&group.stream(), range);
                extend(range, group.span_close());
            }
            other => extend(range, other.span()),
        }
    }
}

fn extend(range: &mut Option<(usize, usize)>, span: proc_macro2::Span) {
    let (start, end) = (span.start().line, span.end().line);
    *range = Some(match *range {
        Some((lo, hi)) => (lo.min(start), hi.max(end)),
        None => (start, end),
    });
}

fn item_attrs(item: &syn::Item) -> Option<&Vec<syn::Attribute>> {
    Some(match item {
        syn::Item::Const(i) => &i.attrs,
        syn::Item::Enum(i) => &i.attrs,
        syn::Item::ExternCrate(i) => &i.attrs,
        syn::Item::Fn(i) => &i.attrs,
        syn::Item::ForeignMod(i) => &i.attrs,
        syn::Item::Impl(i) => &i.attrs,
        syn::Item::Macro(i) => &i.attrs,
        syn::Item::Mod(i) => &i.attrs,
        syn::Item::Static(i) => &i.attrs,
        syn::Item::Struct(i) => &i.attrs,
        syn::Item::Trait(i) => &i.attrs,
        syn::Item::TraitAlias(i) => &i.attrs,
        syn::Item::Type(i) => &i.attrs,
        syn::Item::Union(i) => &i.attrs,
        syn::Item::Use(i) => &i.attrs,
        _ => return None,
    })
}
