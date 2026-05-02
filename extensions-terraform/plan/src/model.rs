use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[non_exhaustive]
pub struct Plan {
    pub format_version: Option<String>,
    pub resource_changes: Option<Vec<ResourceChange>>,
    pub output_changes: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Deserialize, Debug)]
#[non_exhaustive]
pub struct ResourceChange {
    pub address: String,
    pub module: Option<String>,
    pub mode: Option<String>,
    pub r#type: Option<String>,
    pub name: Option<String>,
    pub change: Change,
}

#[derive(Deserialize, Debug)]
#[non_exhaustive]
pub struct Change {
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Action {
    Create,
    Delete,
    Update,
    Replace,
    Read,
    NoOp,
    /// SEC-31 (TASK-0833): a Terraform plan action this build does not
    /// recognize (e.g., `forget`, `import`, or a future variant). The
    /// renderer surfaces these with a distinct color and a warning banner
    /// so operators do not miss audit-relevant changes the tool cannot name.
    Unknown,
}

impl Action {
    /// Returns `None` for an empty action list (no actions reported by
    /// Terraform). For non-empty lists that do not match a known shape,
    /// returns `Some(Action::Unknown)` and emits a `tracing::warn!` with
    /// the raw action strings — fail-loud, not fail-open (SEC-31).
    pub fn classify(actions: &[String]) -> Option<Self> {
        match actions {
            [] => None,
            [s] if s == "no-op" => Some(Action::NoOp),
            [s] if s == "create" => Some(Action::Create),
            [s] if s == "read" => Some(Action::Read),
            [s] if s == "update" => Some(Action::Update),
            [s] if s == "delete" => Some(Action::Delete),
            [a, b] if (a == "delete" && b == "create") || (a == "create" && b == "delete") => {
                Some(Action::Replace)
            }
            other => {
                tracing::warn!(
                    actions = ?other,
                    "unrecognized terraform plan action sequence; surfacing as Unknown"
                );
                Some(Action::Unknown)
            }
        }
    }

    pub fn color(self) -> comfy_table::Color {
        match self {
            Action::Create => comfy_table::Color::Green,
            Action::Delete => comfy_table::Color::Red,
            Action::Update => comfy_table::Color::Yellow,
            Action::Replace => comfy_table::Color::Magenta,
            Action::Read => comfy_table::Color::Cyan,
            Action::NoOp => comfy_table::Color::DarkGrey,
            Action::Unknown => comfy_table::Color::DarkRed,
        }
    }

    pub fn is_change(self) -> bool {
        !matches!(self, Action::NoOp)
    }

    pub fn label(self) -> &'static str {
        match self {
            Action::Create => "create",
            Action::Delete => "delete",
            Action::Update => "update",
            Action::Replace => "replace",
            Action::Read => "read",
            Action::NoOp => "no-op",
            Action::Unknown => "unknown",
        }
    }

    /// Sort priority for resource table ordering. Lower = listed first.
    /// `Unknown` sorts first so audit-relevant unrecognized changes are
    /// the first thing the operator sees (SEC-31).
    pub fn sort_priority(self) -> u8 {
        match self {
            Action::Unknown => 0,
            Action::Delete => 1,
            Action::Replace => 2,
            Action::Create => 3,
            Action::Update => 4,
            Action::Read => 5,
            Action::NoOp => 6,
        }
    }
}

#[non_exhaustive]
pub struct ClassifiedChange {
    pub action: Action,
    pub address: String,
    pub resource_type: String,
    pub name: String,
    pub module: Option<String>,
    pub mode: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_no_op() {
        assert_eq!(Action::classify(&["no-op".into()]), Some(Action::NoOp));
    }

    #[test]
    fn classify_create() {
        assert_eq!(Action::classify(&["create".into()]), Some(Action::Create));
    }

    #[test]
    fn classify_delete() {
        assert_eq!(Action::classify(&["delete".into()]), Some(Action::Delete));
    }

    #[test]
    fn classify_update() {
        assert_eq!(Action::classify(&["update".into()]), Some(Action::Update));
    }

    #[test]
    fn classify_read() {
        assert_eq!(Action::classify(&["read".into()]), Some(Action::Read));
    }

    #[test]
    fn classify_replace_delete_create() {
        assert_eq!(
            Action::classify(&["delete".into(), "create".into()]),
            Some(Action::Replace)
        );
    }

    #[test]
    fn classify_replace_create_delete() {
        assert_eq!(
            Action::classify(&["create".into(), "delete".into()]),
            Some(Action::Replace)
        );
    }

    #[test]
    fn classify_empty() {
        assert_eq!(Action::classify(&[]), None);
    }

    #[test]
    fn classify_unknown_single_surfaces_as_unknown() {
        // SEC-31 (TASK-0833): an unrecognized single action is surfaced
        // as `Action::Unknown`, not silently dropped.
        assert_eq!(Action::classify(&["forget".into()]), Some(Action::Unknown));
    }

    #[test]
    fn classify_unknown_combination_surfaces_as_unknown() {
        // SEC-31 (TASK-0833): a combined-action sequence we do not
        // enumerate (e.g., ["create", "delete", "create"] or
        // ["import", "update"]) must surface, not vanish.
        assert_eq!(
            Action::classify(&["import".into(), "update".into()]),
            Some(Action::Unknown)
        );
    }

    #[test]
    fn is_change_false_for_noop() {
        assert!(!Action::NoOp.is_change());
    }

    #[test]
    fn is_change_true_for_create() {
        assert!(Action::Create.is_change());
    }

    #[test]
    fn is_change_true_for_all_non_noop() {
        for action in [
            Action::Create,
            Action::Delete,
            Action::Update,
            Action::Replace,
            Action::Read,
        ] {
            assert!(action.is_change(), "{action:?} should be a change");
        }
    }
}
