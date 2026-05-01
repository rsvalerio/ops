use std::collections::HashMap;

use ops_core::table::{Cell, OpsTable};

use crate::model::{Action, ClassifiedChange};

const ACTION_DISPLAY_ORDER: [Action; 6] = [
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

    filtered.sort_by(|a, b| {
        a.action
            .sort_priority()
            .cmp(&b.action.sort_priority())
            .then_with(|| a.resource_type.cmp(&b.resource_type))
            .then_with(|| a.name.cmp(&b.name))
    });

    let mut table = OpsTable::with_tty(is_tty);
    table.set_header(vec!["Action", "Type", "Name", "Module"]);

    let term_width = terminal_size::terminal_size().map(|(w, _)| w.0 as usize);

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
        let capped = std::cmp::max(20, width.saturating_sub(40));
        table.set_max_width(3, capped as u16);
    }

    format!("{table}\n")
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
