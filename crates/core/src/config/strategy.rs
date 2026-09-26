//! `[commands.<name>.strategy]`: run one exec command once per matrix cell
//! (TASK-2277), modelled on GitHub Actions `strategy`.
//!
//! ```toml
//! [commands.doc-default]
//! program = "cargo"
//! args = ["doc", "--no-deps", "-p", "${matrix.crate}"]
//!
//! [commands.doc-default.strategy]
//! matrix = { crate = ["dbsec-core", "dbsec"] }
//! max_parallel = 1
//! fail_fast = false
//! ```
//!
//! A matrix command is **one step** to the enclosing plan: its `exclusive`
//! covers the whole matrix and it succeeds only when every cell succeeds.
//! `max_parallel` / `fail_fast` here govern only the cells, never the plan
//! around them — the runner schedules the cells inside the step.
//!
//! Cells follow GitHub Actions semantics: the axes form a Cartesian product
//! (ordered by axis name), `exclude` drops every product cell matching all of
//! an entry's pairs, then each `include` entry is merged into every product
//! cell it does not contradict on an axis value — or, when it fits none,
//! becomes a cell of its own.
//!
//! `${matrix.<key>}` is substituted into `args`, `env` values and `cwd` by
//! [`substitute`], in its own pass before `${VAR}` expansion. It never falls
//! back to the environment: an unknown key is an error, checked at load time
//! by [`ExecCommandSpec::validate`](super::ExecCommandSpec::validate).

use std::borrow::Cow;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::serde_defaults;

/// Upper bound on the cells one matrix may produce.
///
/// A product of a few axes multiplies quickly, and every cell is a process spawn and a progress row:
/// a typo'd axis should fail the load, not fork thousands of children.
pub const MAX_MATRIX_CELLS: usize = 256;

/// One `include` / `exclude` entry: `key = value` pairs.
pub type MatrixEntry = IndexMap<String, String>;

/// A command's `strategy` table.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Strategy {
    /// The axes and their `include` / `exclude` adjustments.
    pub matrix: Matrix,
    /// Cells running at once; `1` runs them one after another. `None` runs
    /// every cell at once, up to the runner's `OPS_MAX_PARALLEL` cap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_parallel: Option<usize>,
    /// Stop the remaining cells after the first failing one (the default).
    /// `false` runs every cell and reports each failure.
    #[serde(default = "serde_defaults::default_true")]
    pub fail_fast: bool,
}

impl Strategy {
    /// A fail-fast strategy over `matrix` with no concurrency bound.
    ///
    /// Preferred over struct-literal syntax because [`Strategy`] is
    /// `#[non_exhaustive]`; adjust `max_parallel` / `fail_fast` directly.
    #[must_use]
    pub const fn new(matrix: Matrix) -> Self {
        Self {
            matrix,
            max_parallel: None,
            fail_fast: true,
        }
    }
}

/// `strategy.matrix`: named axes plus the GitHub Actions `include` /
/// `exclude` lists, which live beside the axes (so neither name can be an
/// axis).
///
/// Axis values are string lists. A table value (`crate = { from = ... }`) is
/// rejected today by the type, which keeps that spelling free for values
/// derived from the workspace.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[non_exhaustive]
pub struct Matrix {
    /// Entries merged into matching cells, or added as cells of their own.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include: Vec<MatrixEntry>,
    /// Entries whose pairs, all matching, drop a product cell.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<MatrixEntry>,
    /// The axes. Commands deserialize through `toml::Value`, whose tables are
    /// sorted, so the order is by axis name, not declaration order.
    #[serde(flatten)]
    pub axes: IndexMap<String, Vec<String>>,
}

impl Matrix {
    /// Whether the matrix declares nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.axes.is_empty() && self.include.is_empty() && self.exclude.is_empty()
    }

    /// Append `other` to `self`: each axis's values extend the axis of the
    /// same name (a new name becomes a new axis), and `include` / `exclude`
    /// entries extend the lists. Used to concatenate `[extend.<name>]`
    /// matrix entries across config layers.
    pub fn append(&mut self, other: &Self) {
        for (key, values) in &other.axes {
            self.axes
                .entry(key.clone())
                .or_default()
                .extend(values.iter().cloned());
        }
        self.include.extend(other.include.iter().cloned());
        self.exclude.extend(other.exclude.iter().cloned());
    }

    /// Expand the matrix into its cells, in a stable order: the product in
    /// axis-name order, then include-only cells in entry order.
    ///
    /// # Errors
    ///
    /// If an axis is empty or badly named, an `exclude` entry is empty or
    /// names a key that is not an axis, a key or value contains a control
    /// character, or the matrix yields no cells or more than
    /// [`MAX_MATRIX_CELLS`].
    pub fn cells(&self) -> Result<Vec<MatrixCell>, String> {
        let product = self.product()?;
        let mut cells: Vec<MatrixCell> = Vec::with_capacity(product.len());
        for cell in product {
            if !self.exclude.iter().any(|entry| cell.matches_all(entry)) {
                cells.push(cell);
            }
        }
        // `include` applies to the product's cells only, never to cells an
        // earlier include created (GitHub Actions semantics).
        let product_len = cells.len();
        for entry in &self.include {
            check_entry(entry, "include")?;
            let mut merged = false;
            for cell in cells.iter_mut().take(product_len) {
                if self.fits(cell, entry) {
                    for (key, value) in entry {
                        cell.set(key, value);
                    }
                    merged = true;
                }
            }
            if !merged {
                let cell = MatrixCell {
                    values: entry.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                };
                if !cells.contains(&cell) {
                    cells.push(cell);
                }
            }
        }
        if cells.is_empty() {
            return Err("matrix produces no cells (every combination is excluded)".to_string());
        }
        if cells.len() > MAX_MATRIX_CELLS {
            return Err(format!(
                "matrix produces {} cells, more than the {MAX_MATRIX_CELLS} allowed",
                cells.len()
            ));
        }
        Ok(cells)
    }

    /// The Cartesian product of the axes, with `exclude` entries checked for
    /// keys that name no axis (a typo would otherwise exclude nothing).
    fn product(&self) -> Result<Vec<MatrixCell>, String> {
        if self.axes.is_empty() {
            if self.include.is_empty() {
                return Err("matrix declares no axes and no `include` entries".to_string());
            }
            return Ok(Vec::new());
        }
        let mut total: usize = 1;
        for (key, values) in &self.axes {
            check_key(key)?;
            if values.is_empty() {
                return Err(format!("matrix axis `{key}` has no values"));
            }
            for (i, value) in values.iter().enumerate() {
                check_value(key, value)?;
                // Two equal values make two identical cells: one id, one
                // label, two runs — never what the list meant.
                if values.iter().take(i).any(|v| v == value) {
                    return Err(format!("matrix axis `{key}` lists {value:?} twice"));
                }
            }
            total = total
                .checked_mul(values.len())
                .filter(|n| *n <= MAX_MATRIX_CELLS)
                .ok_or_else(|| {
                    format!("matrix has more than the {MAX_MATRIX_CELLS} cells allowed")
                })?;
        }
        for entry in &self.exclude {
            if entry.is_empty() {
                return Err("an `exclude` entry is empty and would exclude every cell".to_string());
            }
            for key in entry.keys() {
                if !self.axes.contains_key(key) {
                    return Err(format!(
                        "`exclude` names `{key}`, which is not a matrix axis"
                    ));
                }
            }
        }
        let mut cells = vec![MatrixCell::default()];
        for (key, values) in &self.axes {
            let mut next = Vec::with_capacity(cells.len().saturating_mul(values.len()));
            for cell in &cells {
                for value in values {
                    let mut extended = cell.clone();
                    extended.values.push((key.clone(), value.clone()));
                    next.push(extended);
                }
            }
            cells = next;
        }
        Ok(cells)
    }

    /// An include entry fits a product cell when it agrees with every axis
    /// value it names; keys an earlier include added may be overwritten.
    fn fits(&self, cell: &MatrixCell, entry: &MatrixEntry) -> bool {
        entry
            .iter()
            .filter(|(key, _)| self.axes.contains_key(*key))
            .all(|(key, value)| cell.get(key) == Some(value.as_str()))
    }
}

/// One matrix cell: its `key = value` pairs, axes first in axis-name
/// order, then keys added by `include`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MatrixCell {
    pub values: Vec<(String, String)>,
}

impl MatrixCell {
    /// The value of `key` in this cell.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    fn set(&mut self, key: &str, value: &str) {
        if let Some(slot) = self.values.iter_mut().find(|(k, _)| k == key) {
            slot.1 = value.to_string();
        } else {
            self.values.push((key.to_string(), value.to_string()));
        }
    }

    fn matches_all(&self, entry: &MatrixEntry) -> bool {
        entry
            .iter()
            .all(|(key, value)| self.get(key) == Some(value.as_str()))
    }

    /// `key=value` pairs joined by `, ` — the cell's name in labels and
    /// failure messages.
    #[must_use]
    pub fn describe(&self) -> String {
        self.values
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The cell's step id / progress label: `name [key=value, ...]`.
    #[must_use]
    pub fn label(&self, name: &str) -> String {
        format!("{name} [{}]", self.describe())
    }
}

fn check_key(key: &str) -> Result<(), String> {
    if key.is_empty()
        || !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(format!(
            "matrix key {key:?} must be non-empty ASCII letters, digits, `_` or `-`"
        ));
    }
    Ok(())
}

fn check_value(key: &str, value: &str) -> Result<(), String> {
    if value.chars().any(char::is_control) {
        return Err(format!(
            "matrix value {value:?} of `{key}` contains a control character"
        ));
    }
    Ok(())
}

fn check_entry(entry: &MatrixEntry, list: &str) -> Result<(), String> {
    for (key, value) in entry {
        check_key(key).map_err(|e| format!("`{list}` entry: {e}"))?;
        check_value(key, value)?;
    }
    Ok(())
}

/// A `${matrix.<key>}` reference that could not be substituted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatrixRefError {
    /// `${matrix.<key>}` names a key the lookup does not know.
    Unknown(String),
    /// `${matrix.` with no closing `}`.
    Unterminated,
}

impl std::fmt::Display for MatrixRefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown(key) => write!(f, "${{matrix.{key}}}"),
            Self::Unterminated => f.write_str("an unterminated `${matrix.` reference"),
        }
    }
}

const REF_OPEN: &str = "${matrix.";

/// Replace every `${matrix.<key>}` in `template` with `lookup(key)`.
///
/// Borrows when `template` holds no reference. Passing a lookup that always
/// returns `None` turns this into a "does it reference the matrix?" check.
///
/// # Errors
///
/// [`MatrixRefError::Unknown`] for the first key `lookup` does not resolve,
/// [`MatrixRefError::Unterminated`] for a reference missing its `}`.
pub fn substitute<'a, 'v>(
    template: &'a str,
    lookup: impl Fn(&str) -> Option<&'v str>,
) -> Result<Cow<'a, str>, MatrixRefError> {
    if !template.contains(REF_OPEN) {
        return Ok(Cow::Borrowed(template));
    }
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some((before, after_open)) = rest.split_once(REF_OPEN) {
        out.push_str(before);
        let (key, after) = after_open
            .split_once('}')
            .ok_or(MatrixRefError::Unterminated)?;
        let value = lookup(key).ok_or_else(|| MatrixRefError::Unknown(key.to_string()))?;
        out.push_str(value);
        rest = after;
    }
    out.push_str(rest);
    Ok(Cow::Owned(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix(toml_src: &str) -> Matrix {
        toml::from_str(toml_src).expect("matrix must parse")
    }

    fn described(m: &Matrix) -> Vec<String> {
        m.cells()
            .expect("cells")
            .iter()
            .map(MatrixCell::describe)
            .collect()
    }

    #[test]
    fn axes_form_the_cartesian_product_ordered_by_axis_name() {
        // Declared `os` first; TOML tables are sorted, so `crate` leads.
        let m = matrix(
            r#"os = ["linux", "mac"]
crate = ["a", "b"]"#,
        );
        assert_eq!(
            described(&m),
            [
                "crate=a, os=linux",
                "crate=a, os=mac",
                "crate=b, os=linux",
                "crate=b, os=mac"
            ]
        );
    }

    #[test]
    fn exclude_drops_every_cell_matching_all_pairs() {
        let m = matrix(
            r#"os = ["linux", "mac"]
crate = ["a", "b"]
exclude = [{ os = "mac", crate = "b" }]"#,
        );
        assert_eq!(
            described(&m),
            ["crate=a, os=linux", "crate=a, os=mac", "crate=b, os=linux"]
        );
    }

    #[test]
    fn exclude_naming_no_axis_is_an_error() {
        let m = matrix(
            r#"crate = ["a"]
exclude = [{ crat = "a" }]"#,
        );
        let err = m.cells().unwrap_err();
        assert!(err.contains("`crat`"), "{err}");
    }

    /// The GitHub Actions documentation example for `include`: entries
    /// extend every product cell they do not contradict, later includes may
    /// overwrite keys earlier includes added, and an entry that fits no
    /// product cell becomes its own cell (and is never merged into another
    /// include-created cell).
    #[test]
    fn include_follows_github_actions_semantics() {
        let m = matrix(
            r#"fruit = ["apple", "pear"]
animal = ["cat", "dog"]
include = [
  { color = "green" },
  { color = "pink", animal = "cat" },
  { fruit = "apple", shape = "circle" },
  { fruit = "banana" },
  { fruit = "banana", animal = "cat" },
]"#,
        );
        assert_eq!(
            described(&m),
            [
                "animal=cat, fruit=apple, color=pink, shape=circle",
                "animal=cat, fruit=pear, color=pink",
                "animal=dog, fruit=apple, color=green, shape=circle",
                "animal=dog, fruit=pear, color=green",
                "fruit=banana",
                "animal=cat, fruit=banana",
            ]
        );
    }

    #[test]
    fn include_can_re_add_an_excluded_cell() {
        let m = matrix(
            r#"crate = ["a", "b"]
exclude = [{ crate = "b" }]
include = [{ crate = "b", features = "x" }]"#,
        );
        assert_eq!(described(&m), ["crate=a", "crate=b, features=x"]);
    }

    #[test]
    fn include_only_matrix_yields_one_cell_per_entry() {
        let m = matrix(r#"include = [{ crate = "a" }, { crate = "b" }]"#);
        assert_eq!(described(&m), ["crate=a", "crate=b"]);
    }

    #[test]
    fn empty_and_oversized_matrices_are_errors() {
        assert!(matrix("").cells().unwrap_err().contains("no axes"));
        assert!(matrix("crate = []")
            .cells()
            .unwrap_err()
            .contains("no values"));
        assert!(matrix(r#"crate = ["a", "a"]"#)
            .cells()
            .unwrap_err()
            .contains("twice"));
        assert!(matrix(
            r#"crate = ["a"]
exclude = [{ crate = "a" }]"#
        )
        .cells()
        .unwrap_err()
        .contains("no cells"));
        let values: Vec<String> = (0..17).map(|i| format!("\"v{i}\"")).collect();
        let list = values.join(", ");
        let big = matrix(&format!("a = [{list}]\nb = [{list}]"));
        assert!(big.cells().unwrap_err().contains("256"));
    }

    #[test]
    fn a_table_valued_axis_is_rejected_by_the_type() {
        let err = toml::from_str::<Matrix>(r#"crate = { from = "workspace-crates" }"#)
            .expect_err("table axis values are reserved");
        assert!(err.to_string().contains("sequence"), "{err}");
    }

    #[test]
    fn strategy_defaults_to_fail_fast_without_a_bound() {
        let s: Strategy = toml::from_str(r#"matrix = { crate = ["a"] }"#).unwrap();
        assert!(s.fail_fast);
        assert_eq!(s.max_parallel, None);
    }

    #[test]
    fn substitute_replaces_every_reference() {
        let cell = MatrixCell {
            values: vec![
                ("crate".into(), "core".into()),
                ("os".into(), "linux".into()),
            ],
        };
        let out = substitute("-p=${matrix.crate}/${matrix.os}/${matrix.crate}", |k| {
            cell.get(k)
        })
        .unwrap();
        assert_eq!(out, "-p=core/linux/core");
    }

    #[test]
    fn substitute_borrows_without_references_and_leaves_env_vars_alone() {
        let out = substitute("${HOME}/x", |_| None).unwrap();
        assert!(matches!(out, Cow::Borrowed("${HOME}/x")));
    }

    #[test]
    fn substitute_reports_unknown_and_unterminated_references() {
        assert_eq!(
            substitute("${matrix.crat}", |_| None),
            Err(MatrixRefError::Unknown("crat".into()))
        );
        assert_eq!(
            substitute("${matrix.crate", |_| Some("x")),
            Err(MatrixRefError::Unterminated)
        );
    }

    #[test]
    fn append_extends_axes_and_lists() {
        let mut base = matrix(r#"crate = ["a"]"#);
        base.append(&matrix(
            r#"crate = ["b"]
exclude = [{ crate = "a" }]"#,
        ));
        assert_eq!(base.axes["crate"], ["a", "b"]);
        assert_eq!(base.exclude.len(), 1);
    }
}
