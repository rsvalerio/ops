//! Deserialization contract with terraform's JSON plan format.
//!
//! The types below map `terraform show -json` output (plan JSON
//! `format_version` 1.x, pinned in tests against 1.2; nothing here gates on
//! the version field). Throughout, an `Option` field means "absent from the
//! document", not "not applicable" — terraform omits keys rather than
//! nulling them.

use serde::Deserialize;

/// A terraform plan document, as emitted by `terraform show -json`.
#[derive(Deserialize, Debug)]
#[non_exhaustive]
pub struct Plan {
    /// Plan format version string (e.g. `"1.2"`); informational, not gated
    /// on. `None` when the document omits it.
    pub format_version: Option<String>,
    /// One entry per resource terraform plans to touch; `None` when the
    /// document carries no `resource_changes` array.
    pub resource_changes: Option<Vec<ResourceChange>>,
    /// Planned output value changes keyed by output name; `None` when the
    /// document carries no `output_changes` object.
    pub output_changes: Option<serde_json::Map<String, serde_json::Value>>,
}

/// One planned change to a single resource instance.
#[derive(Deserialize, Debug)]
#[non_exhaustive]
pub struct ResourceChange {
    /// Terraform address of the resource (e.g. `aws_instance.web[0]`).
    pub address: String,
    /// Module path prefix terraform reports the resource under; `None` for
    /// root-module resources.
    pub module: Option<String>,
    /// Management mode, `"managed"` or `"data"`; `None` when omitted.
    pub mode: Option<String>,
    /// Resource type (e.g. `aws_instance`); `None` when omitted.
    pub r#type: Option<String>,
    /// Resource name segment of the address; `None` when omitted.
    pub name: Option<String>,
    /// The change itself: its action sequence.
    pub change: Change,
}

/// The action sequence terraform plans for one resource.
#[derive(Deserialize, Debug)]
#[non_exhaustive]
pub struct Change {
    /// Raw action verbs in order (e.g. `["delete", "create"]` for a
    /// replace).
    pub actions: Vec<String>,
}

/// The action classification this crate renders and sorts by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Action {
    /// Resource will be created.
    Create,
    /// Resource will be destroyed.
    Delete,
    /// Resource will be updated in place.
    Update,
    /// Resource will be destroyed and recreated.
    Replace,
    /// Resource will only be read.
    Read,
    /// Nothing will happen.
    NoOp,
    /// SEC-31 (TASK-0833): a Terraform plan action this build does not
    /// recognize (e.g., `forget`, `import`, or a future variant). The
    /// renderer surfaces these with a distinct color and a warning banner
    /// so operators do not miss audit-relevant changes the tool cannot name.
    Unknown,
}

impl Action {
    /// Classifies a raw terraform action sequence into one [`Action`].
    ///
    /// Returns `None` for an empty action list (no actions reported by
    /// Terraform). For non-empty lists that do not match a known shape,
    /// returns `Some(Action::Unknown)` and emits a `tracing::warn!` with
    /// the raw action strings — fail-loud, not fail-open (SEC-31).
    #[must_use = "the classified action drives rendering, colour and sorting"]
    pub fn classify(actions: &[String]) -> Option<Self> {
        match actions {
            [] => None,
            [s] if s == "no-op" => Some(Self::NoOp),
            [s] if s == "create" => Some(Self::Create),
            [s] if s == "read" => Some(Self::Read),
            [s] if s == "update" => Some(Self::Update),
            [s] if s == "delete" => Some(Self::Delete),
            [a, b] if (a == "delete" && b == "create") || (a == "create" && b == "delete") => {
                Some(Self::Replace)
            }
            other => {
                tracing::warn!(
                    actions = ?other,
                    "unrecognized terraform plan action sequence; surfacing as Unknown"
                );
                Some(Self::Unknown)
            }
        }
    }

    /// Terminal colour this action renders with.
    #[must_use = "use the returned colour; classification has no side effect"]
    pub const fn color(self) -> comfy_table::Color {
        match self {
            Self::Create => comfy_table::Color::Green,
            Self::Delete => comfy_table::Color::Red,
            Self::Update => comfy_table::Color::Yellow,
            Self::Replace => comfy_table::Color::Magenta,
            Self::Read => comfy_table::Color::Cyan,
            Self::NoOp => comfy_table::Color::DarkGrey,
            Self::Unknown => comfy_table::Color::DarkRed,
        }
    }

    /// Whether this action modifies anything (everything except `NoOp`).
    #[must_use = "branch on the verdict; classification has no side effect"]
    pub const fn is_change(self) -> bool {
        !matches!(self, Self::NoOp)
    }

    /// Lowercase display label for this action (e.g. `"no-op"`).
    #[must_use = "render the returned label; classification has no side effect"]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Delete => "delete",
            Self::Update => "update",
            Self::Replace => "replace",
            Self::Read => "read",
            Self::NoOp => "no-op",
            Self::Unknown => "unknown",
        }
    }

    /// Sort priority for resource table ordering. Lower = listed first.
    /// `Unknown` sorts first so audit-relevant unrecognized changes are
    /// the first thing the operator sees (SEC-31).
    #[must_use = "use the returned priority for ordering; it has no side effect"]
    pub const fn sort_priority(self) -> u8 {
        match self {
            Self::Unknown => 0,
            Self::Delete => 1,
            Self::Replace => 2,
            Self::Create => 3,
            Self::Update => 4,
            Self::Read => 5,
            Self::NoOp => 6,
        }
    }
}

/// One sanitized, display-ready change produced by `classify_plan`.
#[non_exhaustive]
pub struct ClassifiedChange {
    /// Classified action; drives colour, label and sort order.
    pub action: Action,
    /// Sanitized resource address.
    pub address: String,
    /// Sanitized resource type (empty when terraform omitted `type`).
    pub resource_type: String,
    /// Sanitized resource name (empty when terraform omitted `name`).
    pub name: String,
    /// Sanitized module path; `None` for root-module resources.
    pub module: Option<String>,
    /// Management mode, defaulted to `"managed"` when terraform omitted it.
    /// Populated and sanitized but not rendered by this crate.
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
