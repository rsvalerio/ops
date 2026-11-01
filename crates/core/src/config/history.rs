//! The release each `.ops.toml` field arrived in, and the check built on it.
//!
//! CI installs the `ops` a project pins while a developer often runs a newer
//! one, so a field newer than the pin passes every local gate and fails every
//! `ops` step in CI. [`HISTORY`] records when each field became loadable, so
//! `ops config check` can name what an older `ops` would reject without
//! installing it.
//!
//! The check reads the raw `.ops.toml` and `.ops.d/*.toml`, not the loaded
//! [`super::Config`]: the load merges in defaults and materializes every
//! `clone`, which erases what the project itself wrote.
//!
//! **Adding a config field means adding it to [`HISTORY`]**, under the next
//! unreleased version: the next minor after the workspace version, since
//! `feat` is what cuts a minor release. A test reads the accepted fields from
//! serde and fails until the new one is listed.
//!
//! Rows are keyed by where a field means something, not by its literal key
//! path: `exec.args` is as old as `ops` and `clone.args` arrived in 0.86.0, so a field
//! that gained a new context, or changes behaviour there, gets its own row.
//! Field values are not tracked, nor is `[themes.<name>.error_block]`: it
//! ignores unknown keys, so no release rejects a field inside it.

use std::fmt;
use std::path::Path;
use std::str::FromStr;

use anyhow::Context;

use super::loader::{read_capped_toml_file, read_conf_d_files};

/// The mise configuration a project pins its tools in.
const MISE_TOML: &str = "mise.toml";

/// A released `ops` version, ordered the way versions are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct OpsVersion {
    major: u64,
    minor: u64,
    patch: u64,
}

impl OpsVersion {
    const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

/// Text that names no release: `latest`, a partial `0.85`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOpsVersionError(String);

impl fmt::Display for ParseOpsVersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` is not a release version such as 0.85.1", self.0)
    }
}

impl std::error::Error for ParseOpsVersionError {}

impl fmt::Display for OpsVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl FromStr for OpsVersion {
    type Err = ParseOpsVersionError;

    /// `1.2.3` or `v1.2.3`. A partial `1.2` or a `latest` names no release.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let invalid = || ParseOpsVersionError(text.to_owned());
        let mut parts = text
            .strip_prefix('v')
            .unwrap_or(text)
            .split('.')
            .map(str::parse);
        let mut next = || parts.next().and_then(Result::ok).ok_or_else(invalid);
        let version = Self::new(next()?, next()?, next()?);
        if parts.next().is_some() {
            return Err(invalid());
        }
        Ok(version)
    }
}

/// Every config field by the release that first accepted it, newest first.
/// See the module docs for the row keys and for how to add a field.
///
/// `commands` entries are keyed by their shape (`exec`, `composite`,
/// `clone`), `[themes.<name>]` under `theme`, `[extend.<name>]` under
/// `extend`, and a `strategy` table under `strategy`.
const HISTORY: &[(OpsVersion, &[&str])] = &[
    (
        OpsVersion::new(0, 91, 0),
        &["exec.after", "exec.wrap", "clone.after", "clone.wrap"],
    ),
    (OpsVersion::new(0, 88, 0), &["cargo.workspaces"]),
    (OpsVersion::new(0, 86, 0), &["clone.args"]),
    (
        OpsVersion::new(0, 85, 0),
        &["exec.raw", "composite.raw", "clone.raw"],
    ),
    (
        OpsVersion::new(0, 75, 0),
        &[
            "lint_actions",
            "lint_actions.allow",
            "cargo",
            "cargo.locked",
        ],
    ),
    (
        OpsVersion::new(0, 74, 0),
        &["foundation", "foundation.waivers"],
    ),
    (
        OpsVersion::new(0, 65, 0),
        &[
            "exec.strategy",
            "clone.strategy",
            "strategy.matrix",
            "strategy.max_parallel",
            "strategy.fail_fast",
            "extend.matrix",
        ],
    ),
    (
        OpsVersion::new(0, 62, 0),
        &["extend.help", "extend.category"],
    ),
    (
        OpsVersion::new(0, 61, 0),
        &[
            "backlog",
            "backlog.default_status",
            "backlog.statuses",
            "backlog.zero_padded_ids",
            "backlog.task_prefix",
            "backlog.backlog_directory",
        ],
    ),
    (
        OpsVersion::new(0, 60, 0),
        &[
            "clone.clone",
            "clone.help",
            "clone.aliases",
            "clone.category",
            "clone.env",
            "clone.cwd",
            "clone.timeout_secs",
            "clone.exclusive",
            "extend.args",
        ],
    ),
    (OpsVersion::new(0, 57, 0), &["extend", "extend.commands"]),
    (OpsVersion::new(0, 56, 0), &["exec.exclusive"]),
    (OpsVersion::new(0, 47, 0), &["data.provider_budget_secs"]),
    (
        OpsVersion::new(0, 34, 0),
        &[
            "theme.report",
            "report.icon_ok",
            "report.icon_info",
            "report.icon_warning",
            "report.icon_error",
            "report.color_ok",
            "report.color_info",
            "report.color_warning",
            "report.color_error",
            "report.title_color",
        ],
    ),
    (
        OpsVersion::new(0, 17, 0),
        &[
            "theme.plan_header_prefix",
            "theme.header_color",
            "theme.label_color",
            "theme.separator_color",
            "theme.duration_color",
            "theme.summary_color",
            "theme.layout_kind",
        ],
    ),
    (OpsVersion::new(0, 13, 0), &["about", "about.fields"]),
    (
        OpsVersion::new(0, 12, 0),
        &[
            "output.category_order",
            "exec.aliases",
            "exec.category",
            "composite.aliases",
            "composite.category",
        ],
    ),
    (OpsVersion::new(0, 9, 0), &["output.stderr_tail_lines"]),
    (
        OpsVersion::new(0, 6, 0),
        &["exec.help", "composite.help", "theme.left_pad"],
    ),
    (
        OpsVersion::new(0, 1, 0),
        &[
            "output",
            "commands",
            "data",
            "themes",
            "extensions",
            "stack",
            "output.theme",
            "output.columns",
            "output.show_error_detail",
            "data.path",
            "extensions.enabled",
            "exec.program",
            "exec.args",
            "exec.env",
            "exec.cwd",
            "exec.timeout_secs",
            "composite.commands",
            "composite.parallel",
            "composite.fail_fast",
            "theme.icon_pending",
            "theme.icon_running",
            "theme.icon_succeeded",
            "theme.icon_failed",
            "theme.icon_skipped",
            "theme.separator_char",
            "theme.step_indent",
            "theme.running_template",
            "theme.tick_chars",
            "theme.running_template_overhead",
            "theme.plan_header_style",
            "theme.summary_prefix",
            "theme.summary_separator",
            "theme.error_block",
            "theme.description",
        ],
    ),
];

/// The oldest release: what a config using nothing newer needs.
const FIRST_RELEASE: OpsVersion = OpsVersion::new(0, 1, 0);

fn since(row: &str) -> Option<OpsVersion> {
    HISTORY
        .iter()
        .find(|(_, rows)| rows.contains(&row))
        .map(|(version, _)| *version)
}

/// One field a project's config sets, and the release it needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsedField {
    /// The file setting it, relative to the workspace root.
    pub file: String,
    /// Its key path, e.g. `commands.build.raw`.
    pub field: String,
    /// The first release accepting it.
    pub since: OpsVersion,
}

/// The fields set by the workspace's `.ops.toml` and `.ops.d/*.toml`, in
/// those files' load order. A key [`HISTORY`] does not know is skipped: this
/// binary's own load is what rejects it.
///
/// # Errors
///
/// A config file cannot be read or is not valid TOML. Symlinks are refused,
/// as they are on the load path.
pub fn fields_used(workspace_root: &Path) -> anyhow::Result<Vec<UsedField>> {
    let mut files = vec![workspace_root.join(".ops.toml")];
    files.extend(read_conf_d_files(&workspace_root.join(".ops.d"))?.unwrap_or_default());
    let mut out = Vec::new();
    for path in files {
        let Some(text) = read_capped_toml_file(&path)? else {
            continue;
        };
        let table: toml::Table = text
            .parse()
            .with_context(|| format!("failed to parse config file: {:?}", path.display()))?;
        let file = path
            .strip_prefix(workspace_root)
            .unwrap_or(&path)
            .display()
            .to_string();
        walk(&table, "", "", &file, &mut out);
    }
    Ok(out)
}

/// The release a config using `fields` needs at the least.
#[must_use]
pub fn minimum_version(fields: &[UsedField]) -> OpsVersion {
    fields
        .iter()
        .map(|used| used.since)
        .max()
        .unwrap_or(FIRST_RELEASE)
}

/// The `ops` version `mise.toml` pins under `[tools]`, if it pins one.
///
/// # Errors
///
/// `mise.toml` cannot be read or parsed, or its pin names no exact release
/// (`latest`, a partial `0.85`): there is no version to check against.
pub fn mise_ops_pin(workspace_root: &Path) -> anyhow::Result<Option<OpsVersion>> {
    let Some(text) = read_capped_toml_file(&workspace_root.join(MISE_TOML))? else {
        return Ok(None);
    };
    let table: toml::Table = text
        .parse()
        .with_context(|| format!("failed to parse {MISE_TOML}"))?;
    let Some(pin) = table.get("tools").and_then(|tools| tools.get("ops")) else {
        return Ok(None);
    };
    pin.as_str()
        .or_else(|| pin.get("version")?.as_str())
        .and_then(|version| version.parse().ok())
        .map(Some)
        .with_context(|| {
            format!("{MISE_TOML} pins ops to {pin}, not a release; pass --as <version>")
        })
}

/// Record every field of `table`, descending where [`HISTORY`] has rows for
/// a nested table. `row` is the key prefix in [`HISTORY`], `path` the real
/// one; free-form maps such as `env` have no rows and are not descended.
fn walk(table: &toml::Table, row: &str, path: &str, file: &str, out: &mut Vec<UsedField>) {
    for (key, value) in table {
        // The serde alias of `aliases`, on every command shape.
        let canonical = if key == "alias" { "aliases" } else { key };
        let child_row = join(row, canonical);
        let Some(field_since) = since(&child_row) else {
            continue;
        };
        let child_path = join(path, key);
        let field = if row == "clone" && key != "clone" {
            format!("{child_path} (beside clone)")
        } else {
            child_path.clone()
        };
        out.push(UsedField {
            file: file.to_owned(),
            field,
            since: field_since,
        });
        let Some(nested) = value.as_table() else {
            continue;
        };
        let mut entries = |entry_row: fn(&toml::Table) -> &str| {
            for (name, entry) in nested {
                if let Some(entry) = entry.as_table() {
                    let entry_path = join(&child_path, name);
                    walk(entry, entry_row(entry), &entry_path, file, out);
                }
            }
        };
        match child_row.as_str() {
            "commands" => entries(command_shape),
            "themes" => entries(|_| "theme"),
            "extend" => entries(|_| "extend"),
            "exec.strategy" | "clone.strategy" => walk(nested, "strategy", &child_path, file, out),
            "theme.report" => walk(nested, "report", &child_path, file, out),
            "output" | "data" | "extensions" | "about" | "backlog" | "foundation"
            | "lint_actions" | "cargo" => walk(nested, &child_row, &child_path, file, out),
            _ => {}
        }
    }
}

/// The [`HISTORY`] prefix of a `[commands.<name>]` table, picked the way
/// `CommandSpec`'s `Deserialize` picks the variant.
fn command_shape(table: &toml::Table) -> &'static str {
    let composite_only = ["parallel", "fail_fast"]
        .iter()
        .any(|key| table.contains_key(*key));
    if table.contains_key("clone") {
        "clone"
    } else if table.contains_key("commands") || (!table.contains_key("program") && composite_only) {
        "composite"
    } else {
        "exec"
    }
}

fn join(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_owned()
    } else {
        format!("{prefix}.{key}")
    }
}

#[cfg(test)]
mod tests {
    use super::super::commands::{CloneCommandSpec, CompositeCommandSpec, ExecCommandSpec};
    use super::super::extend::ExtendEntry;
    use super::super::sections::{
        AboutConfig, BacklogSection, CargoSection, DataConfig, ExtensionConfig, FoundationSection,
        LintActionsSection, OutputConfig,
    };
    use super::super::strategy::Strategy;
    use super::super::theme_types::{ReportTheme, ThemeConfig};
    use super::super::Config;
    use super::*;

    fn version(text: &str) -> OpsVersion {
        text.parse().expect("valid version")
    }

    fn project(ops_toml: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".ops.toml"), ops_toml).expect("write");
        dir
    }

    fn used(ops_toml: &str) -> Vec<(String, String)> {
        let dir = project(ops_toml);
        fields_used(dir.path())
            .expect("fields")
            .into_iter()
            .map(|f| (f.field, f.since.to_string()))
            .collect()
    }

    fn has(fields: &[(String, String)], field: &str, since: &str) -> bool {
        fields.iter().any(|(f, s)| f == field && s == since)
    }

    /// The fields `T` accepts, read from the list serde's
    /// `deny_unknown_fields` error names.
    fn accepted<T: serde::de::DeserializeOwned>() -> Vec<String> {
        let Err(err) = toml::from_str::<T>("zzz_not_a_field = 1") else {
            panic!("an unknown field must be refused");
        };
        err.message()
            .split('`')
            .skip(3)
            .step_by(2)
            .map(str::to_owned)
            .collect()
    }

    fn assert_listed<T: serde::de::DeserializeOwned>(prefix: &str) {
        let fields = accepted::<T>();
        assert!(!fields.is_empty(), "{prefix}: no fields read from serde");
        for field in fields {
            if field == "alias" {
                continue; // `walk` folds the serde alias into `aliases`.
            }
            let row = join(prefix, &field);
            assert!(
                since(&row).is_some(),
                "config field `{row}` is missing from config::history::HISTORY: add it under \
                 the next unreleased version (the next minor after the workspace version)"
            );
        }
    }

    /// A new config field fails here until [`HISTORY`] records its release.
    #[test]
    fn every_config_field_is_in_the_history() {
        assert_listed::<Config>("");
        assert_listed::<OutputConfig>("output");
        assert_listed::<DataConfig>("data");
        assert_listed::<ExtensionConfig>("extensions");
        assert_listed::<AboutConfig>("about");
        assert_listed::<BacklogSection>("backlog");
        assert_listed::<FoundationSection>("foundation");
        assert_listed::<LintActionsSection>("lint_actions");
        assert_listed::<CargoSection>("cargo");
        assert_listed::<ExecCommandSpec>("exec");
        assert_listed::<CompositeCommandSpec>("composite");
        assert_listed::<CloneCommandSpec>("clone");
        assert_listed::<ExtendEntry>("extend");
        assert_listed::<Strategy>("strategy");
        assert_listed::<ThemeConfig>("theme");
        assert_listed::<ReportTheme>("report");
    }

    /// A row may name the next unreleased version, never one past it.
    #[test]
    fn history_stops_at_the_next_release_and_is_newest_first() {
        let current = version(env!("CARGO_PKG_VERSION"));
        let next = OpsVersion::new(current.major, current.minor.saturating_add(1), 0);
        assert!(HISTORY.iter().all(|(version, _)| *version <= next));
        assert!(HISTORY.windows(2).all(|pair| pair[0].0 > pair[1].0));
    }

    #[test]
    fn args_beside_clone_needs_0_86_but_args_on_an_exec_does_not() {
        let fields = used(
            "[commands.a]\nprogram = \"x\"\nargs = [\"y\"]\n\
             [commands.b]\nclone = \"a\"\nargs = [\"z\"]\n",
        );
        assert!(has(&fields, "commands.a.args", "0.1.0"), "{fields:?}");
        assert!(
            has(&fields, "commands.b.args (beside clone)", "0.86.0"),
            "{fields:?}"
        );
    }

    #[test]
    fn after_and_wrap_need_the_release_after_0_90() {
        let fields = used(
            "[commands.a]\nprogram = \"x\"\nafter = [\"b\"]\nwrap = [\"w\"]\n\
             [commands.c]\nclone = \"a\"\nwrap = []\n",
        );
        assert!(has(&fields, "commands.a.after", "0.91.0"), "{fields:?}");
        assert!(has(&fields, "commands.a.wrap", "0.91.0"), "{fields:?}");
        assert!(
            has(&fields, "commands.c.wrap (beside clone)", "0.91.0"),
            "{fields:?}"
        );
    }

    #[test]
    fn minimum_is_the_newest_field_used_and_an_empty_config_needs_the_first_release() {
        let dir = project("[commands.a]\nprogram = \"x\"\nraw = true\n");
        let fields = fields_used(dir.path()).expect("fields");
        assert_eq!(minimum_version(&fields), version("0.85.0"));
        assert_eq!(minimum_version(&[]), FIRST_RELEASE);
    }

    #[test]
    fn every_shape_and_nested_table_is_walked() {
        let fields = used(
            "[commands.c]\ncommands = [\"a\"]\nraw = true\n\
             [commands.p]\nparallel = true\nalias = [\"q\"]\n\
             [commands.m]\nprogram = \"x\"\nalias = [\"n\"]\n\
             [commands.m.strategy]\nmax_parallel = 2\n\
             [commands.m.strategy.matrix]\nos = [\"a\"]\n\
             [commands.m.env]\nraw = \"not a field\"\n\
             [extend.verify]\nhelp = \"h\"\n\
             [cargo]\nworkspaces = [\"a\"]\n\
             [themes.t.report]\nicon_ok = \"y\"\n\
             [themes.t.error_block]\ncolor = \"red\"\n",
        );
        for (field, since) in [
            ("commands.c.raw", "0.85.0"),
            ("commands.p.alias", "0.12.0"),
            ("commands.m.alias", "0.12.0"),
            ("commands.m.strategy", "0.65.0"),
            ("commands.m.strategy.max_parallel", "0.65.0"),
            ("extend.verify.help", "0.62.0"),
            ("cargo.workspaces", "0.88.0"),
            ("themes.t.report.icon_ok", "0.34.0"),
            ("themes.t.error_block", "0.1.0"),
        ] {
            assert!(has(&fields, field, since), "{field}: {fields:?}");
        }
        assert!(
            !fields.iter().any(|(f, _)| f.contains(".env.")),
            "env is a free-form map: {fields:?}"
        );
    }

    #[test]
    fn conf_d_fragments_count_and_name_their_file() {
        let dir = project("");
        let conf_d = dir.path().join(".ops.d");
        std::fs::create_dir(&conf_d).expect("mkdir");
        std::fs::write(conf_d.join("x.toml"), "[cargo]\nworkspaces = [\"a\"]\n").expect("write");
        let fields = fields_used(dir.path()).expect("fields");
        let found = fields
            .iter()
            .find(|f| f.field == "cargo.workspaces")
            .expect("fragment field");
        assert_eq!(
            found.file,
            Path::new(".ops.d").join("x.toml").display().to_string()
        );
    }

    #[test]
    fn no_config_uses_no_fields_and_a_broken_one_is_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(fields_used(dir.path()).expect("fields").is_empty());
        let broken = project("not = = toml");
        assert!(fields_used(broken.path()).is_err());
    }

    #[test]
    fn mise_pin_reads_a_string_or_a_table_and_absence_is_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mise = dir.path().join(MISE_TOML);
        assert_eq!(mise_ops_pin(dir.path()).expect("no file"), None);
        std::fs::write(&mise, "[tools]\nrust = \"1.0.0\"\n").expect("write");
        assert_eq!(mise_ops_pin(dir.path()).expect("no pin"), None);
        std::fs::write(&mise, "[tools]\nops = \"0.85.1\"\n").expect("write");
        assert_eq!(
            mise_ops_pin(dir.path()).expect("string"),
            Some(version("0.85.1"))
        );
        std::fs::write(&mise, "[tools]\nops = { version = \"0.79.3\" }\n").expect("write");
        assert_eq!(
            mise_ops_pin(dir.path()).expect("table"),
            Some(version("0.79.3"))
        );
    }

    #[test]
    fn a_pin_naming_no_release_is_an_error_not_a_pass() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(MISE_TOML), "[tools]\nops = \"latest\"\n").expect("write");
        let err = mise_ops_pin(dir.path()).expect_err("latest names no release");
        assert!(err.to_string().contains("--as"), "{err}");
    }

    #[test]
    fn versions_parse_with_an_optional_v_and_order_numerically() {
        assert_eq!(version("v0.85.1"), version("0.85.1"));
        assert_eq!(version("0.85.1").to_string(), "0.85.1");
        assert!(version("0.100.0") > version("0.99.9"));
        for bad in ["latest", "0.85", "0.85.1.2", "", "0.x.1"] {
            assert!(bad.parse::<OpsVersion>().is_err(), "{bad}");
        }
    }
}
