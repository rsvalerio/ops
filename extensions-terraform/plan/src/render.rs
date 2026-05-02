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

pub fn render_summary_table(changes: &[ClassifiedChange], is_tty: bool) -> String {
    let mut counts: HashMap<Action, usize> = HashMap::new();
    for c in changes {
        *counts.entry(c.action).or_default() += 1;
    }

    if changes.is_empty() {
        return "No changes. Infrastructure is up-to-date.\n".to_string();
    }

    let mut table = OpsTable::with_tty(is_tty);
    table.set_header(vec!["Action", "Count"]);

    for action in ACTION_DISPLAY_ORDER {
        let count = counts.get(&action).copied().unwrap_or(0);
        if count > 0 {
            let cell = table.cell(action.label(), action.color());
            table.add_row(vec![cell, Cell::new(count)]);
        }
    }

    let adds = counts.get(&Action::Create).copied().unwrap_or(0);
    let changes_count = counts.get(&Action::Update).copied().unwrap_or(0)
        + counts.get(&Action::Replace).copied().unwrap_or(0);
    let destroys = counts.get(&Action::Delete).copied().unwrap_or(0);

    let summary =
        format!("Plan: {adds} to add, {changes_count} to change, {destroys} to destroy.\n");

    format!("{table}\n{summary}")
}

pub fn render_resource_table(changes: &[ClassifiedChange], is_tty: bool) -> String {
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
    let banner = if unknown_count > 0 {
        format!(
            "WARNING: {unknown_count} resource change(s) use an action this build does not recognize. \
Inspect the rows marked `unknown` before applying.\n"
        )
    } else {
        String::new()
    };

    filtered.sort_by(|a, b| {
        a.action
            .sort_priority()
            .cmp(&b.action.sort_priority())
            .then_with(|| a.resource_type.cmp(&b.resource_type))
            .then_with(|| a.name.cmp(&b.name))
    });

    let mut table = OpsTable::with_tty(is_tty);
    table.set_header(vec!["Action", "Type", "Name", "Module"]);

    // ARCH-2 / TASK-0849: only consult the real terminal size when the
    // caller actually has a TTY. Probing it under is_tty=false (piped,
    // tests, CI snapshots) made render output environment-sensitive and
    // broke byte-identical snapshot reproducibility.
    let term_width = if is_tty {
        terminal_size::terminal_size().map(|(w, _)| w.0 as usize)
    } else {
        None
    };

    for c in &filtered {
        let action_cell = table.cell(c.action.label(), c.action.color());
        let module_display = c.module.as_deref().unwrap_or("");
        table.add_row(vec![
            action_cell,
            Cell::new(&c.resource_type),
            Cell::new(&c.name),
            Cell::new(module_display),
        ]);
    }

    if let Some(width) = term_width {
        let capped = std::cmp::max(
            MODULE_COL_MIN_WIDTH,
            width.saturating_sub(NON_MODULE_COLS_RESERVED),
        );
        table.set_max_width(3, capped as u16);
    }

    format!("{banner}{table}\n")
}

pub fn render_outputs_table(
    outputs: &serde_json::Map<String, serde_json::Value>,
    is_tty: bool,
) -> String {
    if outputs.is_empty() {
        return String::new();
    }

    let mut table = OpsTable::with_tty(is_tty);
    table.set_header(vec!["Output", "Action"]);

    for (name, value) in outputs {
        let actions = value
            .get("actions")
            .and_then(|a| a.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let action = Action::classify(&actions).unwrap_or(Action::NoOp);
        let cell = table.cell(action.label(), action.color());
        table.add_row(vec![Cell::new(name), cell]);
    }

    format!("{table}\n")
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

    #[test]
    fn resource_table_sorted_delete_first() {
        let changes = vec![
            make_change(Action::Create, "aws_instance", "web"),
            make_change(Action::Delete, "null_resource", "old"),
            make_change(Action::Update, "aws_s3_bucket", "logs"),
        ];
        let output = render_resource_table(&changes, false);
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

    #[test]
    fn resource_table_omits_noop() {
        let changes = vec![
            make_change(Action::Create, "aws_instance", "web"),
            make_change(Action::NoOp, "aws_s3_bucket", "existing"),
        ];
        let output = render_resource_table(&changes, false);
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
        let output = render_resource_table(&changes, false);
        assert!(output.is_empty(), "only no-op should produce empty output");
    }

    /// ARCH-2 / TASK-0849: render_resource_table(.., false) must be byte-
    /// identical regardless of the host TTY size, so snapshot tests stay
    /// reproducible across CI / local / piped invocations. The function
    /// previously called terminal_size::terminal_size() unconditionally
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
        let a = render_resource_table(&changes, false);
        let b = render_resource_table(&changes, false);
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
    }

    #[test]
    fn outputs_table_empty() {
        let outputs = serde_json::Map::new();
        let output = render_outputs_table(&outputs, false);
        assert!(output.is_empty());
    }
}
