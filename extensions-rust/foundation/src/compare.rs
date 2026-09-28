//! Semantic comparison of a repo's TOML against a foundation template.
//!
//! The template is a baseline, not a copy: every key it sets must be present
//! with the same value, and keys the repo adds are its own business. Comments
//! and formatting never count. Arrays compare as sets, so reordering a list is
//! not drift but adding to or removing from it is — a widened license allow
//! list is exactly the divergence the check exists to surface.

use toml::Value;

/// How leaf values are compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    /// Scalars must be equal and arrays must hold the same items.
    Exact,
    /// Lint tables: a leaf is a lint spec (`"warn"` or `{ level = "warn", .. }`)
    /// and passes when the repo sets it at least as strictly.
    LintLevel,
}

/// One place where the repo diverges from the baseline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drift {
    /// `<file>` or `<file>:<dotted.key>` — also the key a waiver names.
    pub location: String,
    pub message: String,
}

impl Drift {
    pub(crate) fn new(location: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            location: location.into(),
            message: message.into(),
        }
    }
}

/// Compare `actual` against the baseline `expected`, appending every
/// divergence under `location` to `out`.
pub fn compare(
    expected: &Value,
    actual: Option<&Value>,
    location: &str,
    rule: Rule,
    out: &mut Vec<Drift>,
) {
    let Some(actual) = actual else {
        out.push(Drift::new(location, "missing"));
        return;
    };
    if rule == Rule::LintLevel {
        if let Some(want) = lint_level(expected) {
            compare_lint_level(want, actual, location, out);
            return;
        }
    }
    match (expected, actual) {
        (Value::Table(want), Value::Table(have)) => {
            for (key, value) in want {
                compare(value, have.get(key), &join(location, key), rule, out);
            }
        }
        (Value::Array(want), Value::Array(have)) => compare_sets(want, have, location, out),
        (want, have) if want == have => {}
        (want, have) => out.push(Drift::new(
            location,
            format!("expected {}, found {}", render(want), render(have)),
        )),
    }
}

fn compare_sets(want: &[Value], have: &[Value], location: &str, out: &mut Vec<Drift>) {
    let missing: Vec<String> = want
        .iter()
        .filter(|v| !have.contains(v))
        .map(render)
        .collect();
    let extra: Vec<String> = have
        .iter()
        .filter(|v| !want.contains(v))
        .map(render)
        .collect();
    if !missing.is_empty() {
        out.push(Drift::new(
            location,
            format!("missing items {}", missing.join(", ")),
        ));
    }
    if !extra.is_empty() {
        out.push(Drift::new(
            location,
            format!("unexpected items {}", extra.join(", ")),
        ));
    }
}

fn compare_lint_level(want: Level, actual: &Value, location: &str, out: &mut Vec<Drift>) {
    match lint_level(actual) {
        Some(have) if have >= want => {}
        Some(have) => out.push(Drift::new(
            location,
            format!(
                "level {} is weaker than the baseline {}",
                have.name(),
                want.name()
            ),
        )),
        None => out.push(Drift::new(
            location,
            format!("expected a lint level, found {}", render(actual)),
        )),
    }
}

/// Lint levels in strictness order, so `>=` reads "at least as strict".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    Allow,
    Warn,
    Deny,
    Forbid,
}

impl Level {
    const fn name(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Warn => "warn",
            Self::Deny => "deny",
            Self::Forbid => "forbid",
        }
    }
}

/// The level of a Cargo lint spec: `"warn"` or `{ level = "warn", priority = -1 }`.
fn lint_level(spec: &Value) -> Option<Level> {
    let name = match spec {
        Value::String(s) => s.as_str(),
        Value::Table(t) => t.get("level")?.as_str()?,
        _ => return None,
    };
    match name {
        "allow" => Some(Level::Allow),
        "warn" => Some(Level::Warn),
        "deny" => Some(Level::Deny),
        "forbid" => Some(Level::Forbid),
        _ => None,
    }
}

fn join(location: &str, key: &str) -> String {
    if location.contains(':') {
        format!("{location}.{key}")
    } else {
        format!("{location}:{key}")
    }
}

/// Inline TOML rendering for messages. `Value`'s `Display` renders inline, so
/// a table prints as `{ level = "warn" }` rather than a multi-line document.
fn render(value: &Value) -> String {
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> Value {
        Value::Table(toml::from_str(src).expect("valid toml"))
    }

    fn drift(expected: &str, actual: &str, rule: Rule) -> Vec<Drift> {
        let mut out = Vec::new();
        compare(
            &parse(expected),
            Some(&parse(actual)),
            "f.toml",
            rule,
            &mut out,
        );
        out
    }

    #[test]
    fn identical_tables_do_not_drift() {
        assert!(drift(
            "a = 1\n[t]\nb = \"x\"\n",
            "a = 1\n[t]\nb = \"x\"\n",
            Rule::Exact
        )
        .is_empty());
    }

    #[test]
    fn extra_keys_are_the_repos_own() {
        let out = drift(
            "a = 1\n",
            "a = 1\nignore = [\"RUSTSEC-1\"]\n[extra]\nk = 2\n",
            Rule::Exact,
        );
        assert!(out.is_empty(), "{out:?}");
    }

    #[test]
    fn missing_key_and_changed_value_drift_with_their_path() {
        let out = drift(
            "a = 1\n[t]\nb = \"x\"\nc = true\n",
            "a = 2\n[t]\nb = \"x\"\n",
            Rule::Exact,
        );
        assert_eq!(
            out,
            vec![
                Drift::new("f.toml:a", "expected 1, found 2"),
                Drift::new("f.toml:t.c", "missing"),
            ]
        );
    }

    #[test]
    fn arrays_compare_as_sets() {
        assert!(drift("l = [\"a\", \"b\"]\n", "l = [\"b\", \"a\"]\n", Rule::Exact).is_empty());
        let out = drift(
            "l = [\"a\", \"b\"]\n",
            "l = [\"a\", \"GPL-3.0\"]\n",
            Rule::Exact,
        );
        assert_eq!(
            out,
            vec![
                Drift::new("f.toml:l", "missing items \"b\""),
                Drift::new("f.toml:l", "unexpected items \"GPL-3.0\""),
            ]
        );
    }

    #[test]
    fn a_scalar_where_a_table_is_expected_drifts() {
        let out = drift("[t]\nb = 1\n", "t = 3\n", Rule::Exact);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].location, "f.toml:t");
    }

    #[test]
    fn stricter_lint_levels_satisfy_the_baseline() {
        let base = "[clippy]\nall = { level = \"warn\", priority = -1 }\nunwrap_used = \"warn\"\n";
        let repo =
            "[clippy]\nall = { level = \"deny\", priority = -1 }\nunwrap_used = \"forbid\"\n";
        assert!(drift(base, repo, Rule::LintLevel).is_empty());
    }

    #[test]
    fn weaker_or_missing_lints_drift() {
        let base = "[clippy]\npanic = \"warn\"\ntodo = \"warn\"\n";
        let out = drift(base, "[clippy]\npanic = \"allow\"\n", Rule::LintLevel);
        assert_eq!(
            out,
            vec![
                Drift::new(
                    "f.toml:clippy.panic",
                    "level allow is weaker than the baseline warn"
                ),
                Drift::new("f.toml:clippy.todo", "missing"),
            ]
        );
    }

    #[test]
    fn a_missing_file_level_value_is_reported_at_the_location() {
        let mut out = Vec::new();
        compare(&parse("a = 1\n"), None, "deny.toml", Rule::Exact, &mut out);
        assert_eq!(out, vec![Drift::new("deny.toml", "missing")]);
    }
}
