//! Human-readable description of one resource change: *what* it is about
//! (target), *what kind* of thing (kind) and *which attributes* change.
//!
//! Everything here is derived deterministically from the plan JSON so the
//! table an operator approves an apply from reads the same on every run.
//! Callers sanitize the returned strings before they leave the crate.

use serde_json::Value;

use crate::model::{Action, ResourceChange};

/// Resource names that say nothing beyond the type (`resource "x" "this"`).
const GENERIC_NAMES: [&str; 4] = ["this", "main", "default", "self"];

/// Attributes that name the real-world object, in preference order.
const NAME_ATTRIBUTES: [&str; 2] = ["display_name", "name"];

/// Human name of the object the change is about: its `display_name` or
/// `name` attribute (new value first, so a rename shows the new name), else
/// the module/`for_each`/`count` keys in its address joined by `/`.
pub fn target(rc: &ResourceChange) -> String {
    let c = &rc.change;
    NAME_ATTRIBUTES
        .iter()
        .find_map(|key| {
            known_str(c.after.as_ref(), c.after_sensitive.as_ref(), key)
                .or_else(|| known_str(c.before.as_ref(), c.before_sensitive.as_ref(), key))
        })
        .map_or_else(|| address_keys(&rc.address).join("/"), str::to_string)
}

/// The resource type without its provider prefix, spaced, plus the resource
/// name when it is neither generic nor already implied by the type.
pub fn kind(rc: &ResourceChange) -> String {
    let ty = rc.r#type.as_deref().unwrap_or_default();
    let name = rc.name.as_deref().unwrap_or_default();
    let tail = ty.split_once('_').map_or(ty, |(_, t)| t);
    let base = tail.replace('_', " ");
    let redundant = name.is_empty() || GENERIC_NAMES.contains(&name) || tail.contains(name);
    match (base.is_empty(), redundant) {
        (true, _) => name.to_string(),
        (false, true) => base,
        (false, false) => format!("{base} {name}"),
    }
}

/// What the change touches: the paths forcing a replace, or the top-level
/// attributes an update modifies; falls back to terraform's `action_reason`
/// (e.g. `tainted`, `no resource config`) when that is all there is.
pub fn changed(rc: &ResourceChange, action: Action) -> Vec<String> {
    let c = &rc.change;
    let specific = match action {
        Action::Replace => c
            .replace_paths
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|p| format_path(p))
            .filter(|p| !p.is_empty())
            .collect(),
        Action::Update | Action::Unknown => changed_keys(
            c.before.as_ref(),
            c.after.as_ref(),
            c.after_unknown.as_ref(),
        ),
        _ => Vec::new(),
    };
    if !specific.is_empty() {
        return specific;
    }
    rc.action_reason
        .as_deref()
        .map(humanize_reason)
        .into_iter()
        .collect()
}

/// A string attribute that is present, non-empty and not marked sensitive.
fn known_str<'a>(state: Option<&'a Value>, mask: Option<&Value>, key: &str) -> Option<&'a str> {
    let sensitive = match mask {
        Some(Value::Bool(b)) => *b,
        Some(m) => m.get(key).is_some_and(|v| v == &Value::Bool(true)),
        None => false,
    };
    if sensitive {
        return None;
    }
    state?.get(key)?.as_str().filter(|s| !s.is_empty())
}

/// Instance keys in a terraform address, in order:
/// `module.vm["vm1"].aws_eip.ip[0]` → `["vm1", "0"]`. Quoted keys may hold
/// `.`, `]` and escaped quotes, so this scans rather than splits.
fn address_keys(address: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut chars = address.chars();
    while let Some(ch) = chars.next() {
        if ch != '[' {
            continue;
        }
        let mut key = String::new();
        match chars.next() {
            Some('"') => {
                while let Some(k) = chars.next() {
                    match k {
                        '\\' => key.extend(chars.next()),
                        '"' => break,
                        _ => key.push(k),
                    }
                }
                // Consume the closing `]`.
                chars.next();
            }
            Some(first) => {
                key.push(first);
                key.extend(chars.by_ref().take_while(|&k| k != ']'));
            }
            None => break,
        }
        keys.push(key);
    }
    keys
}

/// `["rules", 0, "port"]` → `rules[0].port`.
fn format_path(steps: &[Value]) -> String {
    let mut out = String::new();
    for step in steps {
        match step {
            Value::Number(n) => {
                out.push('[');
                out.push_str(&n.to_string());
                out.push(']');
            }
            Value::String(s) => {
                if !out.is_empty() {
                    out.push('.');
                }
                out.push_str(s);
            }
            _ => {}
        }
    }
    out
}

/// Top-level attributes whose value differs or becomes known only after
/// apply. Nested changes are reported at their top-level attribute.
fn changed_keys(
    before: Option<&Value>,
    after: Option<&Value>,
    unknown: Option<&Value>,
) -> Vec<String> {
    let empty = serde_json::Map::new();
    let before = before.and_then(Value::as_object).unwrap_or(&empty);
    let after = after.and_then(Value::as_object).unwrap_or(&empty);
    let unknown = unknown.and_then(Value::as_object).unwrap_or(&empty);
    let mut keys: Vec<&String> = before
        .keys()
        .chain(after.keys())
        .chain(unknown.keys())
        .collect();
    keys.sort();
    keys.dedup();
    keys.into_iter()
        .filter(|k| {
            unknown.get(*k).is_some_and(has_unknown)
                || before.get(*k).unwrap_or(&Value::Null) != after.get(*k).unwrap_or(&Value::Null)
        })
        .cloned()
        .collect()
}

/// Whether an `after_unknown` subtree flags anything as unknown.
fn has_unknown(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Array(a) => a.iter().any(has_unknown),
        Value::Object(o) => o.values().any(has_unknown),
        _ => false,
    }
}

/// `replace_because_tainted` → `tainted`.
fn humanize_reason(reason: &str) -> String {
    reason
        .split_once("_because_")
        .map_or(reason, |(_, why)| why)
        .replace('_', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rc(json: serde_json::Value) -> ResourceChange {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn target_prefers_display_name_then_address_keys() {
        let named = rc(serde_json::json!({
            "address": "module.vm[\"a\"].oci_core_instance.instance",
            "change": { "actions": ["update"], "before": { "display_name": "old" }, "after": { "display_name": "vm1" } }
        }));
        assert_eq!(target(&named), "vm1");
        let keyed = rc(serde_json::json!({
            "address": "module.vm[\"vm1\"].oci_core_public_ip.public_ip[0]",
            "change": { "actions": ["update"] }
        }));
        assert_eq!(target(&keyed), "vm1/0");
    }

    #[test]
    fn target_skips_sensitive_names() {
        let secret = rc(serde_json::json!({
            "address": "x.y",
            "change": { "actions": ["update"], "after": { "name": "s" }, "after_sensitive": { "name": true } }
        }));
        assert_eq!(target(&secret), "");
    }

    #[test]
    fn address_keys_handle_quoted_punctuation() {
        assert_eq!(
            address_keys(r#"module.a["x.]\"y"].t.n[3]"#),
            vec!["x.]\"y", "3"]
        );
        assert!(address_keys("aws_instance.web").is_empty());
    }

    #[test]
    fn kind_drops_provider_and_redundant_names() {
        let k = |ty: &str, name: &str| {
            kind(&rc(serde_json::json!({
                "address": "a", "type": ty, "name": name, "change": { "actions": ["create"] }
            })))
        };
        assert_eq!(k("oci_core_public_ip", "public_ip"), "core public ip");
        assert_eq!(
            k("oci_core_vnic_attachments", "vnic_attachment"),
            "core vnic attachments"
        );
        assert_eq!(
            k("oci_identity_dynamic_group", "this"),
            "identity dynamic group"
        );
        assert_eq!(
            k("local_file", "testinfra_ssh_config"),
            "file testinfra_ssh_config"
        );
    }

    #[test]
    fn changed_lists_replace_paths_and_update_keys() {
        let replace = rc(serde_json::json!({
            "address": "a",
            "change": { "actions": ["delete", "create"], "replace_paths": [["source_details", "image_id"], ["rules", 0, "port"]] }
        }));
        assert_eq!(
            changed(&replace, Action::Replace),
            vec!["source_details.image_id", "rules[0].port"]
        );

        let update = rc(serde_json::json!({
            "address": "a",
            "change": {
                "actions": ["update"],
                "before": { "id": "1", "private_ip_id": "p1", "tags": { "a": "1" } },
                "after": { "id": "1", "tags": { "a": "2" } },
                "after_unknown": { "private_ip_id": true, "tags": {} }
            }
        }));
        assert_eq!(
            changed(&update, Action::Update),
            vec!["private_ip_id", "tags"]
        );
    }

    #[test]
    fn changed_falls_back_to_action_reason() {
        let tainted = rc(serde_json::json!({
            "address": "a",
            "action_reason": "replace_because_tainted",
            "change": { "actions": ["delete", "create"] }
        }));
        assert_eq!(changed(&tainted, Action::Replace), vec!["tainted"]);
        let create = rc(serde_json::json!({ "address": "a", "change": { "actions": ["create"] } }));
        assert!(changed(&create, Action::Create).is_empty());
    }
}
