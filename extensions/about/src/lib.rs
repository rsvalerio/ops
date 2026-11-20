//! Generic about command: displays project identity card for any stack.
//!
//! Stack-specific extensions provide a `"project_identity"` data provider
//! returning [`ops_core::project_identity::ProjectIdentity`] as JSON.
//! When no provider is available, a minimal identity is built from the filesystem.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]

pub mod cards;
pub mod coverage;
pub mod deps;
pub mod identity;
pub mod lru;
pub mod machine;
pub mod manifest_cache;
pub mod manifest_io;
pub mod providers;
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
pub mod text_util;
pub mod units;
pub mod workspace;

#[cfg(feature = "sqlite")]
pub mod code;

#[cfg(feature = "sqlite")]
pub mod loc;

use std::io::Write;
use std::path::Path;

use ops_core::project_identity::{AboutCard, ProjectIdentity};
use ops_core::text::{capitalize, dir_name};
use ops_extension::provider_names::{COVERAGE, RUST_LOC, SQLITE, TOKEI};
use ops_extension::{DataProviderError, ExtensionType};

/// Write one machine-readable `--json` document: pretty-printed, newline
/// terminated. Every `ops about … --json` subpage funnels through here so
/// they share one formatting contract.
///
/// # Errors
///
/// If serialization or the write fails.
pub fn write_json_document<T: serde::Serialize + ?Sized>(
    writer: &mut dyn Write,
    doc: &T,
) -> anyhow::Result<()> {
    use anyhow::Context as _;
    serde_json::to_writer_pretty(&mut *writer, doc).context("writing JSON document")?;
    writeln!(writer)?;
    Ok(())
}

const NAME: &str = "about";
const DESCRIPTION: &str = "Project identity card";
const SHORTNAME: &str = "about";

/// Command extension that registers the `about` command.
pub struct AboutExtension;

ops_extension::impl_extension! {
    AboutExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::COMMAND,
    command_names: &["about"],
    data_provider_name: None,
    register_commands: |_self, registry| {
        // A bare "ops" would resolve through the invoking environment's
        // PATH, where a shim earlier on PATH silently becomes the binary
        // that renders the card. `ops_subcommand` spawns the absolute
        // current_exe()-resolved binary and renders as `ops about`. The
        // command only reads (providers, manifests, the db), so it opts out
        // of exclusivity.
        let mut spec = ops_core::config::ExecCommandSpec::ops_subcommand("about");
        spec.exclusive = false;
        registry.insert(
            "about".into(),
            ops_core::config::CommandSpec::Exec(spec),
        );
    },
    register_data_providers: |_self, _registry| {},
    factory: ABOUT_FACTORY = |_, _| {
        Some((NAME, Box::new(AboutExtension)))
    },
}

/// Options for the about command.
#[non_exhaustive]
pub struct AboutOptions {
    /// Re-collect provider data instead of serving cached results.
    pub refresh: bool,
    /// Card fields to render; `None` renders every field.
    pub visible_fields: Option<Vec<String>>,
    /// Whether the `writer` handed to [`run_about`] is a real terminal.
    ///
    /// Set `true` when writing to a terminal, `false` for buffers/files,
    /// regardless of whether `stdout` happens to be a TTY.
    pub is_tty: bool,
}

impl AboutOptions {
    /// Build options that render `visible_fields` from cached data to a non-terminal writer.
    ///
    /// Set the remaining flags by name with [`Self::with_refresh`] and
    /// [`Self::with_tty`].
    #[must_use = "pass the options to `run_about`; a discarded one renders nothing"]
    pub const fn new(visible_fields: Option<Vec<String>>) -> Self {
        Self { refresh: false, visible_fields, is_tty: false }
    }

    /// Set whether providers re-collect their data instead of serving cached results.
    #[must_use = "the updated options are returned, not applied in place"]
    pub const fn with_refresh(mut self, refresh: bool) -> Self {
        self.refresh = refresh;
        self
    }

    /// Set whether the writer handed to [`run_about`] is a real terminal.
    #[must_use = "the updated options are returned, not applied in place"]
    pub const fn with_tty(mut self, is_tty: bool) -> Self {
        self.is_tty = is_tty;
        self
    }
}

/// Run the generic about command.
///
/// Tries the `"project_identity"` data provider first. If no stack-specific
/// provider is registered, builds a minimal identity from the filesystem.
///
/// # Errors
///
/// If the `"project_identity"` provider fails with anything other than
/// `NotFound` (which falls back to a filesystem-derived identity), or if
/// writing the rendered card fails.
pub fn run_about(
    data_registry: &ops_extension::DataRegistry,
    opts: &AboutOptions,
    cwd: &Path,
    writer: &mut dyn Write,
) -> anyhow::Result<()> {
    let config = std::sync::Arc::new(ops_core::config::Config::empty());
    // `refresh` is fixed at construction time. It changes cache semantics
    // for every provider that runs on this context, so it is not something
    // a provider mid-traversal may reassign.
    let mut ctx = ops_extension::Context::new(config, cwd.to_path_buf());
    if opts.refresh {
        ctx = ctx.with_refresh();
    }

    warm_generic_providers(&mut ctx, data_registry, opts.refresh);
    let mut identity = resolve_identity(&mut ctx, data_registry, cwd)?;

    if identity.loc.is_none()
        || identity.dependency_count.is_none()
        || identity.coverage_percent.is_none()
        || identity.regions.is_empty()
    {
        enrich_from_db(&ctx, &mut identity);
    }

    let card = AboutCard::from_identity_filtered(&identity, opts.visible_fields.as_deref());
    writeln!(writer, "{}", card.render(opts.is_tty))?;

    Ok(())
}

fn warm_generic_providers(
    ctx: &mut ops_extension::Context,
    data_registry: &ops_extension::DataRegistry,
    refresh: bool,
) {
    // Sqlite/tokei warm-up failures are warn-logged, like the coverage
    // branch below, so a real provider error (permissions, disk full) is
    // not silently rendered as zeros.
    crate::providers::warm_providers(ctx, data_registry, &[SQLITE, TOKEI, RUST_LOC], "main");
    if refresh {
        match ctx.get_or_provide(COVERAGE, data_registry) {
            Ok(_) | Err(DataProviderError::NotFound(_)) => {}
            Err(e) => tracing::warn!(error = ?e, "coverage collection failed"),
        }
    }
}

fn resolve_identity(
    ctx: &mut ops_extension::Context,
    data_registry: &ops_extension::DataRegistry,
    cwd: &Path,
) -> anyhow::Result<ProjectIdentity> {
    match ctx.get_or_provide("project_identity", data_registry) {
        // Share the diagnosable deserialization path with `load_or_default`
        // so a failure names the `project_identity` provider, the target
        // type, and the failing field path — rather than a bare
        // `invalid type: ...` that could have come from any of
        // ProjectIdentity's fields or its nested `languages`. The helper
        // borrows the Arc payload rather than deep-cloning the JSON tree.
        Ok(value) => crate::providers::deserialize_payload("project_identity", value.as_ref()),
        Err(DataProviderError::NotFound(_)) => Ok(build_fallback_identity(cwd)?),
        Err(e) => Err(e.into()),
    }
}

/// Enrich identity with LOC/file count from `SQLite` if available.
///
/// Each of the six underlying queries acquires `db.lock()` independently,
/// so a concurrent ingestion that runs between samples can produce an
/// identity whose `loc`, `file_count`, `dependency_count`,
/// `coverage_percent`, and `regions` describe different snapshots. This is
/// accepted as a render-time visual artefact, as in
/// `units::enrich_from_db`: holding a single lock across all six queries
/// would require the helper layer to take an already-held `&Connection`,
/// and the about card re-renders on every invocation, so a stale frame is
/// self-correcting.
#[cfg(feature = "sqlite")]
fn enrich_from_db(ctx: &ops_extension::Context, identity: &mut ProjectIdentity) {
    let Some(db) = ops_sqlite::get_db(ctx) else {
        return;
    };

    use ops_sqlite::sql;
    if identity.regions.is_empty() {
        fill_rust_regions(db, identity);
    }
    if identity.loc.is_none() {
        identity.loc = query_kept("query_project_loc", || sql::query_project_loc(db), |&n| n > 0);
    }
    if identity.file_count.is_none() {
        identity.file_count = query_kept(
            "query_project_file_count",
            || sql::query_project_file_count(db),
            |&n| n > 0,
        );
    }
    if identity.dependency_count.is_none() {
        identity.dependency_count =
            query_kept("query_dependency_count", || sql::query_dependency_count(db), |&n| n > 0);
    }
    if identity.coverage_percent.is_none() {
        identity.coverage_percent = query_kept(
            "query_project_coverage",
            || sql::query_project_coverage(db),
            |c| c.lines_count > 0,
        )
        .map(|c| c.lines_percent);
    }
    // `languages` is deliberately not enriched: the card no longer renders
    // a language mix, and the `about code` page reads
    // `query_project_languages` itself.
}

/// Fill `regions`, and the `loc`/`file_count` totals, from the Rust region split.
///
/// The Rust region split is the card's codebase breakdown, and when it
/// exists it also supplies the totals: tokei counts every language while
/// the split counts only `.rs` code, so an all-language total must not
/// sit above a `.rs`-only split. Region-derived `loc`/`file_count`
/// therefore override whatever the provider set, and [`enrich_from_db`]
/// runs this first so its `is_none()` guards skip the tokei fills. The file count is the distinct-file figure —
/// per-region `files` overlap, so summing them would double-count a file
/// holding both production code and a `#[cfg(test)]` block.
#[cfg(feature = "sqlite")]
fn fill_rust_regions(db: &ops_sqlite::Sqlite, identity: &mut ProjectIdentity) {
    use ops_sqlite::sql;
    let summary =
        query_kept("query_rust_loc_summary", || sql::query_rust_loc_summary(db), |r| !r.is_empty());
    if let Some(regions) = summary {
        identity.loc = Some(regions.iter().map(|r| r.code).sum());
        identity.regions = regions
            .iter()
            .map(|r| ops_core::project_identity::RegionStat::new(&r.region, r.files, r.code))
            .collect();
        let files = query_kept(
            "query_rust_loc_file_count",
            || sql::query_rust_loc_file_count(db),
            |&n| n > 0,
        );
        if files.is_some() {
            identity.file_count = files;
        }
    }
}

/// Run one enrichment `query`, keeping its value only when `keep` accepts it.
///
/// A failed query logs at `tracing::warn!` with the query name and error
/// and yields `None`, so the identity field it would have filled stays as
/// the provider left it.
#[cfg(feature = "sqlite")]
fn query_kept<T>(
    query_name: &'static str,
    query: impl FnOnce() -> anyhow::Result<T>,
    keep: impl FnOnce(&T) -> bool,
) -> Option<T> {
    match query() {
        Ok(value) if keep(&value) => Some(value),
        Ok(_) => None,
        Err(e) => {
            tracing::warn!(query = query_name, error = ?e, "about: enrichment query failed");
            None
        }
    }
}

#[cfg(not(feature = "sqlite"))]
// CLIPPY: `const` keeps `cargo clippy -p ops-about` green with the
// `sqlite` feature off, where `missing_const_for_fn` fires on this empty stub.
const fn enrich_from_db(_ctx: &ops_extension::Context, _identity: &mut ProjectIdentity) {}

/// Build a minimal identity from the filesystem when no stack provider exists.
///
/// `project_root` follows the same non-UTF-8 policy as the provider path
/// ([`identity::build_identity_value`]): a non-UTF-8 `cwd` is rejected.
///
/// # Errors
///
/// [`DataProviderError`] when `cwd` is not valid UTF-8.
fn build_fallback_identity(cwd: &std::path::Path) -> Result<ProjectIdentity, DataProviderError> {
    use ops_core::stack::Stack;

    let name = dir_name(cwd).to_string();

    let stack = Stack::detect(cwd);
    let stack_label = stack.map_or_else(|| "Generic".to_string(), |s| capitalize(s.as_str()));

    let project_root = identity::project_root_str(cwd)?;
    Ok(ProjectIdentity::new(name, stack_label, project_root, "modules"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The registered command spawns an absolute
    /// `current_exe()`-derived program, never a bare PATH-resolved `"ops"`,
    /// and renders as `ops about`. It only reads (providers, manifests, the
    /// db), so it may overlap other steps in a parallel plan.
    #[test]
    fn registered_about_spawns_absolute_ops_and_displays_as_ops() {
        use ops_core::config::CommandSpec;
        use ops_extension::Extension as _;

        let mut registry = ops_extension::CommandRegistry::new();
        AboutExtension.register_commands(&mut registry);

        let Some(CommandSpec::Exec(exec)) = registry.get("about") else {
            panic!("about must be registered as an Exec spec");
        };
        // `current_exe()` succeeds under the test harness, so the program
        // must resolve absolute here; the literal "ops" is only the
        // fallback for when that lookup fails.
        assert!(
            std::path::Path::new(&exec.program).is_absolute(),
            "about must spawn an absolute current_exe()-derived program, got {:?}",
            exec.program
        );
        assert_eq!(exec.display_cmd(), "ops about");
        assert_eq!(exec.args, vec!["about".to_string()]);
        assert!(!exec.exclusive, "about only reads, so it may overlap other steps");
    }

    /// `run_about` composes `resolve_identity`, the
    /// `NotFound` fallback, the four-condition `enrich_from_db` guard and the
    /// card render, and no test drove it. With an empty registry
    /// `project_identity` is `NotFound`, so the rendered card must reflect
    /// `build_fallback_identity(cwd)` — the directory name and path — rather
    /// than erroring or rendering blank.
    #[test]
    fn run_about_falls_back_to_the_filesystem_identity_when_no_provider_is_registered() {
        let dir = tempfile::tempdir().unwrap();
        let cwd = dir.path().join("my-fallback-project");
        std::fs::create_dir(&cwd).unwrap();

        let registry = ops_extension::DataRegistry::new();
        let opts = AboutOptions::new(None);
        let mut out: Vec<u8> = Vec::new();
        run_about(&registry, &opts, &cwd, &mut out).expect("the fallback path must succeed");

        let rendered = String::from_utf8(out).unwrap();
        let expected = build_fallback_identity(&cwd).unwrap();
        assert!(
            rendered.contains(&expected.name),
            "card must show the fallback name {:?}; got {rendered}",
            expected.name
        );
        assert_eq!(expected.name, "my-fallback-project");
        // Is_tty = false must mean no ANSI escapes,
        // asserted through the runner rather than the card formatter.
        assert!(
            !rendered.contains('\u{1b}'),
            "a non-TTY writer must receive no ESC bytes; got {rendered:?}"
        );
    }

    #[test]
    fn fallback_identity_uses_dir_name() {
        let cwd = std::path::Path::new("/tmp/my-project");
        let id = build_fallback_identity(cwd).unwrap();
        assert_eq!(id.name, "my-project");
        assert_eq!(id.project_path, "/tmp/my-project");
        assert!(id.version.is_none());
        assert!(id.module_count.is_none());
    }

    /// The fallback path applies the provider path's non-UTF-8 policy: a
    /// non-UTF-8 `cwd` is rejected rather than rendered with `U+FFFD`.
    #[test]
    #[cfg(unix)]
    fn fallback_identity_rejects_non_utf8_cwd() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let bad_cwd = std::path::Path::new(OsStr::from_bytes(b"/ws/\xff/proj"));
        let err = build_fallback_identity(bad_cwd)
            .expect_err("non-UTF-8 cwd must be rejected on the fallback path");
        assert!(matches!(err, DataProviderError::ComputationMessage(_)));
        assert!(!err.to_string().contains('\u{FFFD}'));
    }

    #[test]
    fn fallback_identity_module_label_is_modules() {
        let cwd = std::path::Path::new("/tmp/test");
        let id = build_fallback_identity(cwd).unwrap();
        assert_eq!(id.module_label, "modules");
    }

    #[test]
    fn fallback_identity_defaults_are_empty() {
        let cwd = std::path::Path::new("/tmp/test");
        let id = build_fallback_identity(cwd).unwrap();
        assert!(id.description.is_none());
        assert!(id.license.is_none());
        assert!(id.repository.is_none());
        assert!(id.homepage.is_none());
        assert!(id.msrv.is_none());
        assert!(id.loc.is_none());
        assert!(id.file_count.is_none());
        assert!(id.dependency_count.is_none());
        assert!(id.coverage_percent.is_none());
        assert!(id.languages.is_empty());
        assert!(id.authors.is_empty());
        assert!(id.stack_detail.is_none());
    }

    #[test]
    fn fallback_identity_detects_rust_stack() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "").unwrap();
        let id = build_fallback_identity(dir.path()).unwrap();
        assert_eq!(id.stack_label, "Rust");
    }

    #[test]
    fn fallback_identity_detects_go_stack() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.mod"), "module test").unwrap();
        let id = build_fallback_identity(dir.path()).unwrap();
        assert_eq!(id.stack_label, "Go");
    }

    #[test]
    fn fallback_identity_detects_node_stack() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("package.json"), "{}").unwrap();
        let id = build_fallback_identity(dir.path()).unwrap();
        assert_eq!(id.stack_label, "Node");
    }

    #[test]
    fn fallback_identity_detects_python_stack() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("pyproject.toml"), "").unwrap();
        let id = build_fallback_identity(dir.path()).unwrap();
        assert_eq!(id.stack_label, "Python");
    }

    #[test]
    fn fallback_identity_generic_when_no_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let id = build_fallback_identity(dir.path()).unwrap();
        assert_eq!(id.stack_label, "Generic");
    }

    #[test]
    fn fallback_identity_name_from_dir() {
        let dir = tempfile::tempdir().unwrap();
        let expected = dir.path().file_name().unwrap().to_string_lossy().to_string();
        let id = build_fallback_identity(dir.path()).unwrap();
        assert_eq!(id.name, expected);
    }

    #[test]
    fn about_options_builder_names_each_flag() {
        let opts = AboutOptions::new(None);
        assert!(!opts.refresh && !opts.is_tty);
        let opts = AboutOptions::new(None).with_refresh(true);
        assert!(opts.refresh && !opts.is_tty);
        let opts = AboutOptions::new(None).with_tty(true);
        assert!(!opts.refresh && opts.is_tty);
    }

    #[test]
    fn about_options_fields() {
        let opts = AboutOptions {
            refresh: true,
            visible_fields: Some(vec!["project".to_string(), "codebase".to_string()]),
            is_tty: false,
        };
        assert!(opts.refresh);
        assert_eq!(opts.visible_fields.unwrap().len(), 2);

        let opts_default = AboutOptions { refresh: false, visible_fields: None, is_tty: false };
        assert!(!opts_default.refresh);
        assert!(opts_default.visible_fields.is_none());
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn enrich_from_db_logs_and_defaults_when_tables_missing() {
        // No tokei/coverage/metadata tables exist — every query returns 0 / empty
        // and per-query failures are warned (we exercise the fallible branches).
        let db = ops_sqlite::Sqlite::open_in_memory().expect("open in-memory db");
        ops_sqlite::init_schema(&db).expect("init_schema");

        let config = std::sync::Arc::new(ops_core::config::Config::empty());
        let mut ctx = ops_extension::Context::new(config, std::path::PathBuf::from("/tmp"));
        ctx.attach_db(std::sync::Arc::new(db));

        let mut identity = ProjectIdentity::default();
        enrich_from_db(&ctx, &mut identity);

        // Defaults preserved when underlying tables are absent
        assert!(identity.loc.is_none());
        assert!(identity.file_count.is_none());
        assert!(identity.dependency_count.is_none());
        assert!(identity.coverage_percent.is_none());
        assert!(identity.languages.is_empty());
        assert!(identity.regions.is_empty());
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn enrich_from_db_preserves_provider_loc_when_db_returns_zero() {
        let db = ops_sqlite::Sqlite::open_in_memory().expect("open in-memory db");
        ops_sqlite::init_schema(&db).expect("init_schema");

        let config = std::sync::Arc::new(ops_core::config::Config::empty());
        let mut ctx = ops_extension::Context::new(config, std::path::PathBuf::from("/tmp"));
        ctx.attach_db(std::sync::Arc::new(db));

        let mut identity = ProjectIdentity::default();
        identity.loc = Some(42);
        enrich_from_db(&ctx, &mut identity);

        assert_eq!(identity.loc, Some(42), "provider-supplied loc must survive");
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn enrich_from_db_skips_all_queries_when_identity_fully_populated() {
        let db = ops_sqlite::Sqlite::open_in_memory().expect("open in-memory db");
        ops_sqlite::init_schema(&db).expect("init_schema");

        let config = std::sync::Arc::new(ops_core::config::Config::empty());
        let mut ctx = ops_extension::Context::new(config, std::path::PathBuf::from("/tmp"));
        ctx.attach_db(std::sync::Arc::new(db));

        let mut identity = ProjectIdentity::default();
        identity.loc = Some(100);
        identity.file_count = Some(10);
        identity.dependency_count = Some(5);
        identity.coverage_percent = Some(85.0);
        identity.languages =
            vec![ops_core::project_identity::LanguageStat::new("Rust", 100, 10, 100.0, 100.0)];
        identity.regions = vec![ops_core::project_identity::RegionStat::new("main", 10, 100)];

        let lang_count_before = identity.languages.len();
        let region_count_before = identity.regions.len();

        enrich_from_db(&ctx, &mut identity);

        assert_eq!(identity.loc, Some(100));
        assert_eq!(identity.file_count, Some(10));
        assert_eq!(identity.dependency_count, Some(5));
        assert_eq!(identity.coverage_percent, Some(85.0));
        assert_eq!(identity.languages.len(), lang_count_before);
        assert_eq!(identity.regions.len(), region_count_before);
    }

    /// A populated `rust_loc_summary` view supplies the region split and
    /// overrides `loc`/`file_count` — tokei's all-language totals must not
    /// sit above a `.rs`-only split. The file count is the distinct-file
    /// figure (3 files), not the sum of per-region rows (4).
    #[cfg(feature = "sqlite")]
    #[test]
    fn enrich_from_db_regions_override_loc_and_file_count() {
        let db = ops_sqlite::Sqlite::open_in_memory().expect("open in-memory db");
        ops_sqlite::init_schema(&db).expect("init_schema");
        {
            let conn = db.lock().expect("lock");
            conn.execute_batch(
                "CREATE TABLE rust_loc_files (file VARCHAR, region VARCHAR, code BIGINT, \
                 docs BIGINT, comments BIGINT, blanks BIGINT, lines BIGINT); \
                 INSERT INTO rust_loc_files VALUES ('src/lib.rs', 'main', 300, 40, 10, 50, 400); \
                 INSERT INTO rust_loc_files VALUES ('src/util.rs', 'main', 200, 10, 5, 25, 240); \
                 INSERT INTO rust_loc_files VALUES ('src/lib.rs', 'test', 120, 0, 4, 16, 140); \
                 INSERT INTO rust_loc_files VALUES ('examples/demo.rs', 'example', 30, 2, 1, 7, 40); \
                 CREATE VIEW rust_loc_summary AS \
                 SELECT region, COUNT(*) AS files, SUM(code) AS code, SUM(docs) AS docs, \
                 SUM(comments) AS comments, SUM(blanks) AS blanks, SUM(lines) AS lines \
                 FROM rust_loc_files GROUP BY region ORDER BY code DESC",
            )
            .expect("rust loc fixture");
        }

        let config = std::sync::Arc::new(ops_core::config::Config::empty());
        let mut ctx = ops_extension::Context::new(config, std::path::PathBuf::from("/tmp"));
        ctx.attach_db(std::sync::Arc::new(db));

        let mut identity = ProjectIdentity::default();
        identity.loc = Some(999_999);
        identity.file_count = Some(9_999);
        enrich_from_db(&ctx, &mut identity);

        assert_eq!(identity.loc, Some(650), "summed region code");
        assert_eq!(identity.file_count, Some(3), "distinct files, not 4");
        assert_eq!(identity.regions.len(), 3);
        let main = identity.regions.iter().find(|r| r.region == "main").expect("main region");
        assert_eq!((main.files, main.code), (2, 500));
    }

    #[cfg(not(feature = "sqlite"))]
    #[test]
    fn enrich_from_db_noop_without_sqlite() {
        let config = std::sync::Arc::new(ops_core::config::Config::empty());
        let ctx = ops_extension::Context::new(config, std::path::PathBuf::from("/tmp"));
        let mut identity = ProjectIdentity::default();
        enrich_from_db(&ctx, &mut identity);
        // Should be a no-op — all fields remain default
        assert!(identity.loc.is_none());
        assert!(identity.file_count.is_none());
        assert!(identity.dependency_count.is_none());
        assert!(identity.coverage_percent.is_none());
        assert!(identity.languages.is_empty());
    }
}
