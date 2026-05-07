//! Go `project_units` data provider.
//!
//! Reads `go.work` / `go.mod` to build a list of [`ProjectUnit`] entries
//! describing each module. LOC/file counts are enriched by the generic
//! `ops_about::run_about_units` runner.

use std::path::Path;

use ops_about::cards::format_unit_name;
use ops_core::project_identity::ProjectUnit;
use ops_extension::{Context, DataProvider, DataProviderError};

pub(crate) const PROVIDER_NAME: &str = "project_units";

pub(crate) struct GoUnitsProvider;

impl DataProvider for GoUnitsProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let units = collect_units(ctx.working_directory.as_path());
        serde_json::to_value(&units).map_err(DataProviderError::from)
    }
}

fn collect_units(cwd: &Path) -> Vec<ProjectUnit> {
    if let Some(dirs) = workspace_use_dirs(cwd) {
        return dirs
            .into_iter()
            .map(|dir| unit_from_use_dir(cwd, &dir))
            .collect();
    }
    let (module, go_version) = read_mod_info(cwd);
    match module {
        Some(m) => {
            let mut unit = ProjectUnit::new(
                last_segment(Some(&m)).unwrap_or_else(|| m.clone()),
                // Empty path matches every file in `tokei_files` via starts_with.
                String::new(),
            );
            unit.version = go_version;
            unit.description = Some(m);
            vec![unit]
        }
        None => vec![],
    }
}

/// FN-1 / TASK-0820: build the [`ProjectUnit`] for a single `go.work` use
/// directive. Handles path normalisation, the out-of-tree diagnostic, the
/// per-module `go.mod` lookup, and the description-shaping that distinguishes
/// `(outside project root)` members.
fn unit_from_use_dir(cwd: &Path, dir: &str) -> ProjectUnit {
    let normalized = normalize_module_path(dir);
    // PATTERN-1 (TASK-1027): test the *first path component* rather than the
    // raw string. `starts_with("..")` would also flag legal directories like
    // `..staging/api` or `..backup-2025` whose first component merely begins
    // with two dots. Split on both `/` and `\\` so go.work entries authored on
    // Windows are handled too.
    let out_of_tree = normalized
        .split(['/', '\\'])
        .next()
        .is_some_and(|first| first == "..");
    if out_of_tree {
        // Out-of-tree workspace members (e.g. `use ../shared`) match no
        // `tokei_files.file` entry under cwd, so the unit would render with
        // zero LOC and no diagnostic. Surface it instead.
        // ERR-7 (TASK-0665 / TASK-0809): Debug-format the directive so
        // embedded newlines / ANSI escapes cannot forge log lines, matching
        // the project-wide path-log policy.
        tracing::warn!(
            directive = ?dir,
            "go.work `use` directive points outside the project root; LOC stats will be empty",
        );
    }
    let mod_path = cwd.join(&normalized);
    let (module, go_version) = read_mod_info(&mod_path);
    let name = last_segment(module.as_deref()).unwrap_or_else(|| format_unit_name(&normalized));
    let description = if out_of_tree {
        Some(match module {
            Some(m) => format!("{m} (outside project root)"),
            None => "(outside project root)".to_string(),
        })
    } else {
        module
    };
    let mut unit = ProjectUnit::new(name, normalized);
    unit.version = go_version;
    unit.description = description;
    unit
}

/// Normalize a `go.work` use-directive entry so it matches `tokei_files.file`
/// paths (which are recorded relative to cwd with no `./` prefix).
/// `.` → empty string (signals project-root module; enriched via project-wide
/// stats instead of the per-crate SQL join).
fn normalize_module_path(dir: &str) -> String {
    let trimmed = dir
        .trim_start_matches("./")
        .trim_start_matches(".\\")
        .trim_end_matches(['/', '\\']);
    if trimmed == "." {
        String::new()
    } else {
        trimmed.to_string()
    }
}

fn last_segment(module: Option<&str>) -> Option<String> {
    module
        .and_then(|m| m.rsplit('/').next())
        .map(|s| s.to_string())
}

fn workspace_use_dirs(root: &Path) -> Option<Vec<String>> {
    crate::go_work::parse_use_dirs(root)
}

fn read_mod_info(dir: &Path) -> (Option<String>, Option<String>) {
    match crate::go_mod::parse(dir) {
        Some(m) => (m.module, m.go_version),
        None => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_use_dirs_multi() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t./api\n\t./cmd\n)\n",
        )
        .unwrap();
        let dirs = workspace_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./api", "./cmd"]);
    }

    #[test]
    fn read_mod_info_basic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module github.com/user/myapp\n\ngo 1.22\n",
        )
        .unwrap();
        let (module, go_version) = read_mod_info(dir.path());
        assert_eq!(module.as_deref(), Some("github.com/user/myapp"));
        assert_eq!(go_version.as_deref(), Some("1.22"));
    }

    #[test]
    fn collect_units_workspace() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t./api\n\t./cmd\n)\n",
        )
        .unwrap();
        let api = dir.path().join("api");
        std::fs::create_dir(&api).unwrap();
        std::fs::write(api.join("go.mod"), "module example.com/api\n\ngo 1.21\n").unwrap();
        let cmd = dir.path().join("cmd");
        std::fs::create_dir(&cmd).unwrap();
        std::fs::write(cmd.join("go.mod"), "module example.com/cmd\n\ngo 1.22\n").unwrap();

        let units = collect_units(dir.path());
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].name, "api");
        assert_eq!(units[0].description.as_deref(), Some("example.com/api"));
        assert_eq!(units[1].version.as_deref(), Some("1.22"));
    }

    #[test]
    fn collect_units_single_mod() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module github.com/user/app\n\ngo 1.23\n",
        )
        .unwrap();
        let units = collect_units(dir.path());
        assert_eq!(units.len(), 1);
        // Empty path signals project-root module; enrichment uses project-wide stats.
        assert_eq!(units[0].path, "");
        assert_eq!(units[0].name, "app");
    }

    #[test]
    fn normalize_strips_dot_slash_prefix() {
        assert_eq!(
            normalize_module_path("./staging/src/k8s.io/api"),
            "staging/src/k8s.io/api"
        );
        assert_eq!(normalize_module_path("./api/"), "api");
        assert_eq!(normalize_module_path("."), "");
        assert_eq!(normalize_module_path("pkg/foo"), "pkg/foo");
    }

    #[test]
    fn collect_units_out_of_tree_use_directive_does_not_panic() {
        // `use ../shared` is accepted by cmd/go but lives outside cwd; the
        // resulting unit has a `..` path that won't match tokei_files. We
        // surface a diagnostic and emit the unit anyway (zero LOC).
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t../shared\n)\n",
        )
        .unwrap();
        let units = collect_units(dir.path());
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].path, "../shared");
        assert_eq!(
            units[0].description.as_deref(),
            Some("(outside project root)")
        );
    }

    /// ERR-7 (TASK-0809): the `use` directive flows through `tracing::warn!`
    /// via the `?` formatter so embedded newlines or ANSI escapes cannot
    /// forge multi-line log records. DUP-3 / TASK-0985: shared helper —
    /// see `ops_about::test_support`.
    #[test]
    fn directive_debug_escapes_control_characters() {
        let dir = "../shared\nINJECTED line\u{1b}[31m";
        ops_about::test_support::assert_debug_escapes_control_chars(dir);
    }

    /// PATTERN-1 (TASK-1027): a `use ..staging/api` directive points at a
    /// legal directory whose first component merely *begins* with `..`. It
    /// must be treated as in-tree: no `(outside project root)` suffix and no
    /// `tracing::warn!` emitted. The previous `starts_with("..")` check
    /// misclassified this as out-of-tree.
    /// Minimal `tracing::Subscriber` that records the `Level` of every event.
    /// Lets us assert that no `WARN` was emitted without pulling in
    /// `tracing-subscriber` as a dev-dependency.
    struct WarnCounter(std::sync::Arc<std::sync::atomic::AtomicUsize>);
    impl tracing::Subscriber for WarnCounter {
        fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            if *event.metadata().level() == tracing::Level::WARN {
                self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }
        fn enter(&self, _span: &tracing::span::Id) {}
        fn exit(&self, _span: &tracing::span::Id) {}
    }

    #[test]
    fn collect_units_dotdot_prefixed_dir_is_in_tree() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t./..staging/api\n)\n",
        )
        .unwrap();
        let staging_api = dir.path().join("..staging").join("api");
        std::fs::create_dir_all(&staging_api).unwrap();
        std::fs::write(
            staging_api.join("go.mod"),
            "module example.com/staging/api\n\ngo 1.21\n",
        )
        .unwrap();

        let warn_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let subscriber = WarnCounter(warn_count.clone());
        let units = tracing::subscriber::with_default(subscriber, || collect_units(dir.path()));

        assert_eq!(units.len(), 1);
        assert_eq!(units[0].path, "..staging/api");
        // In-tree: description is the bare module name, no suffix.
        assert_eq!(
            units[0].description.as_deref(),
            Some("example.com/staging/api")
        );
        assert!(!units[0]
            .description
            .as_deref()
            .unwrap_or("")
            .contains("(outside project root)"));
        assert_eq!(warn_count.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[test]
    fn collect_units_empty() {
        let dir = tempfile::tempdir().unwrap();
        let units = collect_units(dir.path());
        assert!(units.is_empty());
    }
}
