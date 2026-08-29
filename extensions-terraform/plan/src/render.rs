use std::collections::HashMap;

use ops_core::table::{Cell, OpsTable};

use crate::model::{Action, ClassifiedChange};

/// READ-5 / TASK-0920: minimum width for the wrapping `Module` column.
/// Below this the table looks broken; we'd rather wrap module paths.
const MODULE_COL_MIN_WIDTH: usize = 20;

/// READ-5 / TASK-0920: columns the change table reserves for the three
/// non-wrapping cells (`Action`, `Type`, `Name`) plus the four `│ … │`
/// separators of `OpsTable`'s frame. The remaining terminal columns
/// after subtracting this budget are handed to the `Module` column. If a
/// future column is added or removed, update this constant alongside
/// `set_header(...)` so the budget reflects the new shape.
const NON_MODULE_COLS_RESERVED: usize = 40;

const ACTION_DISPLAY_ORDER: [Action; 7] = [
    Action::Unknown,
    Action::Create,
    Action::Delete,
    Action::Update,
    Action::Replace,
    Action::Read,
    Action::NoOp,
];

/// SEC-11 / TASK-1939, TASK-2032: this crate used to carry its own
/// sanitizer, but `OpsTable` is the shared sink for every table in the
/// workspace, so the defence now lives there and every caller gets it.
/// Re-exported under the local name because the plan model also sanitizes
/// values that are logged rather than tabulated (see `lib.rs`).
pub(crate) use ops_core::table::sanitise_table_text as sanitize_terminal_text;

/// SEC-31 (TASK-0833 / TASK-1954): the banner shown above a table whose
/// rows carry an action this build cannot name, so an operator does not
/// miss audit-relevant changes. Shared by the resource and outputs
/// tables on the same terms.
fn unknown_banner(unknown_count: usize, noun: &str) -> String {
    if unknown_count == 0 {
        return String::new();
    }
    format!(
        "WARNING: {unknown_count} {noun} change(s) use an action this build does not recognize. \
Inspect the rows marked `unknown` before applying.\n"
    )
}

#[must_use]
pub fn render_summary_table(changes: &[ClassifiedChange], use_color: bool) -> String {
    let mut counts: HashMap<Action, usize> = HashMap::new();
    for c in changes {
        // Each tally counts a distinct element of the in-memory `changes`
        // slice, so no count can exceed `changes.len()` (at most `isize::MAX`)
        // and `saturating_add` is exactly `+ 1`.
        let count = counts.entry(c.action).or_default();
        *count = count.saturating_add(1);
    }

    if changes.is_empty() {
        return "No changes. Infrastructure is up-to-date.\n".to_string();
    }

    // PATTERN-1 / TASK-1017: `OpsTable::with_tty` only gates colour in
    // `cell()`, so the colour preference (not TTY detection) is what
    // belongs here. Width-aware rendering is decoupled in
    // `render_resource_table` via a separate `is_tty` flag.
    let mut table = OpsTable::with_tty(use_color);
    table.set_header(vec!["Action", "Count"]);

    for action in ACTION_DISPLAY_ORDER {
        let count = counts.get(&action).copied().unwrap_or(0);
        if count > 0 {
            let cell = table.cell(action.label(), action.color());
            table.add_row(vec![cell, Cell::new(count)]);
        }
    }

    let adds = counts.get(&Action::Create).copied().unwrap_or(0);
    // Disjoint tallies over the same slice, so the sum is at most
    // `changes.len()` and `saturating_add` is exactly `+`.
    let changes_count = counts
        .get(&Action::Update)
        .copied()
        .unwrap_or(0)
        .saturating_add(counts.get(&Action::Replace).copied().unwrap_or(0));
    let destroys = counts.get(&Action::Delete).copied().unwrap_or(0);

    let summary =
        format!("Plan: {adds} to add, {changes_count} to change, {destroys} to destroy.\n");

    format!("{table}\n{summary}")
}

/// PATTERN-1 / TASK-1017: `is_tty` drives terminal-width probing
/// (right-sizing the `Module` column); `use_color` drives whether
/// `Action::color()` is applied to cells.
///
/// The two were previously conflated under one boolean, which (a) made
/// piped-but-coloured output environment-sensitive and (b) disabled
/// width probing on a real TTY when `--no-color` was set. Callers must
/// derive `is_tty` from `IsTerminal` on the actual writer (or pass
/// `false` for buffered sinks) and `use_color` from the user's
/// preference (e.g. `!no_color`).
#[must_use]
pub fn render_resource_table(
    changes: &[ClassifiedChange],
    is_tty: bool,
    use_color: bool,
) -> String {
    let mut filtered: Vec<&ClassifiedChange> =
        changes.iter().filter(|c| c.action.is_change()).collect();

    if filtered.is_empty() {
        return String::new();
    }

    // SEC-31 (TASK-0833): if any change carries an unrecognized action,
    // prepend a banner so an operator does not miss audit-relevant rows
    // they cannot name. The rows themselves render with `Action::Unknown`
    // styling and sort to the top of the table.
    let unknown_count = filtered
        .iter()
        .filter(|c| matches!(c.action, Action::Unknown))
        .count();
    let banner = unknown_banner(unknown_count, "resource");

    filtered.sort_by(|a, b| {
        a.action
            .sort_priority()
            .cmp(&b.action.sort_priority())
            .then_with(|| a.resource_type.cmp(&b.resource_type))
            .then_with(|| a.name.cmp(&b.name))
    });

    // PATTERN-1 / TASK-1017: `OpsTable::with_tty` gates colour in
    // `cell()`, so the colour preference is what belongs here. The
    // `is_tty` flag controls width probing below.
    let mut table = OpsTable::with_tty(use_color);
    table.set_header(vec!["Action", "Type", "Name", "Module"]);

    // ARCH-2 / TASK-0849: only consult the real terminal size when the
    // caller actually has a TTY. Probing it under is_tty=false (piped,
    // tests, CI snapshots) made render output environment-sensitive and
    // broke byte-identical snapshot reproducibility.
    let term_width = if is_tty {
        terminal_size::terminal_size().map(|(w, _)| usize::from(w.0))
    } else {
        None
    };

    for c in &filtered {
        let action_cell = table.cell(c.action.label(), c.action.color());
        let module_display = c.module.as_deref().unwrap_or("");
        table.add_row(vec![
            action_cell,
            OpsTable::text_cell(&c.resource_type),
            OpsTable::text_cell(&c.name),
            OpsTable::text_cell(module_display),
        ]);
    }

    if let Some(width) = term_width {
        let capped = std::cmp::max(
            MODULE_COL_MIN_WIDTH,
            width.saturating_sub(NON_MODULE_COLS_RESERVED),
        );
        table.set_max_width(3, u16::try_from(capped).unwrap_or(u16::MAX));
    }

    format!("{banner}{table}\n")
}

#[must_use]
pub fn render_outputs_table(
    outputs: &serde_json::Map<String, serde_json::Value>,
    use_color: bool,
) -> String {
    if outputs.is_empty() {
        return String::new();
    }

    // PATTERN-1 / TASK-1017: see `render_summary_table`. The outputs
    // table has no width-aware column, so it only needs the colour
    // preference.
    let mut table = OpsTable::with_tty(use_color);
    table.set_header(vec!["Output", "Action"]);

    let mut unknown_count: usize = 0;
    for (name, value) in outputs {
        let action = classify_output_action(name, value);
        if matches!(action, Action::Unknown) {
            // At most one increment per entry of `outputs`, so this can
            // never exceed `outputs.len()`; `saturating_add` is `+ 1`.
            unknown_count = unknown_count.saturating_add(1);
        }
        let cell = table.cell(action.label(), action.color());
        table.add_row(vec![OpsTable::text_cell(name), cell]);
    }

    format!("{}{table}\n", unknown_banner(unknown_count, "output"))
}

/// SEC-31 / TASK-1954: fail closed when an output's planned change
/// cannot be read.
///
/// A missing `actions` key, a value that is not an array, an array
/// holding non-string entries, and an empty sequence all used to
/// collapse to `Action::NoOp` — labelling the row "nothing is happening
/// to this output" precisely when this build could not tell. Terraform
/// outputs are frequently a stack's sensitive surface (generated
/// credentials, endpoints), and the operator reads this table before
/// approving an apply. The resource side has surfaced the same
/// degradation as `Action::Unknown` with a `tracing::warn!` since
/// TASK-0833 (`Action::classify`); the outputs table now matches.
fn classify_output_action(name: &str, value: &serde_json::Value) -> Action {
    // SEC-11 / TASK-1939: the output key is untrusted plan text and this
    // event may land on a terminal, so sanitize before logging it too.
    let Some(array) = value.get("actions").and_then(serde_json::Value::as_array) else {
        tracing::warn!(
            output = %sanitize_terminal_text(name),
            "terraform output change has no readable `actions` array; surfacing as Unknown"
        );
        return Action::Unknown;
    };

    let actions: Vec<String> = array
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    if actions.len() != array.len() {
        tracing::warn!(
            output = %sanitize_terminal_text(name),
            "terraform output `actions` array holds non-string entries; surfacing as Unknown"
        );
        return Action::Unknown;
    }

    Action::classify(&actions).unwrap_or_else(|| {
        tracing::warn!(
            output = %sanitize_terminal_text(name),
            "terraform output reports an empty `actions` list; surfacing as Unknown"
        );
        Action::Unknown
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_change(action: Action, rtype: &str, name: &str) -> ClassifiedChange {
        ClassifiedChange {
            action,
            address: format!("{rtype}.{name}"),
            resource_type: rtype.to_string(),
            name: name.to_string(),
            module: None,
            mode: "managed".to_string(),
        }
    }

    fn make_change_in_module(
        action: Action,
        rtype: &str,
        name: &str,
        module: &str,
    ) -> ClassifiedChange {
        ClassifiedChange {
            module: Some(module.to_string()),
            ..make_change(action, rtype, name)
        }
    }

    #[test]
    fn summary_table_shows_nonzero_actions() {
        let changes = vec![
            make_change(Action::Create, "aws_instance", "web"),
            make_change(Action::Create, "aws_instance", "api"),
            make_change(Action::Delete, "null_resource", "old"),
        ];
        let output = render_summary_table(&changes, false);
        assert!(output.contains("create"), "should contain create: {output}");
        assert!(output.contains("delete"), "should contain delete: {output}");
        assert!(
            !output.contains("update"),
            "should not contain update: {output}"
        );
        assert!(
            output.contains("2 to add"),
            "should contain '2 to add': {output}"
        );
        assert!(
            output.contains("1 to destroy"),
            "should contain '1 to destroy': {output}"
        );
    }

    #[test]
    fn summary_table_empty_changes() {
        let output = render_summary_table(&[], false);
        assert!(
            output.contains("No changes"),
            "empty should say no changes: {output}"
        );
    }

    /// TEST-6 / TASK-1956: `ACTION_DISPLAY_ORDER` lists `Unknown` first
    /// so an unrecognized action is the first thing in the summary. That
    /// ordering had no test.
    #[test]
    fn summary_table_lists_unknown_before_other_actions() {
        let changes = vec![
            make_change(Action::Create, "aws_instance", "web"),
            make_change(Action::Delete, "null_resource", "old"),
            make_change(Action::Unknown, "aws_instance", "gone"),
        ];
        let output = render_summary_table(&changes, false);
        let unknown_pos = output.find("unknown").expect("unknown row present");
        let create_pos = output.find("create").expect("create row present");
        let delete_pos = output.find("delete").expect("delete row present");
        assert!(
            unknown_pos < create_pos && unknown_pos < delete_pos,
            "unknown must be listed first: {output}"
        );
    }

    #[test]
    fn resource_table_sorted_delete_first() {
        let changes = vec![
            make_change(Action::Create, "aws_instance", "web"),
            make_change(Action::Delete, "null_resource", "old"),
            make_change(Action::Update, "aws_s3_bucket", "logs"),
        ];
        let output = render_resource_table(&changes, false, false);
        let delete_pos = output.find("delete").expect("delete should be present");
        let create_pos = output.find("create").expect("create should be present");
        let update_pos = output.find("update").expect("update should be present");
        assert!(
            delete_pos < create_pos,
            "delete should appear before create"
        );
        assert!(
            create_pos < update_pos,
            "create should appear before update"
        );
    }

    /// TEST-6 / TASK-1956: the SEC-31 banner exists so an operator cannot
    /// miss rows the tool cannot name. An inverted condition or a renamed
    /// label would have passed the whole suite before this test.
    #[test]
    fn resource_table_unknown_action_shows_warning_banner() {
        let changes = vec![
            make_change(Action::Unknown, "aws_instance", "gone"),
            make_change(Action::Unknown, "aws_instance", "imported"),
            make_change(Action::Create, "aws_instance", "web"),
        ];
        let output = render_resource_table(&changes, false, false);
        assert!(
            output.starts_with(
                "WARNING: 2 resource change(s) use an action this build does not recognize."
            ),
            "banner must lead the table and report the count: {output}"
        );
        assert!(
            output.contains("Inspect the rows marked `unknown` before applying."),
            "banner must tell the operator what to do: {output}"
        );
    }

    /// TEST-6 / TASK-1956: no banner when nothing is unrecognized.
    #[test]
    fn resource_table_without_unknown_has_no_banner() {
        let changes = vec![make_change(Action::Create, "aws_instance", "web")];
        let output = render_resource_table(&changes, false, false);
        assert!(
            !output.contains("WARNING"),
            "no banner without unknown rows: {output}"
        );
    }

    /// TEST-6 / TASK-1956: `sort_priority() == 0` puts Unknown above
    /// Delete. The existing sort test only compared delete/create/update.
    #[test]
    fn resource_table_sorts_unknown_above_delete() {
        let changes = vec![
            make_change(Action::Delete, "null_resource", "old"),
            make_change(Action::Unknown, "zzz_last_by_type", "gone"),
        ];
        let output = render_resource_table(&changes, false, false);
        let unknown_pos = output.find("unknown").expect("unknown row present");
        let delete_pos = output.find("delete").expect("delete row present");
        assert!(
            unknown_pos < delete_pos,
            "unknown must sort above delete even with a later type: {output}"
        );
    }

    /// TEST-6 / TASK-1956: every `make_change` helper set `module: None`,
    /// so `c.module.as_deref().unwrap_or("")` had only ever been
    /// exercised on the `None` side.
    #[test]
    fn resource_table_renders_the_module_column() {
        let changes = vec![
            make_change_in_module(
                Action::Create,
                "aws_instance",
                "web",
                "module.networking.module.vpc",
            ),
            make_change(Action::Delete, "null_resource", "old"),
        ];
        let output = render_resource_table(&changes, false, false);
        assert!(
            output.contains("module.networking.module.vpc"),
            "Some(module) must render: {output}"
        );
        assert!(
            output.contains("null_resource"),
            "None module must still render its row: {output}"
        );
    }

    #[test]
    fn resource_table_omits_noop() {
        let changes = vec![
            make_change(Action::Create, "aws_instance", "web"),
            make_change(Action::NoOp, "aws_s3_bucket", "existing"),
        ];
        let output = render_resource_table(&changes, false, false);
        assert!(
            !output.contains("no-op"),
            "no-op should be filtered: {output}"
        );
        assert!(
            output.contains("aws_instance"),
            "create should be present: {output}"
        );
    }

    #[test]
    fn resource_table_empty_after_filter() {
        let changes = vec![make_change(Action::NoOp, "aws_s3_bucket", "existing")];
        let output = render_resource_table(&changes, false, false);
        assert!(output.is_empty(), "only no-op should produce empty output");
    }

    /// SEC-11 / TASK-1939: an escape sequence or a bare carriage return
    /// in a resource name must never reach the operator's terminal — it
    /// can erase the rows already printed and redraw a fake summary line
    /// on the screen an apply is approved from.
    #[test]
    fn resource_table_strips_control_characters() {
        let changes = vec![make_change_in_module(
            Action::Create,
            "aws_\u{1b}[31minstance",
            "web\r\u{1b}[2K\u{1b}[1A",
            "module.a\u{7f}b",
        )];
        let output = render_resource_table(&changes, false, false);
        assert!(
            !output.contains('\u{1b}'),
            "no ESC byte may survive: {output:?}"
        );
        assert!(!output.contains('\r'), "no CR byte may survive: {output:?}");
        assert!(
            !output.contains('\u{7f}'),
            "no DEL byte may survive: {output:?}"
        );
        assert!(
            output.contains("aws_[31minstance"),
            "the visible text must still render: {output}"
        );
    }

    /// ARCH-2 / TASK-0849: `render_resource_table`(.., false) must be byte-
    /// identical regardless of the host TTY size, so snapshot tests stay
    /// reproducible across CI / local / piped invocations. The function
    /// previously called `terminal_size::terminal_size()` unconditionally
    /// which made output environment-sensitive.
    #[test]
    fn resource_table_non_tty_output_is_stable_across_term_widths() {
        let changes = vec![
            make_change(Action::Create, "aws_instance", "web"),
            make_change(Action::Update, "aws_s3_bucket", "logs"),
            make_change(Action::Delete, "null_resource", "old"),
        ];
        // Drive two non-TTY renders back-to-back. Any branch that consults
        // the real terminal size could theoretically observe a window
        // resize between them; under is_tty=false they must NOT call
        // terminal_size at all and the output is therefore identical.
        let a = render_resource_table(&changes, false, false);
        let b = render_resource_table(&changes, false, false);
        assert_eq!(a, b, "non-TTY output must be deterministic");
        // Sanity: width-dependent module-column truncation should not
        // appear when no TTY is available — the final column carries the
        // full module name (here, an empty string is fine).
        assert!(a.contains("aws_instance"), "full type must be present: {a}");
    }

    #[test]
    fn outputs_table_renders_actions() {
        let mut outputs = serde_json::Map::new();
        outputs.insert("region".into(), serde_json::json!({"actions": ["create"]}));
        outputs.insert("vpc_id".into(), serde_json::json!({"actions": ["update"]}));
        let output = render_outputs_table(&outputs, false);
        assert!(output.contains("region"), "should contain region: {output}");
        assert!(output.contains("vpc_id"), "should contain vpc_id: {output}");
        assert!(output.contains("create"), "should contain create: {output}");
        assert!(output.contains("update"), "should contain update: {output}");
        assert!(
            !output.contains("WARNING"),
            "readable outputs need no banner: {output}"
        );
    }

    #[test]
    fn outputs_table_empty() {
        let outputs = serde_json::Map::new();
        let output = render_outputs_table(&outputs, false);
        assert!(output.is_empty());
    }

    /// SEC-31 / TASK-1954: fail closed. Each of these used to render as
    /// `no-op`, telling the operator nothing was happening to an output
    /// whose planned change this build could not read.
    #[test]
    fn outputs_table_degraded_actions_render_as_unknown() {
        for (label, value) in [
            ("missing actions key", serde_json::json!({"before": 1})),
            (
                "non-array actions",
                serde_json::json!({"actions": "create"}),
            ),
            (
                "non-string entries",
                serde_json::json!({"actions": [1, {"a": 2}, null]}),
            ),
            ("empty actions array", serde_json::json!({"actions": []})),
        ] {
            let mut outputs = serde_json::Map::new();
            outputs.insert("db_password".into(), value);
            let output = render_outputs_table(&outputs, false);
            assert!(
                output.contains("unknown"),
                "{label} must render as unknown: {output}"
            );
            assert!(
                !output.contains("no-op"),
                "{label} must not fail open to no-op: {output}"
            );
            assert!(
                output.starts_with(
                    "WARNING: 1 output change(s) use an action this build does not recognize."
                ),
                "{label} must raise the banner: {output}"
            );
        }
    }

    /// SEC-31 / TASK-1954: a mixed table counts only the degraded rows.
    #[test]
    fn outputs_table_banner_counts_only_unreadable_outputs() {
        let mut outputs = serde_json::Map::new();
        outputs.insert("region".into(), serde_json::json!({"actions": ["create"]}));
        outputs.insert("db_password".into(), serde_json::json!({"before": 1}));
        outputs.insert("api_key".into(), serde_json::json!({"actions": [7]}));
        let output = render_outputs_table(&outputs, false);
        assert!(
            output.starts_with("WARNING: 2 output change(s)"),
            "only the two unreadable outputs count: {output}"
        );
        assert!(output.contains("create"), "readable row still renders");
    }

    /// SEC-11 / TASK-1939: the output map key is untrusted plan text too.
    #[test]
    fn outputs_table_strips_control_characters_from_keys() {
        let mut outputs = serde_json::Map::new();
        outputs.insert(
            "db_\u{1b}[2Kpassword\r".into(),
            serde_json::json!({"actions": ["create"]}),
        );
        let output = render_outputs_table(&outputs, false);
        assert!(
            !output.contains('\u{1b}'),
            "no ESC byte may survive: {output:?}"
        );
        assert!(!output.contains('\r'), "no CR byte may survive: {output:?}");
        assert!(
            output.contains("db_[2Kpassword"),
            "the visible text must still render: {output}"
        );
    }
}
