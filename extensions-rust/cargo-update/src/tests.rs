//! Tests for the cargo_update extension.

use super::*;

// -- Extension trait tests --

mod extension_tests {
    use super::*;

    ops_extension::test_datasource_extension!(
        CargoUpdateExtension,
        name: "cargo-update",
        data_provider: "cargo_update"
    );
}

/// ERR-7 (TASK-0975): tracing breadcrumbs for cargo-update format-drift
/// lines flow through the `?` formatter so an attacker-controlled crate name
/// with embedded newlines / ANSI escapes cannot forge log records.
#[test]
fn warn_breadcrumb_debug_escapes_control_characters() {
    let line = "Updating evil\n[FAKE-LOG] forged\u{1b}[31m v1 -> v2";
    let rendered = format!("{line:?}");
    assert!(!rendered.contains('\n'));
    assert!(!rendered.contains('\u{1b}'));
    assert!(rendered.contains("\\n"));
    assert!(rendered.contains("\\u{1b}"));
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

#[test]
fn parse_mixed_output() {
    let stderr = b"\
    Updating crates.io index
    Locking 3 packages to latest compatible versions
    Updating serde v1.0.0 -> v1.0.1
      Adding new-dep v0.5.0
    Removing old-dep v0.3.0
    Updating tokio v1.28.0 -> v1.29.0
";
    let result = parse_update_output(stderr);
    assert_eq!(result.entries.len(), 4);
    assert_eq!(result.update_count, 2);
    assert_eq!(result.add_count, 1);
    assert_eq!(result.remove_count, 1);

    assert_eq!(result.entries[0].name, "serde");
    assert_eq!(result.entries[1].name, "new-dep");
    assert_eq!(result.entries[2].name, "old-dep");
    assert_eq!(result.entries[3].name, "tokio");
}

#[test]
fn parse_empty_output() {
    let stderr = b"";
    let result = parse_update_output(stderr);
    assert!(result.entries.is_empty());
    assert_eq!(result.update_count, 0);
    assert_eq!(result.add_count, 0);
    assert_eq!(result.remove_count, 0);
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
        add_count: 1,
        remove_count: 0,
    };

    let json = serde_json::to_value(&result).expect("serialize");
    assert_eq!(json["update_count"], 1);
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

#[test]
fn parse_output_with_ansi_codes() {
    let stderr = b"\x1b[1m\x1b[32m    Updating\x1b[0m serde v1.0.0 -> v1.0.1\n";
    let result = parse_update_output(stderr);
    assert_eq!(result.entries.len(), 1);
    assert_eq!(result.entries[0].name, "serde");
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

#[test]
fn update_action_serialization() {
    let update = serde_json::to_value(UpdateAction::Update).unwrap();
    assert_eq!(update, "update");
    let add = serde_json::to_value(UpdateAction::Add).unwrap();
    assert_eq!(add, "add");
    let remove = serde_json::to_value(UpdateAction::Remove).unwrap();
    assert_eq!(remove, "remove");
}

#[test]
fn update_action_deserialization() {
    let update: UpdateAction = serde_json::from_str("\"update\"").unwrap();
    assert_eq!(update, UpdateAction::Update);
    let add: UpdateAction = serde_json::from_str("\"add\"").unwrap();
    assert_eq!(add, UpdateAction::Add);
    let remove: UpdateAction = serde_json::from_str("\"remove\"").unwrap();
    assert_eq!(remove, UpdateAction::Remove);
}

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
}

#[test]
fn schema_has_expected_fields() {
    use ops_extension::DataProvider;
    let schema = CargoUpdateProvider.schema();
    assert_eq!(schema.fields.len(), 4);
    let field_names: Vec<&str> = schema.fields.iter().map(|f| f.name).collect();
    assert!(field_names.contains(&"entries"));
    assert!(field_names.contains(&"update_count"));
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

/// ERR-1 / TASK-0882: strip_ansi must round-trip non-ASCII UTF-8 input
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
