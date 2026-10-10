//! Stack-agnostic `about dependencies` subpage: per-unit dependency tree.
//!
//! Calls the `project_dependencies` data provider registered by the active stack.

use std::io::{IsTerminal, Write};

use ops_core::project_identity::ProjectDependencies;
use ops_core::style::{cyan, dim};
use ops_extension::DataRegistry;

use crate::providers::{load_or_default, warm_providers};
use crate::text_util::tty_style;

/// Registry key of the `project_dependencies` provider that supplies the
/// per-unit dependency tree rendered on this subpage.
///
/// When no stack registers it, the page falls back to "No dependency data
/// available."
pub const PROJECT_DEPENDENCIES_PROVIDER: &str = "project_dependencies";

/// # Errors
///
/// If the current directory cannot be determined, a required data provider
/// fails, or writing the rendered output fails.
pub fn run_about_deps(data_registry: &DataRegistry) -> anyhow::Result<()> {
    let is_tty = std::io::stdout().is_terminal();
    run_about_deps_with(data_registry, &mut std::io::stdout(), is_tty)
}

/// `is_tty` reflects the `writer` the caller hands in.
/// See [`crate::units::run_about_units_with`] for the rationale.
///
/// # Errors
///
/// If the current directory cannot be determined, a required data provider
/// fails, or writing the rendered output fails.
pub fn run_about_deps_with(
    data_registry: &DataRegistry,
    writer: &mut dyn Write,
    is_tty: bool,
) -> anyhow::Result<()> {
    let mut ctx = crate::providers::subpage_context("deps")?;

    warm_providers(&mut ctx, data_registry, &["sqlite", "metadata"], "deps");

    let deps: ProjectDependencies =
        load_or_default(&mut ctx, data_registry, PROJECT_DEPENDENCIES_PROVIDER)?;

    let lines = format_dependencies_section(&deps, is_tty);
    if lines.is_empty() {
        writeln!(writer, "No dependency data available.")?;
        return Ok(());
    }
    writeln!(writer, "{}", lines.join("\n"))?;
    Ok(())
}

/// Version of the `ops about dependencies --json` document shape.
pub const DEPS_JSON_SCHEMA_VERSION: u32 = 1;

/// One direct dependency in the `ops about dependencies --json` document.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct DependencyRecord {
    pub name: String,
    /// The version requirement as declared (e.g. `^1.0`).
    pub requirement: String,
}

/// One unit's direct dependencies.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct UnitDepsRecord {
    pub name: String,
    pub dependencies: Vec<DependencyRecord>,
}

/// The `ops about dependencies --json` document.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DepsDocument {
    pub schema_version: u32,
    pub kind: &'static str,
    pub units: Vec<UnitDepsRecord>,
}

/// Build the dependency document: every unit (including ones with no
/// dependencies, so absence is explicit), sorted by name, each unit's
/// dependencies sorted by name.
#[must_use]
pub fn deps_json(deps: &ProjectDependencies) -> DepsDocument {
    let mut units: Vec<UnitDepsRecord> = deps
        .units
        .iter()
        .map(|u| {
            let mut dependencies: Vec<DependencyRecord> = u
                .deps
                .iter()
                .map(|(name, requirement)| DependencyRecord {
                    name: name.clone(),
                    requirement: requirement.clone(),
                })
                .collect();
            dependencies.sort_by(|a, b| a.name.cmp(&b.name));
            UnitDepsRecord {
                name: u.unit_name.clone(),
                dependencies,
            }
        })
        .collect();
    units.sort_by(|a, b| a.name.cmp(&b.name));
    DepsDocument {
        schema_version: DEPS_JSON_SCHEMA_VERSION,
        kind: "about-dependencies",
        units,
    }
}

/// `ops about dependencies --json`.
///
/// # Errors
///
/// If the current directory cannot be determined, the provider fails, or
/// writing fails.
pub fn run_about_deps_json(data_registry: &DataRegistry) -> anyhow::Result<()> {
    run_about_deps_json_with(data_registry, &mut std::io::stdout())
}

/// [`run_about_deps_json`] against an explicit writer.
///
/// # Errors
///
/// If the current directory cannot be determined, the provider fails, or
/// writing fails.
pub fn run_about_deps_json_with(
    data_registry: &DataRegistry,
    writer: &mut dyn Write,
) -> anyhow::Result<()> {
    let mut ctx = crate::providers::subpage_context("deps")?;
    warm_providers(&mut ctx, data_registry, &["sqlite", "metadata"], "deps");
    let deps: ProjectDependencies =
        load_or_default(&mut ctx, data_registry, PROJECT_DEPENDENCIES_PROVIDER)?;
    crate::write_json_document(writer, &deps_json(&deps))
}

/// Registry key of the provider answering `ops about dependencies
/// --duplicates`: crates locked at two or more distinct versions.
pub const PROJECT_DUPLICATES_PROVIDER: &str = "project_duplicate_dependencies";

/// Registry key of the provider answering `ops about dependencies
/// --duplicates --include-dev`: as [`PROJECT_DUPLICATES_PROVIDER`], but
/// dev-dependency edges count too, so dev-only duplicates are listed.
pub const PROJECT_DUPLICATES_WITH_DEV_PROVIDER: &str = "project_duplicate_dependencies_with_dev";

/// Version of the `ops about dependencies --duplicates --json` document.
pub const DUPLICATES_JSON_SCHEMA_VERSION: u32 = 1;

/// Crates the build links at more than one distinct version.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DuplicateReport {
    pub crates: Vec<DuplicateCrate>,
}

/// One duplicated crate.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DuplicateCrate {
    pub name: String,
    /// Every distinct locked version, ascending.
    pub versions: Vec<String>,
    /// Each version below the newest, with what pulls it in.
    pub older: Vec<OlderVersion>,
}

/// A non-newest version of a duplicated crate.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OlderVersion {
    pub version: String,
    /// The workspace's direct dependencies whose (non-dev) dependency tree
    /// contains this version.
    pub pulled_by: Vec<PullingDependency>,
}

/// A direct workspace dependency that pulls an older version in.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullingDependency {
    pub name: String,
    pub version: String,
    /// Whether a semver-compatible update of this dependency (`cargo update
    /// --dry-run -p`, which never writes `Cargo.lock`) drops the older
    /// version. `None` when the dry run could not be performed.
    pub update_removes_duplicate: Option<bool>,
}

/// The `--duplicates --json` document.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicatesDocument<'a> {
    pub schema_version: u32,
    pub kind: &'static str,
    pub crates: &'a [DuplicateCrate],
}

/// Wrap `report` in the versioned `--duplicates --json` envelope.
#[must_use]
pub const fn duplicates_json(report: &DuplicateReport) -> DuplicatesDocument<'_> {
    DuplicatesDocument {
        schema_version: DUPLICATES_JSON_SCHEMA_VERSION,
        kind: "about-dependency-duplicates",
        crates: report.crates.as_slice(),
    }
}

/// Render the duplicates as plain text lines; empty report → one line
/// saying so.
#[must_use]
pub fn format_duplicates_section(report: &DuplicateReport, is_tty: bool) -> Vec<String> {
    if report.crates.is_empty() {
        return vec!["No duplicate dependency versions.".to_string()];
    }
    let mut lines = vec![String::new(), "  DUPLICATE DEPENDENCIES".to_string()];
    for dup in &report.crates {
        lines.push(String::new());
        lines.push(format!(
            "  {}  {}",
            tty_style(&dup.name, cyan, is_tty),
            dup.versions.join(", ")
        ));
        for older in &dup.older {
            if older.pulled_by.is_empty() {
                lines.push(format!("    {} pulled by: unknown", older.version));
            }
            for puller in &older.pulled_by {
                let verdict = match puller.update_removes_duplicate {
                    Some(true) => "update removes it",
                    Some(false) => "update does not remove it",
                    None => "update check failed",
                };
                lines.push(format!(
                    "    {} pulled by {} {} \u{2014} {}",
                    older.version,
                    puller.name,
                    puller.version,
                    tty_style(verdict, dim, is_tty)
                ));
            }
        }
    }
    lines
}

/// Context argument carrying `--duplicates --target` to the provider.
///
/// Read through [`ops_extension::Context::arg`]: the requested triples
/// joined with `,`, or [`DUPLICATES_ALL_TARGETS`]. Absent means the host
/// target.
pub const DUPLICATES_TARGET_ARG: &str = "target";

/// `--target` value that disables the platform filter: every edge counts,
/// whatever target it is gated on.
pub const DUPLICATES_ALL_TARGETS: &str = "all";

/// What `--duplicates` was asked for.
#[derive(Debug, Clone, Copy, Default)]
pub struct DuplicatesOptions<'a> {
    /// Emit the versioned JSON document instead of text.
    pub json: bool,
    /// Follow dev-dependency edges too.
    pub include_dev: bool,
    /// `--target` values; empty means the host target.
    pub targets: &'a [String],
}

/// `ops about dependencies --duplicates [--json] [--include-dev] [--target <triple>...]`.
///
/// `include_dev` also follows dev-dependency edges, so duplicates reachable
/// only through dev-dependencies are listed too. `targets` restricts the
/// graph to edges active on any of those triples (`all` for no filter);
/// empty means the host target.
///
/// # Errors
///
/// If the current directory cannot be determined, the provider fails, or
/// writing fails.
pub fn run_about_duplicates(
    data_registry: &DataRegistry,
    options: DuplicatesOptions<'_>,
) -> anyhow::Result<()> {
    let is_tty = std::io::stdout().is_terminal();
    run_about_duplicates_with(data_registry, &mut std::io::stdout(), is_tty, options)
}

/// [`run_about_duplicates`] against an explicit writer.
///
/// # Errors
///
/// If the current directory cannot be determined, the provider fails, or
/// writing fails.
pub fn run_about_duplicates_with(
    data_registry: &DataRegistry,
    writer: &mut dyn Write,
    is_tty: bool,
    options: DuplicatesOptions<'_>,
) -> anyhow::Result<()> {
    let DuplicatesOptions {
        json,
        include_dev,
        targets,
    } = options;
    let mut ctx = crate::providers::subpage_context("deps")?;
    if !targets.is_empty() {
        ctx = ctx.with_arg(DUPLICATES_TARGET_ARG, targets.join(","));
    }
    warm_providers(&mut ctx, data_registry, &["metadata"], "deps");
    let provider = if include_dev {
        PROJECT_DUPLICATES_WITH_DEV_PROVIDER
    } else {
        PROJECT_DUPLICATES_PROVIDER
    };
    let report: DuplicateReport = load_or_default(&mut ctx, data_registry, provider)?;
    if json {
        return crate::write_json_document(writer, &duplicates_json(&report));
    }
    writeln!(
        writer,
        "{}",
        format_duplicates_section(&report, is_tty).join("\n")
    )?;
    Ok(())
}

/// Formats the `DEPENDENCIES` section lines for the given dependency report.
///
/// Units with no dependencies are skipped; returns an empty vector when no
/// unit has dependencies.
pub fn format_dependencies_section(deps: &ProjectDependencies, is_tty: bool) -> Vec<String> {
    let mut units: Vec<&ops_core::project_identity::UnitDeps> =
        deps.units.iter().filter(|u| !u.deps.is_empty()).collect();
    if units.is_empty() {
        return vec![];
    }
    units.sort_by(|a, b| a.unit_name.cmp(&b.unit_name));

    let mut lines = vec![String::new(), "  DEPENDENCIES".to_string()];

    for unit in units {
        lines.push(String::new());
        lines.push(format!("  {}", tty_style(&unit.unit_name, cyan, is_tty)));

        // Use `split_last` so the connector choice never
        // depends on `len() - 1` over a possibly-zero-length slice. The
        // outer filter already rejects empty deps today, but a future
        // refactor that adds another filter and lets an empty slice through
        // would otherwise panic on `0usize - 1`.
        let Some((last, rest)) = unit.deps.split_last() else {
            continue;
        };
        for (name, version) in rest {
            lines.push(format!(
                "  {}",
                tty_style(
                    &format!("\u{251c}\u{2500}\u{2500} {name} {version}"),
                    dim,
                    is_tty,
                )
            ));
        }
        let (last_name, last_version) = last;
        lines.push(format!(
            "  {}",
            tty_style(
                &format!("\u{2514}\u{2500}\u{2500} {last_name} {last_version}"),
                dim,
                is_tty,
            )
        ));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::project_identity::UnitDeps;

    /// Same gap as the other runners — the empty-state
    /// string had no assertion behind it.
    #[test]
    fn run_about_deps_with_reports_no_data_for_an_empty_registry() {
        let registry = DataRegistry::new();
        let mut out: Vec<u8> = Vec::new();
        run_about_deps_with(&registry, &mut out, false).expect("runner must succeed");
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "No dependency data available.\n"
        );
    }

    /// Pins the `ops about dependencies --json` shape.
    #[test]
    fn deps_json_pins_the_document_shape() {
        let deps = ProjectDependencies::new(vec![
            UnitDeps::new(
                "ops-core",
                vec![
                    ("serde".to_string(), "^1.0".to_string()),
                    ("anyhow".to_string(), "^1.0".to_string()),
                ],
            ),
            UnitDeps::new("ops-cli", vec![]),
        ]);
        let text = serde_json::to_string(&deps_json(&deps)).expect("serialize");
        assert_eq!(
            text,
            "{\"schemaVersion\":1,\"kind\":\"about-dependencies\",\"units\":[\
             {\"name\":\"ops-cli\",\"dependencies\":[]},\
             {\"name\":\"ops-core\",\"dependencies\":[\
             {\"name\":\"anyhow\",\"requirement\":\"^1.0\"},\
             {\"name\":\"serde\",\"requirement\":\"^1.0\"}]}]}"
        );
    }

    #[test]
    fn run_about_deps_json_with_empty_registry_emits_empty_units() {
        let registry = DataRegistry::new();
        let mut out: Vec<u8> = Vec::new();
        run_about_deps_json_with(&registry, &mut out).expect("runner must succeed");
        let value: serde_json::Value = serde_json::from_slice(&out).expect("valid json");
        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["units"], serde_json::json!([]));
    }

    fn sample_duplicates() -> DuplicateReport {
        DuplicateReport {
            crates: vec![DuplicateCrate {
                name: "crossterm".to_string(),
                versions: vec!["0.28.1".to_string(), "0.29.0".to_string()],
                older: vec![OlderVersion {
                    version: "0.28.1".to_string(),
                    pulled_by: vec![PullingDependency {
                        name: "comfy-table".to_string(),
                        version: "7.1.4".to_string(),
                        update_removes_duplicate: Some(true),
                    }],
                }],
            }],
        }
    }

    /// Pins the `--duplicates --json` shape.
    #[test]
    fn duplicates_json_pins_the_document_shape() {
        let report = sample_duplicates();
        let text = serde_json::to_string(&duplicates_json(&report)).expect("serialize");
        assert_eq!(
            text,
            "{\"schemaVersion\":1,\"kind\":\"about-dependency-duplicates\",\"crates\":[\
             {\"name\":\"crossterm\",\"versions\":[\"0.28.1\",\"0.29.0\"],\"older\":[\
             {\"version\":\"0.28.1\",\"pulledBy\":[{\"name\":\"comfy-table\",\
             \"version\":\"7.1.4\",\"updateRemovesDuplicate\":true}]}]}]}"
        );
    }

    #[test]
    fn format_duplicates_section_names_puller_and_verdict() {
        let text = format_duplicates_section(&sample_duplicates(), false).join("\n");
        assert!(text.contains("crossterm  0.28.1, 0.29.0"), "{text}");
        assert!(
            text.contains("0.28.1 pulled by comfy-table 7.1.4 \u{2014} update removes it"),
            "{text}"
        );
        assert_eq!(
            format_duplicates_section(&DuplicateReport::default(), false),
            ["No duplicate dependency versions."]
        );
    }

    #[test]
    fn run_about_duplicates_with_empty_registry_reports_none() {
        let registry = DataRegistry::new();
        let mut out: Vec<u8> = Vec::new();
        run_about_duplicates_with(
            &registry,
            &mut out,
            false,
            DuplicatesOptions {
                json: true,
                ..DuplicatesOptions::default()
            },
        )
        .expect("runner");
        let value: serde_json::Value = serde_json::from_slice(&out).expect("json");
        assert_eq!(value["crates"], serde_json::json!([]));
    }

    #[test]
    fn format_dependencies_section_empty() {
        assert!(format_dependencies_section(&ProjectDependencies::default(), false).is_empty());
    }

    #[test]
    fn format_dependencies_section_renders_tree() {
        let deps = ProjectDependencies::new(vec![
            UnitDeps::new(
                "ops-core",
                vec![
                    ("anyhow".to_string(), "^1.0".to_string()),
                    ("serde".to_string(), "^1.0".to_string()),
                ],
            ),
            UnitDeps::new("ops-cli", vec![("clap".to_string(), "^4.0".to_string())]),
        ]);
        let out = format_dependencies_section(&deps, false).join("\n");
        assert!(out.contains("DEPENDENCIES"));
        assert!(out.contains("ops-cli"));
        assert!(out.contains("ops-core"));
        assert!(out.contains("\u{251c}\u{2500}\u{2500} anyhow"));
        assert!(out.contains("\u{2514}\u{2500}\u{2500} serde"));
        // Sorted alphabetically
        assert!(out.find("ops-cli").unwrap() < out.find("ops-core").unwrap());
    }

    /// When the caller declares the writer is not a TTY,
    /// the rendered output must not contain ANSI escape bytes — even if the
    /// process's stdout happens to be a real terminal at test time.
    #[test]
    fn format_dependencies_section_emits_no_ansi_when_is_tty_false() {
        let deps = ProjectDependencies::new(vec![UnitDeps::new(
            "core",
            vec![("anyhow".to_string(), "^1.0".to_string())],
        )]);
        let out = format_dependencies_section(&deps, false).join("\n");
        assert!(
            !out.contains('\x1b'),
            "non-TTY writer must receive plain text: {out:?}"
        );
    }

    /// Pass a unit with empty deps directly and verify
    /// no panic. The outer filter currently skips it, but the inner loop
    /// must be safe for an empty slice on its own merits.
    #[test]
    fn format_dependencies_section_unit_with_empty_deps_does_not_panic() {
        let deps = ProjectDependencies::new(vec![UnitDeps::new("lonely", vec![])]);
        let out = format_dependencies_section(&deps, false);
        assert!(out.is_empty(), "expected empty output, got: {out:?}");
    }

    #[test]
    fn format_dependencies_section_skips_empty_deps() {
        let deps = ProjectDependencies::new(vec![
            UnitDeps::new("has-deps", vec![("x".to_string(), "1".to_string())]),
            UnitDeps::new("empty", vec![]),
        ]);
        let out = format_dependencies_section(&deps, false).join("\n");
        assert!(out.contains("has-deps"));
        assert!(!out.contains("empty"));
    }
}
