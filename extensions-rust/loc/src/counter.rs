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
//! `"#[cfg(test)]"` and therefore misses `#[cfg(all(test, unix))]` and
//! `#[cfg( test )]`, while over-matching `#[cfg_attr(test, ...)]`,
//! which does not gate the item out of a non-test build at all.
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
//! - Delimiter nesting is bounded at [`MAX_NESTING_DEPTH`]. The token
//!   walkers recurse once per nesting level, so a pathologically nested
//!   file would overflow the stack — which aborts the process with
//!   `SIGSEGV` and cannot be caught. Anything deeper than the cap warns
//!   and falls back to [`count_fallback`] instead, keeping the
//!   never-fails contract for the scan as a whole.

use std::path::Path;
use std::str::FromStr;

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;

/// Maximum delimiter nesting depth the token walkers will descend.
///
/// `mark_tokens`, `stream_has_test_ident` and `fold_span_range` recurse
/// once per `()`/`[]`/`{}` level. `proc_macro2`'s own lexer is
/// deliberately iterative (it even ships a hand-written non-recursive
/// `Drop` for `TokenStream`), so a deeply nested but perfectly valid
/// file — generated code, a macro-expansion fixture, a checked-in fuzz
/// corpus — lexes fine and then overflows the stack inside the walkers.
/// A stack overflow is not a panic: it aborts the process and cannot be
/// caught, so it would kill the whole `ops` run rather than degrading.
///
/// Hand-written Rust does not come close to this depth; the cap exists
/// only to turn an unrecoverable abort into a warned fallback.
pub const MAX_NESTING_DEPTH: usize = 128;

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
    /// The four buckets partition the lines of a single file region: every
    /// line of the file bumps exactly one of them once (see `Locs::add`),
    /// so their sum equals that region's line count, which is bounded by the
    /// source's byte length (`<= isize::MAX`). The sum therefore cannot
    /// exceed `u64::MAX` and each `saturating_add` returns the exact total.
    #[must_use]
    pub const fn lines(&self) -> u64 {
        self.code
            .saturating_add(self.docs)
            .saturating_add(self.comments)
            .saturating_add(self.blanks)
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.lines() == 0
    }

    /// Called exactly once per classified line of one source file, so every
    /// bucket is bounded by that file's line count, itself bounded by the
    /// source's byte length (`<= isize::MAX`). No bucket can reach
    /// `u64::MAX`, so each `saturating_add` returns the exact count.
    const fn add(&mut self, kind: LineKind) {
        match kind {
            LineKind::Code => self.code = self.code.saturating_add(1),
            LineKind::Doc => self.docs = self.docs.saturating_add(1),
            LineKind::Comment => self.comments = self.comments.saturating_add(1),
            LineKind::Blank => self.blanks = self.blanks.saturating_add(1),
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
    const fn bucket_mut(&mut self, region: Region) -> &mut Locs {
        match region {
            Region::Main => &mut self.main,
            Region::Test => &mut self.test,
            Region::Example => &mut self.example,
        }
    }

    /// Record one line of degraded, blank-vs-non-blank counting.
    ///
    /// The building block behind [`count_fallback`], exposed so a caller
    /// that must not hold a whole file in memory (an over-cap file, read
    /// a line at a time) can produce the same shape of counts.
    ///
    /// Called once per source line, so each bucket is bounded by that
    /// file's line count and `saturating_add` returns the exact total.
    pub const fn add_fallback_line(&mut self, region: Region, blank: bool) {
        let bucket = self.bucket_mut(region);
        if blank {
            bucket.blanks = bucket.blanks.saturating_add(1);
        } else {
            bucket.code = bucket.code.saturating_add(1);
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
///
/// # Memory
///
/// `span-locations` makes every parse retain an owned copy of the
/// source plus a line table in a thread-local source map that is never
/// truncated on its own. Left alone, a whole-workspace scan would hold
/// several times the tree's byte size resident until the thread exits.
/// Dropping the map on entry bounds the retention at one file's worth.
/// This invalidates every previously issued `Span`, which is safe here
/// because spans never escape a single `count_source` call.
#[must_use]
pub fn count_source(src: &str, base: Region) -> FileCounts {
    // Must precede the parses below, not follow them: the spans this
    // function reads have to stay valid for the rest of the body.
    proc_macro2::extra::invalidate_current_thread_spans();

    let lines: Vec<&str> = src.lines().collect();
    let line_count = lines.len();
    if line_count == 0 {
        return FileCounts::default();
    }

    let Ok(stream) = TokenStream::from_str(src) else {
        return count_fallback(src, base);
    };

    let mut kinds = vec![LineKind::Blank; line_count];
    if mark_tokens(&stream, &lines, &mut kinds, 0) == Walk::DepthExceeded {
        tracing::warn!(
            max_depth = MAX_NESTING_DEPTH,
            "rust-loc: delimiter nesting exceeds the depth cap; counting blank vs non-blank only"
        );
        return count_fallback(src, base);
    }

    // Any line still Blank was not covered by a token, so it lies in a
    // gap: whitespace or a comment, nothing else is possible.
    for (kind, line) in kinds.iter_mut().zip(&lines) {
        if *kind == LineKind::Blank && !line.trim().is_empty() {
            *kind = LineKind::Comment;
        }
    }

    let mut regions = vec![base; line_count];
    if base == Region::Main {
        if let Ok(file) = parse_file(src, stream) {
            mark_test_items(&file.items, &mut regions);
        }
    }

    let mut counts = FileCounts::default();
    for (kind, region) in kinds.into_iter().zip(regions) {
        counts.bucket_mut(region).add(kind);
    }
    counts
}

/// Parse `src` into a `syn::File`, reusing the stream already lexed.
///
/// `syn::parse_file` re-lexes the source from scratch, so calling it
/// doubles the lexing cost of every file in the workspace.
/// `syn::parse2` takes the `TokenStream` we already hold instead.
///
/// The one behavioural difference is the shebang: `parse_file` strips a
/// leading `#!...` line, `parse2` does not, and `#!` is otherwise the
/// start of an inner attribute. That case is rare enough not to be
/// worth restructuring the lexing pass around, so it keeps the
/// re-lexing path.
fn parse_file(src: &str, stream: TokenStream) -> syn::Result<syn::File> {
    if has_shebang(src) {
        syn::parse_file(src)
    } else {
        syn::parse2(stream)
    }
}

/// Does `src` open with a `#!...` shebang line rather than an inner
/// attribute (`#![...]`)?
fn has_shebang(src: &str) -> bool {
    src.strip_prefix("#!")
        .is_some_and(|rest| !rest.trim_start().starts_with('['))
}

/// Degraded counting for input the lexer or parser rejects.
#[must_use]
pub fn count_fallback(src: &str, base: Region) -> FileCounts {
    let mut counts = FileCounts::default();
    for line in src.lines() {
        counts.add_fallback_line(base, line.trim().is_empty());
    }
    counts
}

// -- token walking --

/// Whether a recursive token walk ran to completion.
///
/// Distinguished from a plain `bool` so callers cannot mistake
/// "finished" for "found something".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Walk {
    Complete,
    DepthExceeded,
}

/// Mark every line touched by a token as [`LineKind::Code`], except doc
/// comments, which the lexer rewrites into `#[doc = "..."]` attributes
/// and which are reclassified as [`LineKind::Doc`].
fn mark_tokens(stream: &TokenStream, lines: &[&str], kinds: &mut [LineKind], depth: usize) -> Walk {
    if depth >= MAX_NESTING_DEPTH {
        return Walk::DepthExceeded;
    }
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
                if mark_tokens(&group.stream(), lines, kinds, depth.saturating_add(1))
                    == Walk::DepthExceeded
                {
                    return Walk::DepthExceeded;
                }
                mark_span_line_range(group.span_close(), kinds, LineKind::Code);
            }
            other => mark_span_line_range(other.span(), kinds, LineKind::Code),
        }
    }
    Walk::Complete
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
    // The `min` above caps `end` at `kinds.len() - 1 <= isize::MAX - 1`
    // (or 0 when `kinds` is empty, where `take(1)` on an empty iterator is
    // still a no-op), so `saturating_add(1)` equals `+ 1` exactly.
    for slot in kinds.iter_mut().take(end.saturating_add(1)).skip(start) {
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

/// Is this attribute a test gate — does it keep the item out of a
/// non-test build?
///
/// Accepts `#[test]` and any framework variant whose final path segment
/// is `test` (`#[tokio::test]`), plus `#[cfg(..)]` whose predicate
/// mentions the bare `test` ident.
///
/// Scanning for a bare ident is deliberately not the same as a
/// substring match: `cfg(feature = "test")` carries `test` as a string
/// *literal*, not an ident, so it correctly does not match. A `not(..)`
/// group is skipped for the same reason — `#[cfg(not(test))]` gates code
/// *out* of test builds, so it is production code.
///
/// `#[cfg_attr(test, ..)]` is deliberately **not** a gate. It applies an
/// attribute conditionally; the item itself compiles in every
/// configuration. A production type carrying
/// `#[cfg_attr(test, derive(Debug))]` is production code, and counting
/// it as test would misattribute the whole item.
fn attr_is_test_gate(attr: &syn::Attribute) -> bool {
    let path = attr.path();
    if path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "test")
    {
        return true;
    }
    if path.is_ident("cfg") {
        return stream_has_test_ident(&attr.to_token_stream(), 0);
    }
    false
}

/// Does `stream` mention the bare `test` ident outside a `not(..)`?
///
/// Recursion is bounded by [`MAX_NESTING_DEPTH`] for the reasons given
/// on that constant. Past the cap the answer is `false`: a `cfg`
/// predicate nested that deep is not a real test gate, and treating it
/// as production code keeps the walk from aborting the process.
fn stream_has_test_ident(stream: &TokenStream, depth: usize) -> bool {
    if depth >= MAX_NESTING_DEPTH {
        return false;
    }
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
            TokenTree::Group(group)
                if stream_has_test_ident(&group.stream(), depth.saturating_add(1)) =>
            {
                return true
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
    fold_span_range(&item.to_token_stream(), &mut range, 0);
    range
}

/// Fold every token's span into `range`.
///
/// Recursion is bounded by [`MAX_NESTING_DEPTH`]. Past the cap the
/// remaining nested spans are skipped; the enclosing group's open and
/// close delimiters are still folded in, so the range stays a superset
/// of the item's first and last lines.
fn fold_span_range(stream: &TokenStream, range: &mut Option<(usize, usize)>, depth: usize) {
    if depth >= MAX_NESTING_DEPTH {
        return;
    }
    for tt in stream.clone() {
        match tt {
            TokenTree::Group(group) => {
                extend(range, group.span_open());
                fold_span_range(&group.stream(), range, depth.saturating_add(1));
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

const fn item_attrs(item: &syn::Item) -> Option<&Vec<syn::Attribute>> {
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
