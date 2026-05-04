//! Rust `project_coverage` data provider.

use ops_core::project_identity::{CoverageStats, ProjectCoverage, UnitCoverage};
use ops_duckdb::sql::{query_crate_coverage, query_or_warn, query_project_coverage};
use ops_extension::{Context, DataProvider, DataProviderError};

use crate::query::{load_workspace_manifest, log_manifest_load_failure};
use crate::units::resolve_crate_display_name;

pub(crate) const PROVIDER_NAME: &str = "project_coverage";

pub(crate) struct RustCoverageProvider;

impl DataProvider for RustCoverageProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let cwd = ctx.working_directory.clone();
        let manifest = match load_workspace_manifest(ctx) {
            Ok(m) => Some(m),
            Err(e) => {
                log_manifest_load_failure(&e);
                None
            }
        };

        let Some(db) = ops_duckdb::get_db(ctx) else {
            return Ok(serde_json::to_value(ProjectCoverage::default())?);
        };

        // ERR-2 / TASK-0376 / PATTERN-1 (TASK-0608): route through
        // `query_or_warn` so this site matches the convention used by every
        // sister DuckDB call in the crate (units, identity::metrics,
        // deps_provider). Wrapping the return in `Option` preserves the
        // early-return-on-failure semantics — if the project_coverage query
        // fails we return a fully-default `ProjectCoverage` rather than
        // partial data, matching the prior behaviour.
        let project_total = query_or_warn(
            "query_project_coverage",
            "reporting empty coverage",
            None,
            || query_project_coverage(db).map(Some),
        );
        let Some(p) = project_total else {
            return Ok(serde_json::to_value(ProjectCoverage::default())?);
        };
        let total = CoverageStats::new(p.lines_percent, p.lines_covered, p.lines_count);

        let units = if let Some(manifest) = manifest {
            let members: &[String] = manifest
                .workspace
                .as_ref()
                .map_or(&[][..], |ws| ws.members.as_slice());
            if members.is_empty() {
                Vec::new()
            } else {
                // READ-5 / TASK-0986: short-circuit when the workspace cwd is
                // not valid UTF-8 instead of piping a U+FFFD-replaced string
                // into the SQL key. The lossy collapse would silently match
                // an unrelated workspace's coverage rows. Sister policy to
                // TASK-0946 (workspace member relpaths in query.rs).
                let Some(cwd_str) = cwd.to_str() else {
                    tracing::warn!(
                        cwd = ?cwd.display(),
                        "non-UTF-8 cwd; skipping per-crate coverage to avoid lossy SQL key collapse"
                    );
                    return Ok(serde_json::to_value(ProjectCoverage::new(
                        total,
                        Vec::new(),
                    ))?);
                };
                let member_strs: Vec<&str> = members.iter().map(String::as_str).collect();
                let per_crate = query_or_warn(
                    "query_crate_coverage",
                    "per-crate coverage will be blank",
                    std::collections::HashMap::<String, ops_duckdb::sql::CrateCoverage>::new(),
                    || query_crate_coverage(db, &member_strs, cwd_str),
                );
                // PERF-1 (TASK-0798): resolve display names up front in one
                // pass over members with coverage rows, so each member's
                // Cargo.toml is read at most once per provide() call.
                let mut display_names: std::collections::HashMap<&str, String> =
                    std::collections::HashMap::with_capacity(per_crate.len());
                for member in members {
                    if per_crate.contains_key(member.as_str()) {
                        display_names
                            .insert(member.as_str(), resolve_crate_display_name(member, &cwd));
                    }
                }
                members
                    .iter()
                    .filter_map(|member| {
                        let cov = per_crate.get(member)?;
                        let unit_name = display_names.remove(member.as_str())?;
                        Some(UnitCoverage::new(
                            unit_name,
                            member.clone(),
                            CoverageStats::new(
                                cov.lines_percent,
                                cov.lines_covered,
                                cov.lines_count,
                            ),
                        ))
                    })
                    .collect()
            }
        } else {
            Vec::new()
        };

        let coverage = ProjectCoverage::new(total, units);
        serde_json::to_value(&coverage).map_err(DataProviderError::from)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    /// READ-5 / TASK-0986: a non-UTF-8 cwd must NOT collapse to a
    /// U+FFFD-replaced SQL key. The provider's short-circuit relies on
    /// `Path::to_str()` returning `None` for non-UTF-8 input — pin that
    /// invariant so a future refactor that swaps in `to_string_lossy`
    /// can't silently re-introduce the lossy-collapse.
    #[test]
    fn non_utf8_cwd_path_to_str_returns_none() {
        // Construct a non-UTF-8 path: 0x80 is a continuation byte with no
        // leading byte, so it's invalid UTF-8.
        let bytes = b"/tmp/non\xC3\x28-utf8";
        let p = Path::new(OsStr::from_bytes(bytes));
        assert!(
            p.to_str().is_none(),
            "non-UTF-8 path must not pass `to_str()`; got: {:?}",
            p.to_str()
        );
        // Confirm that `to_string_lossy` would have produced a U+FFFD
        // replacement key — the very behaviour the short-circuit avoids.
        let lossy = p.to_string_lossy();
        assert!(
            lossy.contains('\u{FFFD}'),
            "expected lossy conversion to produce U+FFFD: {lossy}"
        );
    }
}
