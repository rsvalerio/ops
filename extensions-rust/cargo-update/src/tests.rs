//! Tests for the `cargo_update` extension.

use super::*;

// DUP-3 / TASK-1794: the tracing-capture harness (`BufWriter` + `MakeWriter` +
// the global-dispatcher pin) and the control-character assertion used to be
// re-implemented here — a fourth copy of a helper centralised precisely
// because copies drift, and one that had already drifted into panicking on a
// poisoned lock and on a flush that splits a multi-byte char.
use ops_about::test_support::{assert_rendered_escapes_control_chars, capture_warn};

// -- Extension trait tests --

mod extension_tests {
    use super::*;

    ops_extension::test_datasource_extension!(
        CargoUpdateExtension,
        name: "cargo-update",
        data_provider: "cargo_update"
    );
}

/// ERR-7 (TASK-0975) / TEST-25 (TASK-1783): tracing breadcrumbs for
/// cargo-update lines flow through the `?` formatter, so an attacker-shaped
/// crate name with an embedded ANSI escape cannot forge a log record or
/// repaint the operator's terminal.
///
/// Driven through a real `parse_update_output` call and asserted on the
/// *captured record*: switching any `?field` to `%field` in `lib.rs` puts a
/// raw ESC into the capture and fails this test. The previous version built
/// `format!("{line:?}")` locally and was therefore a test of
/// `std::fmt::Debug for &str`.
#[test]
fn warn_breadcrumb_debug_escapes_control_characters() {
    // A bare ESC followed by a space survives `strip_ansi` (it introduces no
    // recognised sequence), so it reaches the crate-name position and the
    // line is rejected — with the offending line logged.
    let stderr = "    Updating evil\u{1b} v1.0.0 -> v2.0.0\n".as_bytes();
    let logged = capture_warn(|| {
        let result = parse_update_output(stderr);
        assert!(
            result.entries.is_empty(),
            "a crate name carrying a control character must not produce an entry"
        );
    });
    let record = logged.trim_end();
    assert!(
        !record.is_empty(),
        "the rejected line must be logged at WARN"
    );
    assert_rendered_escapes_control_chars(record);
    assert!(
        record.contains("\\u{1b}"),
        "the ESC must appear in its Debug-escaped form; got {record:?}"
    );
}

// -- Parser tests --

#[test]
fn parse_single_update() {
    let stderr = b"    Updating serde v1.0.0 -> v1.0.1\n";
    let result = parse_update_output(stderr);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.update_count, 1);
    assert_eq!(result.add_count, 0);
    assert_eq!(result.remove_count, 0);

    let entry = &result.entries[0];
    assert_eq!(entry.action, UpdateAction::Update);
    assert_eq!(entry.name, "serde");
    assert_eq!(entry.from, Some("1.0.0".to_string()));
    assert_eq!(entry.to, Some("1.0.1".to_string()));
}

#[test]
fn parse_single_add() {
    let stderr = b"      Adding new-crate v0.1.0\n";
    let result = parse_update_output(stderr);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.add_count, 1);

    let entry = &result.entries[0];
    assert_eq!(entry.action, UpdateAction::Add);
    assert_eq!(entry.name, "new-crate");
    assert_eq!(entry.from, None);
    assert_eq!(entry.to, Some("0.1.0".to_string()));
}

#[test]
fn parse_single_remove() {
    let stderr = b"    Removing old-crate v0.2.0\n";
    let result = parse_update_output(stderr);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.remove_count, 1);

    let entry = &result.entries[0];
    assert_eq!(entry.action, UpdateAction::Remove);
    assert_eq!(entry.name, "old-crate");
    assert_eq!(entry.from, Some("0.2.0".to_string()));
    assert_eq!(entry.to, None);
}

/// PATTERN-1 / TASK-1778: `Downgrading` is one of the verbs cargo's
/// `print_lockfile_updates` printer emits (a tightened requirement, a lifted
/// `[patch]`, a yanked release). It used to be dropped with no entry, no count
/// and no log record — silent data loss on the crate's single purpose.
#[test]
fn parse_single_downgrade() {
    let stderr = b" Downgrading serde v1.0.220 -> v1.0.219\n";
    let result = parse_update_output(stderr);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.downgrade_count, 1);
    assert_eq!(result.update_count, 0);

    let entry = &result.entries[0];
    assert_eq!(entry.action, UpdateAction::Downgrade);
    assert_eq!(entry.name, "serde");
    assert_eq!(entry.from.as_deref(), Some("1.0.220"));
    assert_eq!(entry.to.as_deref(), Some("1.0.219"));
}

/// PATTERN-1 / TASK-1778: a `Downgrading` line must never reach the
/// format-drift warn — it is a recognised verb now, not drift.
#[test]
fn downgrade_line_does_not_warn() {
    let logged = capture_warn(|| {
        let result = parse_update_output(b" Downgrading serde v1.0.220 -> v1.0.219\n");
        assert_eq!(result.downgrade_count, 1);
    });
    assert!(
        !logged.contains("possible format drift"),
        "a recognised Downgrading line must not warn; got {logged:?}"
    );
}

/// PATTERN-1 / TASK-1778: the verbose-only `Unchanged` verb is skipped as
/// noise *deliberately* — no entry, and no drift warn either.
#[test]
fn parse_skips_unchanged_line_without_warning() {
    let logged = capture_warn(|| {
        let stderr = b"    Unchanged serde v1.0.0 (latest: v1.1.0)\n";
        let result = parse_update_output(stderr);
        assert!(
            result.entries.is_empty(),
            "Unchanged lines carry no change and must produce no entry"
        );
    });
    assert!(
        !logged.contains("possible format drift"),
        "Unchanged is filtered on purpose and must not warn; got {logged:?}"
    );
}

/// PATTERN-1 / TASK-1778: an unhandled *shape* of a known verb still reaches
/// the drift warn — including for `Downgrading`, the verb the table was
/// missing entirely.
#[test]
fn downgrade_line_with_drifted_shape_warns() {
    let logged = capture_warn(|| {
        let result = parse_update_output(b" Downgrading serde v1.0.220 to v1.0.219\n");
        assert!(result.entries.is_empty());
    });
    assert!(
        logged.contains("possible format drift"),
        "a drifted Downgrading shape must warn; got {logged:?}"
    );
}

#[test]
fn parse_mixed_output() {
    let stderr = b"\
    Updating crates.io index
    Locking 3 packages to latest compatible versions
    Updating serde v1.0.0 -> v1.0.1
      Adding new-dep v0.5.0
    Removing old-dep v0.3.0
 Downgrading down-dep v2.0.0 -> v1.9.0
    Updating tokio v1.28.0 -> v1.29.0
";
    let result = parse_update_output(stderr);
    assert_eq!(result.entries.len(), 5);
    assert_eq!(result.update_count, 2);
    assert_eq!(result.add_count, 1);
    assert_eq!(result.remove_count, 1);
    assert_eq!(result.downgrade_count, 1);

    assert_eq!(result.entries[0].name, "serde");
    assert_eq!(result.entries[1].name, "new-dep");
    assert_eq!(result.entries[2].name, "old-dep");
    assert_eq!(result.entries[3].name, "down-dep");
    assert_eq!(result.entries[4].name, "tokio");
}

#[test]
fn parse_empty_output() {
    let stderr = b"";
    let result = parse_update_output(stderr);
    assert!(result.entries.is_empty());
    assert_eq!(result.update_count, 0);
    assert_eq!(result.add_count, 0);
    assert_eq!(result.remove_count, 0);
    assert_eq!(result.downgrade_count, 0);
}

#[test]
fn parse_no_updates_available() {
    let stderr = b"\
    Updating crates.io index
    Locking 0 packages to latest compatible versions
";
    let result = parse_update_output(stderr);
    assert!(result.entries.is_empty());
}

#[test]
fn parse_strips_v_prefix() {
    let stderr = b"    Updating serde v1.0.0 -> v1.0.1\n";
    let result = parse_update_output(stderr);
    assert_eq!(result.entries[0].from, Some("1.0.0".to_string()));
    assert_eq!(result.entries[0].to, Some("1.0.1".to_string()));
}

#[test]
fn parse_no_v_prefix_passthrough() {
    let stderr = b"    Updating serde 1.0.0 -> 1.0.1\n";
    let result = parse_update_output(stderr);
    assert_eq!(result.entries[0].from, Some("1.0.0".to_string()));
    assert_eq!(result.entries[0].to, Some("1.0.1".to_string()));
}

#[test]
fn parse_skips_warning_lines() {
    let stderr = b"\
warning: some warning message
    Updating serde v1.0.0 -> v1.0.1
note: some note
";
    let result = parse_update_output(stderr);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].name, "serde");
}

#[test]
fn parse_skips_index_update_line() {
    let stderr = b"    Updating crates.io index\n";
    let result = parse_update_output(stderr);
    assert!(result.entries.is_empty());
}

/// TEST-1 / TASK-1077: pin BOTH invariants of the arrow-drift / extra-trailing
/// path in `parse_action_line` — the warn fires AND `entries` stays empty (or,
/// for the Adding/Removing extra-tokens case, the entry is still produced
/// alongside the warn). A refactor that swallows the warn silently (e.g. by
/// short-circuiting the verb match) would otherwise be undetected by the
/// existing tests.
#[test]
fn arrow_drift_and_extra_tokens_warn_fires_with_expected_entries() {
    // -- Updating arrow-drift: warn fires AND entries.is_empty() --
    let logged = capture_warn(|| {
        // Drift shape: `to` instead of `->`.
        let stderr = b"    Updating serde v1.0.0 to v1.0.1\n";
        let result = parse_update_output(stderr);
        assert!(
            result.entries.is_empty(),
            "arrow-drift Updating line must produce no entry"
        );
        assert_eq!(result.update_count, 0);
    });
    assert!(
        logged.contains("WARN") && logged.contains("possible format drift"),
        "arrow-drift must emit the format-drift warn at default level; got {logged:?}"
    );

    // -- Adding with extra trailing tokens: warn fires AND entry is still produced --
    let logged = capture_warn(|| {
        // `Adding new-crate v0.1.0 (locked)` — the (locked) annotation must
        // not be glued onto the version. parse_action_line warns and keeps
        // the entry; observability contract is the warn line.
        let stderr = b"      Adding new-crate v0.1.0 (locked)\n";
        let result = parse_update_output(stderr);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].to.as_deref(), Some("0.1.0"));
    });
    assert!(
        logged.contains("WARN") && logged.contains("unexpected trailing tokens"),
        "Adding extra-tokens must emit the trailing-tokens warn at default level; got {logged:?}"
    );

    // -- Removing with extra trailing tokens: warn fires AND entry is still produced --
    let logged = capture_warn(|| {
        let stderr = b"    Removing old-crate v0.2.0 (yanked)\n";
        let result = parse_update_output(stderr);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].from.as_deref(), Some("0.2.0"));
    });
    assert!(
        logged.contains("WARN") && logged.contains("unexpected trailing tokens"),
        "Removing extra-tokens must emit the trailing-tokens warn at default level; got {logged:?}"
    );
}

/// TASK-0472: a verb-prefixed line that does not match the expected shape
/// must not silently disappear from the count headline. The dropped line
/// is still not produced as an `UpdateEntry`, but operators must observe
/// the drop via tracing — verified here by ensuring the entry list stays
/// empty (so the warn branch is exercised). The warn-level promotion is
/// what makes this observable at the default log filter.
#[test]
fn parse_drops_verb_prefixed_line_with_unexpected_shape() {
    // Hypothetical future cargo format: "Updating serde from v1 to v2"
    let stderr = b"    Updating serde from v1 to v2\n";
    let result = parse_update_output(stderr);
    assert!(
        result.entries.is_empty(),
        "unexpected-shape verb line should not produce an entry"
    );
    assert_eq!(result.update_count, 0);
}

/// PATTERN-1 / TASK-1030: a verb-prefix without a whitespace boundary must
/// not classify as a known verb (no false-positive drift warning) and must
/// not be consumed by `parse_action_line`'s `strip_prefix`. The legitimate
/// `Updating serde v1 -> v2` form must still parse.
#[test]
fn verb_prefix_requires_whitespace_boundary() {
    // `Updatingxyz` is not a known verb: produces no entry AND no warn.
    assert!(
        !starts_with_known_verb("Updatingxyz serde v1 -> v2"),
        "verb prefix without word boundary must not classify as known verb"
    );
    let stderr = b"    Updatingxyz serde v1 -> v2\n";
    let result = parse_update_output(stderr);
    assert!(
        result.entries.is_empty(),
        "Updatingxyz must not produce a parsed entry"
    );

    // Legitimate `Updating ` still works.
    assert!(starts_with_known_verb("Updating serde v1 -> v2"));
    let stderr_ok = b"    Updating serde v1.0.0 -> v1.0.1\n";
    let result_ok = parse_update_output(stderr_ok);
    assert_eq!(result_ok.entries.len(), 1);
    assert_eq!(result_ok.entries[0].name, "serde");
    assert_eq!(result_ok.entries[0].from.as_deref(), Some("1.0.0"));
    assert_eq!(result_ok.entries[0].to.as_deref(), Some("1.0.1"));
}

/// DUP-1 / TASK-1797: `starts_with_known_verb` and `parse_action_line` consume
/// one shared verb + whitespace-boundary match, so they cannot drift apart —
/// the failure TASK-1030 had to patch into both sites separately.
#[test]
fn match_verb_is_the_single_boundary_definition() {
    for verb in ["Updating", "Downgrading", "Adding", "Removing"] {
        assert!(
            match_verb(&format!("{verb} crate-a v1.0.0")).is_some(),
            "{verb} with a whitespace boundary must match"
        );
        assert!(
            match_verb(&format!("{verb}xyz crate-a v1.0.0")).is_none(),
            "{verb} without a whitespace boundary must not match"
        );
        // Bare verb, end-of-string boundary.
        assert!(match_verb(verb).is_some(), "{verb} alone must match");
    }
    assert!(match_verb("Compiling crate-a v1.0.0").is_none());
}

#[test]
fn parse_skips_locking_line() {
    let stderr = b"      Locking 5 packages to latest compatible versions\n";
    let result = parse_update_output(stderr);
    assert!(result.entries.is_empty());
}

#[test]
fn serialization_round_trip() {
    let result = CargoUpdateResult {
        entries: vec![
            UpdateEntry {
                action: UpdateAction::Update,
                name: "serde".to_string(),
                from: Some("1.0.0".to_string()),
                to: Some("1.0.1".to_string()),
            },
            UpdateEntry {
                action: UpdateAction::Add,
                name: "new-crate".to_string(),
                from: None,
                to: Some("0.1.0".to_string()),
            },
        ],
        update_count: 1,
        downgrade_count: 0,
        add_count: 1,
        remove_count: 0,
    };

    let json = serde_json::to_value(&result).expect("serialize");
    assert_eq!(json["update_count"], 1);
    assert_eq!(json["downgrade_count"], 0);
    assert_eq!(json["add_count"], 1);
    assert_eq!(json["remove_count"], 0);
    assert_eq!(json["entries"].as_array().unwrap().len(), 2);
    assert_eq!(json["entries"][0]["action"], "update");
    assert_eq!(json["entries"][1]["action"], "add");
}

#[test]
fn strip_v_prefix_with_v() {
    assert_eq!(strip_v_prefix("v1.0.0"), "1.0.0");
}

#[test]
fn strip_v_prefix_without_v() {
    assert_eq!(strip_v_prefix("1.0.0"), "1.0.0");
}

/// PERF-3 / TASK-0970: the no-escape fast path must avoid the heap
/// allocation entirely. Verified by asserting the Cow is Borrowed —
/// every cargo-update stderr line in CI (no terminal colors) flows
/// through this branch.
#[test]
fn strip_ansi_borrows_when_no_escape() {
    use std::borrow::Cow;
    let input = "    Updating serde v1.0.0 -> v1.0.1";
    let out = strip_ansi(input);
    assert!(
        matches!(out, Cow::Borrowed(_)),
        "expected borrow on no-escape input"
    );
    assert_eq!(out, input);
}

#[test]
fn strip_ansi_owns_when_escape_present() {
    use std::borrow::Cow;
    let input = "\x1b[32mhi\x1b[0m";
    let out = strip_ansi(input);
    assert!(
        matches!(out, Cow::Owned(_)),
        "expected owned rewrite when ANSI present"
    );
    assert_eq!(out, "hi");
}

#[test]
fn strip_ansi_removes_escape_codes() {
    let input = "\x1b[1m\x1b[32mUpdating\x1b[0m serde v1.0.0 -> v1.0.1";
    let clean = strip_ansi(input);
    assert_eq!(clean, "Updating serde v1.0.0 -> v1.0.1");
}

/// SEC-21 / TASK-1790: `strip_ansi` claimed to strip "ANSI escape sequences"
/// but only understood CSI. Cargo emits OSC-8 hyperlinks whenever
/// `term.hyperlinks` is auto-detected, so `ESC ] … BEL` reaches the parser in
/// ordinary interactive use.
#[test]
fn strip_ansi_removes_osc8_hyperlink() {
    let bel = "\x1b]8;;https://crates.io/crates/serde\x07serde\x1b]8;;\x07";
    assert_eq!(strip_ansi(bel), "serde");
    // The ST-terminated form (`ESC \`) is equally valid.
    let st = "\x1b]8;;https://crates.io/crates/serde\x1b\\serde\x1b]8;;\x1b\\";
    assert_eq!(strip_ansi(st), "serde");
}

/// SEC-21 / TASK-1790: two-character escapes (`ESC c` RIS — a full terminal
/// reset) and charset selects (`ESC ( B`) previously fell through with the raw
/// `ESC` intact.
#[test]
fn strip_ansi_removes_two_character_escapes() {
    assert_eq!(strip_ansi("a\x1bcb"), "ab");
    assert_eq!(strip_ansi("a\x1b(Bb"), "ab");
}

#[test]
fn parse_output_with_ansi_codes() {
    let stderr = b"\x1b[1m\x1b[32m    Updating\x1b[0m serde v1.0.0 -> v1.0.1\n";
    let result = parse_update_output(stderr);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].name, "serde");
}

/// SEC-21 / TASK-1790: an OSC-8 hyperlink wrapping a real update line still
/// parses, and nothing escape-shaped reaches the serialized provider JSON.
#[test]
fn osc8_wrapped_update_line_parses_with_no_escape_in_json() {
    let stderr =
        "    \x1b]8;;https://crates.io/crates/serde\x07Updating serde v1.0.0 -> v1.0.1\x1b]8;;\x07\n"
            .as_bytes();
    let result = parse_update_output(stderr);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].name, "serde");
    let json = serde_json::to_string(&result).expect("serialize");
    assert!(
        !json.contains('\u{1b}'),
        "no ESC may reach the provider JSON: {json}"
    );
}

/// SEC-21 / TASK-1790: an `ESC` that survives `strip_ansi` — a bare one that
/// introduces no recognised sequence, or the truncated-CSI bytes TASK-1028
/// deliberately preserves — must never be published as part of a crate name
/// or version. The line is rejected and logged instead.
#[test]
fn control_characters_never_reach_the_serialized_json() {
    for stderr in [
        // Bare ESC (followed by a space, so it introduces nothing).
        "    Updating evil\u{1b} v1.0.0 -> v2.0.0\n",
        // Truncated CSI at end of the version token.
        "      Adding new-crate v0.1.0\u{1b}[3\n",
        // NUL inside the crate name.
        "    Removing old\u{0}crate v0.2.0\n",
    ] {
        let logged = capture_warn(|| {
            let result = parse_update_output(stderr.as_bytes());
            assert!(
                result.entries.is_empty(),
                "control-carrying line must produce no entry: {stderr:?}"
            );
            let json = serde_json::to_string(&result).expect("serialize");
            assert!(
                !json.contains('\u{1b}') && !json.contains('\u{0}'),
                "no control byte may reach the provider JSON: {json}"
            );
        });
        assert!(
            logged.contains("WARN"),
            "a rejected line must be logged, not dropped silently: {stderr:?}"
        );
    }
}

#[test]
fn parse_malformed_updating_line_missing_arrow() {
    let stderr = b"    Updating serde v1.0.0\n";
    let result = parse_update_output(stderr);
    assert!(
        result.entries.is_empty(),
        "incomplete update line should be skipped"
    );
}

#[test]
fn parse_malformed_adding_line_missing_version() {
    let stderr = b"      Adding new-crate\n";
    let result = parse_update_output(stderr);
    assert!(
        result.entries.is_empty(),
        "adding line without version should be skipped"
    );
}

#[test]
fn parse_malformed_removing_line_missing_version() {
    let stderr = b"    Removing old-crate\n";
    let result = parse_update_output(stderr);
    assert!(
        result.entries.is_empty(),
        "removing line without version should be skipped"
    );
}

#[test]
fn parse_adding_line_with_trailing_annotation_does_not_glue_into_version() {
    // TASK-0949: a future cargo annotation must not be silently absorbed into
    // version_raw. The line is parsed (warn-and-keep) but the resulting `to`
    // version is just the version token, not "0.1.0 (locked)".
    let stderr = b"      Adding new-crate v0.1.0 (locked)\n";
    let result = parse_update_output(stderr);
    assert_eq!(result.entries.len(), 1);
    let entry = &result.entries[0];
    assert_eq!(entry.name, "new-crate");
    assert_eq!(entry.to.as_deref(), Some("0.1.0"));
    assert!(entry.from.is_none());
}

#[test]
fn parse_removing_line_with_trailing_annotation_does_not_glue_into_version() {
    let stderr = b"    Removing old-crate v0.1.0 (yanked)\n";
    let result = parse_update_output(stderr);
    assert_eq!(result.entries.len(), 1);
    let entry = &result.entries[0];
    assert_eq!(entry.name, "old-crate");
    assert_eq!(entry.from.as_deref(), Some("0.1.0"));
    assert!(entry.to.is_none());
}

/// SEC-11 / TASK-1799: the version position is validated. Before this, any
/// token was accepted and published as a version — `(locked)` when the
/// annotation preceded the version, `latest`, `???`, or the worst case
/// `Some("")` from a bare `v`, which reads as a known version to every
/// consumer that checks `is_some()`.
#[test]
fn non_version_shaped_token_is_never_published_as_a_version() {
    for (stderr, why) in [
        (
            "      Adding foo v\n",
            "a bare `v` must not become an empty version",
        ),
        (
            "      Adding new-crate (locked) v0.1.0\n",
            "an annotation in the version position must not become the version",
        ),
        (
            "    Updating serde v1.0.0 -> latest\n",
            "`latest` is not a version",
        ),
        ("    Removing old-crate ???\n", "`???` is not a version"),
    ] {
        let result = parse_update_output(stderr.as_bytes());
        assert!(result.entries.is_empty(), "{why}: got {:?}", result.entries);
    }
}

/// SEC-11 / TASK-1799: a rejected version must still be observable — the whole
/// point of the crate's loud-on-drift design. `Adding foo v` carries no
/// `v<digit>` token, so the pre-existing `starts_with_known_verb` gate would
/// not have warned about it.
#[test]
fn non_version_shaped_token_reaches_a_warn() {
    for stderr in [
        "      Adding foo v\n",
        "      Adding new-crate (locked) v0.1.0\n",
        "    Updating serde v1.0.0 -> latest\n",
    ] {
        let logged = capture_warn(|| {
            let result = parse_update_output(stderr.as_bytes());
            assert!(result.entries.is_empty());
        });
        assert!(
            logged.contains("WARN"),
            "a rejected version must be logged: {stderr:?}; got {logged:?}"
        );
    }
}

/// ERR-1 / TASK-1252 regression guard for the SEC-11 validation above: real
/// non-action `Updating` lines (git repositories, index progress) must stay
/// silent rather than becoming a per-run warn.
#[test]
fn non_action_updating_lines_stay_silent() {
    let logged = capture_warn(|| {
        let stderr = b"\
    Updating crates.io index
    Updating git repository `https://github.com/owner/repo`
";
        let result = parse_update_output(stderr);
        assert!(result.entries.is_empty());
    });
    assert!(
        logged.trim().is_empty(),
        "non-action Updating lines must not warn; got {logged:?}"
    );
}

#[test]
fn parse_multiple_updates_same_crate() {
    let stderr = b"\
    Updating serde v1.0.0 -> v1.0.1
    Updating serde_derive v1.0.0 -> v1.0.1
    Updating serde_json v1.0.0 -> v1.0.1
";
    let result = parse_update_output(stderr);
    assert_eq!(result.entries.len(), 3);
    assert_eq!(result.update_count, 3);
    assert_eq!(result.add_count, 0);
    assert_eq!(result.remove_count, 0);
}

#[test]
fn parse_skips_note_lines() {
    let stderr = b"\
note: pass `--verbose` to see more
    Updating serde v1.0.0 -> v1.0.1
";
    let result = parse_update_output(stderr);
    assert_eq!(result.entries.len(), 1);
}

#[test]
fn parse_skips_blank_lines() {
    let stderr = b"\n\n    Updating serde v1.0.0 -> v1.0.1\n\n";
    let result = parse_update_output(stderr);
    assert_eq!(result.entries.len(), 1);
}

#[test]
fn strip_ansi_no_escape_codes() {
    let input = "plain text";
    assert_eq!(strip_ansi(input), "plain text");
}

#[test]
fn strip_ansi_multiple_consecutive_codes() {
    let input = "\x1b[1m\x1b[32m\x1b[4mtext\x1b[0m";
    assert_eq!(strip_ansi(input), "text");
}

#[test]
fn strip_ansi_at_boundaries() {
    let input = "\x1b[31mhello\x1b[0m";
    assert_eq!(strip_ansi(input), "hello");
}

#[test]
fn strip_v_prefix_empty_string() {
    assert_eq!(strip_v_prefix(""), "");
}

#[test]
fn strip_v_prefix_just_v() {
    assert_eq!(strip_v_prefix("v"), "");
}

#[test]
fn parse_updating_line_with_various_index_names() {
    // "Updating github.com index" should also be skipped
    let stderr = b"    Updating github.com index\n";
    let result = parse_update_output(stderr);
    assert!(result.entries.is_empty());
}

/// PATTERN-1 / TASK-1054: a crate whose name contains the substring `index`
/// (e.g. `indexer`, `index-map`, `reindex`) must be parsed as a real update.
/// The previous `starts_with("Updating") && contains("index")` predicate was
/// too broad and silently dropped these entries.
#[test]
fn parse_update_for_crate_name_containing_index_is_not_dropped() {
    let stderr = b"\
    Updating crates.io index
    Updating indexer v1.0.0 -> v1.0.1
    Updating index-map v0.4.0 -> v0.5.0
    Updating reindex v2.0.0 -> v2.0.1
";
    let result = parse_update_output(stderr);
    assert_eq!(
        result.entries.len(),
        3,
        "all three crates whose names contain 'index' must be parsed"
    );
    assert_eq!(result.update_count, 3);
    let names: Vec<&str> = result.entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["indexer", "index-map", "reindex"]);
    // Sanity-check the index-noise line is still filtered (no entry with that shape).
    assert!(!result.entries.iter().any(|e| e.name == "crates.io"));
}

/// PATTERN-1 / TASK-1054: alternate-registry index-progress noise lines
/// (cargo emits a parenthesised suffix for non-default registries) must
/// continue to be filtered.
#[test]
fn parse_skips_alternate_registry_index_progress_line() {
    let stderr = b"    Updating crates.io index (sparse+https://index.crates.io/)\n";
    let result = parse_update_output(stderr);
    assert!(
        result.entries.is_empty(),
        "alternate-registry index-progress line must remain filtered"
    );
}

/// ERR-1 / TASK-1252: a 2-token `Updating crates.io` progress form (some
/// cargo releases / locales emit the index-progress line without the third
/// `index` token) must be filtered as noise — and must NOT trigger the
/// `starts_with_known_verb` format-drift warn that PATTERN-1 / TASK-1054
/// installed.
#[test]
fn parse_skips_two_token_updating_registry_form_no_warn() {
    let logged = capture_warn(|| {
        let stderr = b"    Updating crates.io\n";
        let result = parse_update_output(stderr);
        assert!(
            result.entries.is_empty(),
            "2-token Updating <registry> must be filtered as index-progress noise"
        );
    });
    assert!(
        !logged.contains("possible format drift"),
        "2-token Updating <registry> must not trigger a format-drift warn; got {logged:?}"
    );
}

#[test]
fn update_action_serialization() {
    let update = serde_json::to_value(UpdateAction::Update).unwrap();
    assert_eq!(update, "update");
    let downgrade = serde_json::to_value(UpdateAction::Downgrade).unwrap();
    assert_eq!(downgrade, "downgrade");
    let add = serde_json::to_value(UpdateAction::Add).unwrap();
    assert_eq!(add, "add");
    let remove = serde_json::to_value(UpdateAction::Remove).unwrap();
    assert_eq!(remove, "remove");
}

#[test]
fn update_action_deserialization() {
    let update: UpdateAction = serde_json::from_str("\"update\"").unwrap();
    assert_eq!(update, UpdateAction::Update);
    let downgrade: UpdateAction = serde_json::from_str("\"downgrade\"").unwrap();
    assert_eq!(downgrade, UpdateAction::Downgrade);
    let add: UpdateAction = serde_json::from_str("\"add\"").unwrap();
    assert_eq!(add, UpdateAction::Add);
    let remove: UpdateAction = serde_json::from_str("\"remove\"").unwrap();
    assert_eq!(remove, UpdateAction::Remove);
}

/// PATTERN-1 / TASK-1778: `downgrade_count` is `#[serde(default)]`, so a
/// payload produced before the field existed (the about page reads this JSON
/// from a cache) still deserializes.
#[test]
fn cargo_update_result_deserialization() {
    let json = serde_json::json!({
        "entries": [
            {"action": "update", "name": "serde", "from": "1.0.0", "to": "1.0.1"}
        ],
        "update_count": 1,
        "add_count": 0,
        "remove_count": 0
    });
    let result: CargoUpdateResult = serde_json::from_value(json).unwrap();
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].action, UpdateAction::Update);
    assert_eq!(result.update_count, 1);
    assert_eq!(result.downgrade_count, 0);
}

// -- Provider tests (TEST-5 / TASK-1787, TEST-25 / TASK-1783) --

/// Build a `std::process::Output` with the given raw wait status, so the
/// provider's output-interpretation half can be driven without spawning cargo.
#[cfg(unix)]
fn output_with(raw_status: i32, stdout: &[u8], stderr: &[u8]) -> Output {
    use std::os::unix::process::ExitStatusExt;
    Output {
        status: std::process::ExitStatus::from_raw(raw_status),
        stdout: stdout.to_vec(),
        stderr: stderr.to_vec(),
    }
}

/// SEC-21 / TASK-1537, TEST-25 / TASK-1783: the non-zero-exit branch of the
/// provider formats the stderr tail via the Debug formatter (`{:?}`) so
/// embedded ANSI escapes / NULs / newlines from a poisoned crate cannot forge
/// log records or repaint the operator's terminal.
///
/// Driven through the production `interpret_output`, not a locally rebuilt
/// copy of its `format!`: reverting `{:?}` to `{}` fails this test.
#[cfg(unix)]
#[test]
fn non_zero_exit_stderr_tail_debug_escapes_control_bytes() {
    let stderr_bytes = b"warn: ok\nerror: \x1b[31mhi\x1b[0m\x00bye\n";
    let output = output_with(101 << 8, b"", stderr_bytes);

    let err = interpret_output(&output).expect_err("non-zero exit must be an error");
    let rendered = format!("{err:#}");

    assert!(
        rendered.contains("101"),
        "exit status must reach the operator: {rendered}"
    );
    assert!(
        !rendered.contains('\u{1b}'),
        "ANSI ESC must not survive in: {rendered:?}"
    );
    assert!(
        !rendered.contains('\u{0}'),
        "NUL must not survive in: {rendered:?}"
    );
    assert!(
        !rendered.contains('\n'),
        "embedded stderr newlines must be Debug-escaped: {rendered:?}"
    );
    assert!(
        rendered.contains("hi"),
        "expected stderr context preserved: {rendered}"
    );
}

/// TASK-0502 / TEST-5 / TASK-1787: a non-zero exit must never be reported as
/// "no updates available".
#[cfg(unix)]
#[test]
fn non_zero_exit_never_reports_an_empty_result() {
    let output = output_with(1 << 8, b"", b"error: could not lock Cargo.lock\n");
    assert!(
        interpret_output(&output).is_err(),
        "a failed invocation must not deserialize as an empty result"
    );
}

/// TEST-5 / TASK-1787: cargo prints the dry-run lockfile report on **stderr**.
/// A stdout/stderr wiring mistake would previously have gone unnoticed — the
/// stdout content here would parse to a different, wrong answer.
#[cfg(unix)]
#[test]
fn success_branch_parses_stderr_not_stdout() {
    let output = output_with(
        0,
        b"    Updating wrong-stream v9.9.9 -> v9.9.10\n",
        b"    Updating serde v1.0.0 -> v1.0.1\n      Adding new-dep v0.5.0\n",
    );

    let json = interpret_output(&output).expect("success path must produce JSON");
    assert_eq!(json["update_count"], 1);
    assert_eq!(json["add_count"], 1);
    assert_eq!(json["remove_count"], 0);
    assert_eq!(json["downgrade_count"], 0);
    let entries = json["entries"].as_array().expect("entries array");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["name"], "serde");
    assert_eq!(entries[0]["action"], "update");
    assert_eq!(entries[0]["from"], "1.0.0");
    assert_eq!(entries[0]["to"], "1.0.1");
    assert_eq!(entries[1]["name"], "new-dep");
    assert_eq!(entries[1]["action"], "add");
}

/// ERR-4 / TASK-1535, TEST-25 / TASK-1783: when `run_cargo_update_dry_run`
/// returns a `RunError`, the provider wraps it via `.context(...)` rather than
/// flattening it to Display. Asserted on the `DataProviderError` production
/// actually builds, so flattening the wrap back into
/// `anyhow!("{}: {}", ..)` fails this test — the source chain would be gone.
#[test]
fn provide_wraps_run_error_with_context_preserving_source_chain() {
    use ops_core::subprocess::RunError;
    use std::error::Error as _;
    use std::io;

    let underlying = RunError::Io(io::Error::new(io::ErrorKind::NotFound, "no cargo"));
    let mapped = map_run_error(underlying);

    assert!(
        format!("{mapped:#}").contains("cargo update --dry-run failed"),
        "expected context message in display; got {mapped:#}"
    );

    let mut source: Option<&(dyn std::error::Error + 'static)> = mapped.source();
    let mut found_run_error = false;
    while let Some(cause) = source {
        if cause.downcast_ref::<RunError>().is_some() {
            found_run_error = true;
            break;
        }
        source = cause.source();
    }
    assert!(
        found_run_error,
        "the error chain must still contain the original RunError; got {mapped:?}"
    );
}

/// TEST-5 / TASK-1787: pin the subprocess invocation — argv, working
/// directory, timeout and label — without spawning cargo.
#[test]
fn cargo_update_invocation_is_pinned() {
    let dir = Path::new("/tmp/some-workspace");
    let invocation = cargo_update_invocation(dir);
    assert_eq!(invocation.args, ["update", "--dry-run"]);
    assert_eq!(invocation.working_dir, dir);
    assert_eq!(invocation.timeout, Duration::from_secs(120));
    assert_eq!(invocation.label, "cargo update --dry-run");
    assert_eq!(CARGO_UPDATE_TIMEOUT, Duration::from_secs(120));
}

#[test]
fn schema_has_expected_fields() {
    use ops_extension::DataProvider;
    let schema = CargoUpdateProvider.schema();
    assert_eq!(schema.fields.len(), 5);
    let field_names: Vec<&str> = schema.fields.iter().map(|f| f.name).collect();
    assert!(field_names.contains(&"entries"));
    assert!(field_names.contains(&"update_count"));
    assert!(field_names.contains(&"downgrade_count"));
    assert!(field_names.contains(&"add_count"));
    assert!(field_names.contains(&"remove_count"));
}

#[test]
fn parse_only_adds() {
    let stderr = b"\
      Adding dep-a v0.1.0
      Adding dep-b v0.2.0
      Adding dep-c v0.3.0
";
    let result = parse_update_output(stderr);
    assert_eq!(result.entries.len(), 3);
    assert_eq!(result.add_count, 3);
    assert_eq!(result.update_count, 0);
    assert_eq!(result.remove_count, 0);
}

#[test]
fn parse_only_removes() {
    let stderr = b"\
    Removing dep-a v0.1.0
    Removing dep-b v0.2.0
";
    let result = parse_update_output(stderr);
    assert_eq!(result.entries.len(), 2);
    assert_eq!(result.remove_count, 2);
    assert_eq!(result.update_count, 0);
    assert_eq!(result.add_count, 0);
}

#[test]
fn parse_ignores_unknown_lines() {
    let stderr = b"\
    Compiling something
    Finished something
    Updating serde v1.0.0 -> v1.0.1
";
    let result = parse_update_output(stderr);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].name, "serde");
}

/// ERR-1 / TASK-0882: `strip_ansi` must round-trip non-ASCII UTF-8 input
/// identically. The previous `bytes[i] as char` cast corrupted every
/// continuation byte into a Latin-1 code point.
#[test]
fn strip_ansi_round_trips_non_ascii() {
    let input = "café — naïve résumé 日本語";
    assert_eq!(strip_ansi(input), input);
}

/// ERR-1 / TASK-0882: ANSI sequences are still removed even when
/// surrounded by non-ASCII text.
#[test]
fn strip_ansi_removes_csi_around_unicode() {
    let input = "\x1b[31mcafé\x1b[0m";
    assert_eq!(strip_ansi(input), "café");
}

/// ERR-1 / TASK-0882: a non-ASCII char that happens to land where a CSI
/// final byte would be (0x40..=0x7E) does not break the parser — we only
/// match the final-byte range against single ASCII codepoints, and
/// `chars()` decoding ensures we don't see a stray continuation byte
/// in that range.
#[test]
fn strip_ansi_csi_termination_is_byte_safe() {
    // ESC [ 1 ; 31 m  followed by a non-ASCII char.
    let input = "\x1b[1;31m日本語";
    assert_eq!(strip_ansi(input), "日本語");
}

/// PATTERN-1 / TASK-1028: an input ending mid-CSI (no final byte before
/// EOF) must not silently swallow the leading visible text. Pinned
/// behaviour: `foo` is preserved (it precedes the orphan `\x1b[3`), and
/// the truncated CSI bytes are themselves preserved in the output rather
/// than dropping everything to EOF. The previous implementation kept
/// `foo` (since it was already in `result`) but would silently consume
/// arbitrary trailing characters in inputs like `"\x1b[3foo"`.
#[test]
fn strip_ansi_truncated_csi_preserves_leading_text() {
    let input = "foo\x1b[3";
    let out = strip_ansi(input);
    assert!(
        out.contains("foo"),
        "strip_ansi must not silently swallow `foo` on truncated CSI; got {out:?}"
    );
    // Pin the chosen behaviour: preserve consumed-but-unterminated bytes.
    assert_eq!(out, "foo\x1b[3");
}

/// PATTERN-1 / TASK-1028: trailing visible text after an orphan `\x1b[`
/// (the case the bug report flags as "drains chars to EOF") must not be
/// silently swallowed. The cap of 64 bytes bounds the scan so anything
/// past it is emitted normally.
#[test]
fn strip_ansi_truncated_csi_does_not_swallow_trailing_text() {
    // `\x1b[` with parameter bytes only (no final 0x40..=0x7E), then EOF.
    let input = "\x1b[123";
    let out = strip_ansi(input);
    // `123` are all in the 0x30..=0x39 range — valid CSI parameter bytes,
    // so without the cap they would be consumed silently to EOF.
    assert!(
        out.contains('1') && out.contains('2') && out.contains('3'),
        "strip_ansi must not silently drop CSI-parameter-shaped trailing bytes on EOF; got {out:?}"
    );
}

/// SEC-21 / TASK-1790: a truncated OSC must be bounded the same way — the
/// visible text after it survives.
#[test]
fn strip_ansi_truncated_osc_preserves_trailing_text() {
    let input = "foo\x1b]8;;https://example.com";
    let out = strip_ansi(input);
    assert!(
        out.contains("foo") && out.contains("example.com"),
        "truncated OSC must not swallow text; got {out:?}"
    );
}

// -- Property tests (TEST-9 / TASK-1803) --
//
// The parser is a byte-oriented scanner over untrusted subprocess output whose
// demonstrated failure mode is "an input shape nobody thought to write down":
// eight bugs (TASK-0472 / 0613 / 0882 / 0949 / 0970 / 1028 / 1030 / 1054), six
// of them input-shape bugs, each patched with one more hand-written literal.
mod properties {
    use super::*;
    use proptest::prelude::*;

    /// Text that provably contains no escape introducer, so `strip_ansi` must
    /// be the identity on it.
    fn escape_free_text() -> impl Strategy<Value = String> {
        any::<String>().prop_filter("must contain no ESC", |s| !s.contains('\u{1b}'))
    }

    /// A complete, well-formed CSI sequence.
    fn csi_sequence() -> impl Strategy<Value = String> {
        "[0-9;]{0,6}".prop_map(|params| format!("\u{1b}[{params}m"))
    }

    fn crate_name() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_-]{0,12}"
    }

    fn version() -> impl Strategy<Value = String> {
        "[0-9]{1,3}\\.[0-9]{1,3}\\.[0-9]{1,3}"
    }

    proptest! {
        /// Arbitrary bytes must never panic the parser and must always
        /// terminate — the class TASK-1028 (CSI scan draining to EOF) belongs
        /// to.
        #[test]
        fn parse_update_output_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..1024)) {
            let result = parse_update_output(&bytes);
            // Also the headline invariant: the counts describe `entries`.
            // `saturating_add` only to satisfy the workspace's
            // `arithmetic_side_effects` deny; the counts are line counts.
            let total = result
                .update_count
                .saturating_add(result.downgrade_count)
                .saturating_add(result.add_count)
                .saturating_add(result.remove_count);
            prop_assert_eq!(result.entries.len(), total);
        }

        /// The counts must agree with `entries` for text-shaped input too —
        /// arbitrary bytes rarely produce entries, so drive real line shapes.
        #[test]
        fn counts_match_entries_for_line_shaped_input(
            lines in proptest::collection::vec(
                prop_oneof![
                    (crate_name(), version(), version())
                        .prop_map(|(n, a, b)| format!("    Updating {n} v{a} -> v{b}")),
                    (crate_name(), version(), version())
                        .prop_map(|(n, a, b)| format!(" Downgrading {n} v{a} -> v{b}")),
                    (crate_name(), version()).prop_map(|(n, v)| format!("      Adding {n} v{v}")),
                    (crate_name(), version()).prop_map(|(n, v)| format!("    Removing {n} v{v}")),
                    Just("    Updating crates.io index".to_string()),
                    Just("    Locking 3 packages".to_string()),
                    Just("warning: something".to_string()),
                ],
                0..12,
            ),
        ) {
            let stderr = lines.join("\n");
            let result = parse_update_output(stderr.as_bytes());
            // `saturating_add` only to satisfy the workspace's
            // `arithmetic_side_effects` deny; the counts are line counts.
            let total = result
                .update_count
                .saturating_add(result.downgrade_count)
                .saturating_add(result.add_count)
                .saturating_add(result.remove_count);
            prop_assert_eq!(result.entries.len(), total);
        }

        /// `strip_ansi` is the identity on escape-free input — including
        /// non-ASCII, the case TASK-0882's `bytes[i] as char` cast corrupted,
        /// and the no-allocation fast path TASK-0970 added.
        #[test]
        fn strip_ansi_is_identity_without_escapes(s in escape_free_text()) {
            let out = strip_ansi(&s);
            prop_assert_eq!(out.as_ref(), s.as_str());
        }

        /// Interleaving visible text with complete CSI sequences leaves
        /// exactly the visible text: no escape survives, and nothing visible
        /// is swallowed (TASK-1028).
        #[test]
        fn strip_ansi_removes_every_complete_csi(
            chunks in proptest::collection::vec(
                ("[a-zA-Z0-9 ._-]{0,12}", csi_sequence()),
                0..8,
            ),
        ) {
            let mut input = String::new();
            let mut expected = String::new();
            for (text, csi) in &chunks {
                input.push_str(text);
                input.push_str(csi);
                expected.push_str(text);
            }
            let out = strip_ansi(&input);
            prop_assert_eq!(out.as_ref(), expected.as_str());
            prop_assert!(!out.contains('\x1b'));
        }

        /// Round-trip: rendering a cargo-shaped line and re-parsing it yields
        /// the entry it described. Covers the verb / version / boundary family
        /// (TASK-0613 / 0949 / 1030 / 1054) instead of one literal at a time.
        #[test]
        fn rendered_action_lines_round_trip(
            name in crate_name(),
            from in version(),
            to in version(),
            verb_index in 0usize..4,
        ) {
            let (line, expected) = match verb_index {
                0 => (
                    format!("    Updating {name} v{from} -> v{to}"),
                    UpdateEntry {
                        action: UpdateAction::Update,
                        name,
                        from: Some(from),
                        to: Some(to),
                    },
                ),
                1 => (
                    format!(" Downgrading {name} v{from} -> v{to}"),
                    UpdateEntry {
                        action: UpdateAction::Downgrade,
                        name,
                        from: Some(from),
                        to: Some(to),
                    },
                ),
                2 => (
                    format!("      Adding {name} v{to}"),
                    UpdateEntry {
                        action: UpdateAction::Add,
                        name,
                        from: None,
                        to: Some(to),
                    },
                ),
                _ => (
                    format!("    Removing {name} v{from}"),
                    UpdateEntry {
                        action: UpdateAction::Remove,
                        name,
                        from: Some(from),
                        to: None,
                    },
                ),
            };
            let result = parse_update_output(line.as_bytes());
            prop_assert_eq!(result.entries.len(), 1);
            prop_assert_eq!(&result.entries[0], &expected);
        }
    }
}
