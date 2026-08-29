//! Parse-only YAML validation via the `saphyr` event parser. Accepts
//! multi-document streams (one document per `---`); we only care that all
//! documents parse.
//!
//! The parser is driven at the **event** level rather than through
//! `Yaml::load_from_str`, and that is a security property, not a style
//! choice. `saphyr`'s loader materialises every alias by *cloning* the
//! anchored node, so nested anchors expand multiplicatively — the classic
//! "billion laughs" bomb, where 324 bytes of YAML exhaust the address space
//! and abort the process with `SIGABRT` before any error can be reported.
//! An input byte cap cannot express that bound, because the input is tiny.
//!
//! Walking events instead means nothing is ever materialised: memory is
//! O(depth + anchors) regardless of what the document would expand to. On
//! top of that, [`ExpansionBudget`] computes the expanded node count the
//! loader *would* have produced and rejects the document past
//! [`MAX_EXPANDED_NODES`], so a bomb is reported as a normal
//! [`CheckError::Parse`] failure against the file that contains it.
//!
//! Accept/reject behaviour is unchanged: `Yaml::load_from_str` surfaces only
//! the parser's own `ScanError`s, so the same inputs fail with the same
//! messages.

use std::collections::HashMap;

use saphyr_parser::{Event, Parser};

use crate::error::{CheckError, LimitExceeded};

/// Maximum collection nesting. Matches [`crate::json::MAX_NESTING_DEPTH`] so
/// the two checkers agree on what "too deep" means.
pub const MAX_NESTING_DEPTH: u64 = 128;

/// Maximum number of nodes the **stream** would hold once every alias is
/// expanded.
///
/// Sized above what the per-file byte cap can produce without aliases (a
/// 16 MiB document needs at least two bytes per node) so only genuine alias
/// amplification trips it.
///
/// The budget is *stream-wide*, not per document: [`ExpansionBudget`] is
/// created once per `check_yaml` call and nothing about it is reset at
/// `DocumentEnd`, so the documents of a multi-document stream share one
/// allowance. That is deliberate. The resource this cap protects — the
/// memory a loader would need — is a property of the whole file, and a
/// per-document reset would let an attacker multiply the ceiling by the
/// number of `---` separators they care to type, which costs four bytes
/// each. Since the cap sits far above what any honest 16 MiB file reaches,
/// sharing it across documents costs real inputs nothing.
pub const MAX_EXPANDED_NODES: u64 = 20_000_000;

/// `saphyr` numbers anchors from 1; 0 means "this node has no anchor".
const NO_ANCHOR: usize = 0;

/// Validate that `bytes` parses as YAML (one or more documents).
///
/// # Errors
/// Returns [`CheckError::InvalidUtf8`] when the bytes are not UTF-8, or
/// [`CheckError::Parse`] when the YAML parser rejects the input or the
/// document exceeds [`MAX_NESTING_DEPTH`] / [`MAX_EXPANDED_NODES`].
pub fn check_yaml(bytes: &[u8]) -> Result<(), CheckError> {
    let text = std::str::from_utf8(bytes).map_err(CheckError::InvalidUtf8)?;
    let mut parser = Parser::new_from_str(text);
    let mut budget = ExpansionBudget::default();
    while let Some(event) = parser.next_event() {
        let (event, _span) = event.map_err(CheckError::parse)?;
        if matches!(event, Event::StreamEnd) {
            break;
        }
        budget.observe(&event)?;
    }
    Ok(())
}

/// Tracks what the event stream would cost if a loader expanded it.
///
/// Every node counts as one, and an alias counts as whatever its anchor
/// expands to — which is exactly the product a nested-anchor bomb grows.
///
/// One budget covers the whole stream. `DocumentEnd` clears nothing:
/// `level_cost` keeps accumulating and `anchors` keeps its entries across
/// `---` boundaries, so the documents of a multi-document file share the
/// single [`MAX_EXPANDED_NODES`] allowance rather than each getting a fresh
/// one. See that constant for why.
#[derive(Default)]
struct ExpansionBudget {
    /// Expanded node count accumulated at the current nesting level, and —
    /// at the top level — across every document in the stream.
    level_cost: u64,
    /// For each open collection: its anchor id, and the enclosing level's
    /// cost, restored when the collection closes.
    open: Vec<(usize, u64)>,
    /// Expanded node count of each anchor seen so far.
    anchors: HashMap<usize, u64>,
}

impl ExpansionBudget {
    fn observe(&mut self, event: &Event<'_>) -> Result<(), CheckError> {
        match *event {
            Event::Scalar(_, _, anchor, _) => self.add(anchor, 1)?,
            Event::SequenceStart(anchor, _) | Event::MappingStart(anchor, _) => {
                self.open.push((anchor, self.level_cost));
                if u64::try_from(self.open.len()).unwrap_or(u64::MAX) > MAX_NESTING_DEPTH {
                    return Err(limit_exceeded("nesting depth", MAX_NESTING_DEPTH));
                }
                // The collection node itself; children accumulate on top.
                self.level_cost = 1;
            }
            Event::SequenceEnd | Event::MappingEnd => {
                if let Some((anchor, enclosing)) = self.open.pop() {
                    let cost = self.level_cost;
                    self.level_cost = enclosing;
                    self.add(anchor, cost)?;
                }
            }
            // An alias to an anchor that is still open (a recursive
            // reference) has no recorded cost yet; charge it as one node —
            // the loader rejects it rather than expanding it.
            Event::Alias(id) => {
                let cost = self.anchors.get(&id).copied().unwrap_or(1);
                self.add(NO_ANCHOR, cost)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn add(&mut self, anchor: usize, cost: u64) -> Result<(), CheckError> {
        if anchor != NO_ANCHOR {
            self.anchors.insert(anchor, cost);
        }
        self.level_cost = self.level_cost.saturating_add(cost);
        if self.level_cost > MAX_EXPANDED_NODES {
            return Err(limit_exceeded("expanded node count", MAX_EXPANDED_NODES));
        }
        Ok(())
    }
}

fn limit_exceeded(what: &'static str, limit: u64) -> CheckError {
    CheckError::parse(LimitExceeded { what, limit })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_yaml_passes() {
        assert!(check_yaml(b"a: 1\nb:\n  - 2\n  - 3\n").is_ok());
    }

    #[test]
    fn multi_doc_yaml_passes() {
        assert!(check_yaml(b"a: 1\n---\nb: 2\n").is_ok());
    }

    #[test]
    fn invalid_yaml_fails() {
        let err = check_yaml(b"a: : :\n").unwrap_err();
        assert!(
            matches!(err, CheckError::Parse(_)),
            "expected Parse, got {err:?}"
        );
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn modest_alias_use_is_still_accepted() {
        let doc = b"base: &b { a: 1, b: 2 }\nuse1: *b\nuse2: *b\n";
        assert!(check_yaml(doc).is_ok());
    }

    #[test]
    fn nested_anchor_bomb_is_rejected_instead_of_aborting_the_process() {
        use std::fmt::Write as _;

        // "Billion laughs": nine levels, each aliasing the previous nine
        // times. A few hundred bytes that `Yaml::load_from_str` expands until
        // the allocator aborts.
        let mut doc = String::from("a0: &a0 \"lol\"\n");
        for level in 1..=9 {
            let aliases = (0..9)
                .map(|_| format!("*a{}", level - 1))
                .collect::<Vec<_>>()
                .join(",");
            writeln!(doc, "a{level}: &a{level} [{aliases}]").unwrap();
        }
        assert!(
            doc.len() < 500,
            "bomb should stay tiny: {} bytes",
            doc.len()
        );

        let err = check_yaml(doc.as_bytes()).unwrap_err();
        assert_eq!(
            err.to_string(),
            "input exceeds the expanded node count limit of 20000000"
        );
    }

    #[test]
    fn deeply_nested_yaml_is_rejected() {
        let depth = usize::try_from(MAX_NESTING_DEPTH).unwrap() + 10;
        let doc = format!("{}{}", "[".repeat(depth), "]".repeat(depth));
        let err = check_yaml(doc.as_bytes()).unwrap_err();
        assert_eq!(
            err.to_string(),
            "input exceeds the nesting depth limit of 128"
        );
    }

    #[test]
    fn non_utf8_input_reports_invalid_utf8() {
        let err = check_yaml(b"a: \xff\n").unwrap_err();
        assert!(
            matches!(err, CheckError::InvalidUtf8(_)),
            "expected InvalidUtf8, got {err:?}"
        );
        assert!(err.to_string().starts_with("invalid UTF-8: "));
    }

    #[test]
    fn invalid_utf8_exposes_the_utf8_error_as_its_source() {
        use std::error::Error as _;

        let err = check_yaml(b"a: \xff\n").unwrap_err();
        let source = err.source().expect("InvalidUtf8 must keep its cause");
        assert!(source.downcast_ref::<std::str::Utf8Error>().is_some());
    }
}
