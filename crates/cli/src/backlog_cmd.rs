//! `ops backlog …` dispatch: resolves the backlog config, opens the store,
//! and maps the clap actions onto the ops-backlog handlers.
//!
//! Config precedence: a non-empty `[backlog]` section in `.ops.toml` wins
//! entirely (unset keys fall back to the built-in defaults); otherwise
//! `backlog.config.yml`; otherwise the defaults. ops-backlog knows nothing
//! about `.ops.toml`, so this crate — which depends on both — owns the
//! resolution and the init flow that writes either file.

use std::io::Write;
use std::path::Path;

use anyhow::Context as _;
use ops_backlog::cmd;
use ops_backlog::config::BacklogConfig;
use ops_backlog::store::Store;
use ops_core::config::{BacklogSection, Config};

use crate::args::{BacklogAction, BacklogTaskAction};

/// Resolve the effective backlog config: `.ops.toml`'s `[backlog]` section
/// when it sets any key, else `backlog.config.yml` (absent = defaults).
///
/// A present-but-empty section is treated as absent, and — matching
/// `BacklogConfig::parse` — an empty `statuses` list inside a section is
/// ignored rather than blanking the columns.
///
/// # Errors
///
/// The yml fallback path is taken and `backlog.config.yml` is present but
/// unparseable (the error names the file).
fn resolve_backlog_config(
    cwd: &Path,
    section: Option<&BacklogSection>,
) -> anyhow::Result<BacklogConfig> {
    let Some(section) = section.filter(|s| **s != BacklogSection::default()) else {
        return BacklogConfig::load(cwd);
    };
    let mut cfg = BacklogConfig::default();
    if let Some(v) = &section.default_status {
        cfg.default_status.clone_from(v);
    }
    if let Some(list) = section.statuses.as_ref().filter(|l| !l.is_empty()) {
        cfg.statuses.clone_from(list);
    }
    if let Some(n) = section.zero_padded_ids {
        cfg.zero_padded_ids = n;
    }
    if let Some(v) = &section.task_prefix {
        cfg.task_prefix.clone_from(v);
    }
    if let Some(v) = &section.backlog_directory {
        cfg.backlog_directory.clone_from(v);
    }
    Ok(cfg)
}

/// The config as it stands on disk right now — the post-init read-back that
/// decides where the tasks tree lands. Reads the workspace `.ops.toml` and
/// `backlog.config.yml` (the two files init itself writes) rather than the
/// full loader chain: the backlog section in a global config or `.ops.d`
/// fragment is exotic, and init's contract is about these two files.
///
/// # Errors
///
/// `.ops.toml` is present but unparseable, or the yml fallback path hit an
/// unparseable `backlog.config.yml` — both name the file.
fn effective_backlog_config(cwd: &Path) -> anyhow::Result<BacklogConfig> {
    let overlay = ops_core::config::read_config_file(&cwd.join(".ops.toml"))?;
    resolve_backlog_config(cwd, overlay.as_ref().and_then(|o| o.backlog.as_ref()))
}

/// `ops backlog init`: bootstrap the backlog config and tasks tree. See
/// [`run_backlog_init_to`] for the steps; this is the stdout-facing shell.
///
/// # Errors
///
/// As [`run_backlog_init_to`].
pub fn run_backlog_init(cwd: &Path, backlog_md: bool) -> anyhow::Result<()> {
    run_backlog_init_to(cwd, backlog_md, &mut std::io::stdout())
}

/// The init flow, writer-injectable for tests (`run_init_to` idiom):
/// ensure the config ([`ensure_backlog_config_to`]) then the tasks tree
/// ([`ensure_tasks_tree_to`]). Every step is idempotent and never
/// destructive; reruns report what they skipped.
///
/// # Errors
///
/// As the two steps.
pub fn run_backlog_init_to(
    cwd: &Path,
    backlog_md: bool,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    ensure_backlog_config_to(cwd, backlog_md, out)?;
    ensure_tasks_tree_to(cwd, out)
}

/// The config half of init. `--backlog.md`: write `backlog.config.yml`
/// (the five-key subset ops honors) when absent; `.ops.toml` is untouched.
/// Without the flag: insert a `[backlog]` section with the sane defaults
/// into `.ops.toml` — but only when the backlog is not already configured
/// somewhere: an existing `.ops.toml` section is left byte-unchanged, and
/// an existing `backlog.config.yml` keeps ownership (writing the section
/// anyway would let its defaults shadow the yml's real values, since the
/// section wins at resolve time). The file's formatting and comments
/// survive the edit (`toml_edit`); a missing `.ops.toml` is created
/// carrying only the section.
///
/// # Errors
///
/// `.ops.toml`/`backlog.config.yml` is present but unparseable (read is
/// refused before anything is written), a write fails, or writing `out`
/// failed — each names the path.
pub fn ensure_backlog_config_to(
    cwd: &Path,
    backlog_md: bool,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    if backlog_md {
        let yml = cwd.join("backlog.config.yml");
        if yml
            .try_exists()
            .with_context(|| format!("checking {}", yml.display()))?
        {
            writeln!(out, "backlog.config.yml already exists, left unchanged")
                .context("printing the already-exists notice")?;
        } else {
            // write_config_yml claims the name with a no-clobber atomic
            // link, so a file appearing between the check above and the
            // write fails the claim with an error — never an overwrite.
            ops_backlog::config::write_config_yml(cwd, &BacklogConfig::default())?;
            writeln!(out, "Created backlog.config.yml")
                .context("printing the created-config notice")?;
        }
        return Ok(());
    }

    let yml = cwd.join("backlog.config.yml");
    if yml
        .try_exists()
        .with_context(|| format!("checking {}", yml.display()))?
    {
        writeln!(
            out,
            "backlog.config.yml already configures the backlog, left unchanged"
        )
        .context("printing the yml-owns-the-config notice")?;
        return Ok(());
    }

    let path = cwd.join(".ops.toml");
    let existed = path
        .try_exists()
        .with_context(|| format!("checking {}", path.display()))?;
    // Missing file → empty document; malformed → hard error rather than
    // an edit that could clobber the user's config.
    let mut doc = ops_core::config::read_ops_toml(&path)?;
    if doc.contains_key("backlog") {
        writeln!(
            out,
            ".ops.toml already configures [backlog], left unchanged"
        )
        .context("printing the already-configured notice")?;
        return Ok(());
    }
    let section = BacklogSection::sane_defaults();
    let table = ops_core::config::ensure_table(&mut doc, "backlog")?;
    insert_backlog_section(table, &section)?;
    ops_core::config::write_ops_toml(&path, &doc)?;
    if existed {
        writeln!(out, "Added [backlog] to .ops.toml").context("printing the added-section notice")
    } else {
        writeln!(out, "Created .ops.toml with a [backlog] section")
            .context("printing the created-config notice")
    }
}

/// The tree half of init: create `<backlog_directory>/tasks/` under the
/// config that now applies (`Store::open`'s one requirement), saying so
/// only when it did not already exist.
///
/// # Errors
///
/// A config file is present but unparseable, the directory cannot be
/// created, or writing `out` failed — each names the path.
pub fn ensure_tasks_tree_to(cwd: &Path, out: &mut dyn Write) -> anyhow::Result<()> {
    let cfg = effective_backlog_config(cwd)?;
    ensure_tasks_tree_with(cwd, &cfg, out)
}

/// The tree-creation step with the config already resolved — the fallback
/// `ops init` drives when the backlog bootstrap cannot read `.ops.toml`
/// (malformed TOML): the location comes from the yml-or-defaults config
/// instead, so the tree still lands while the broken manifest is only
/// reported, matching `ops init`'s keep-working contract.
///
/// # Errors
///
/// The directory cannot be created, or writing `out` failed — each names
/// the path.
pub fn ensure_tasks_tree_with(
    cwd: &Path,
    cfg: &BacklogConfig,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    let tasks = cwd.join(&cfg.backlog_directory).join("tasks");
    if tasks.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(&tasks).with_context(|| format!("creating {}", tasks.display()))?;
    writeln!(
        out,
        "Created {}/tasks/",
        cfg.backlog_directory.trim_end_matches('/')
    )
    .context("printing the created-tree notice")
}

/// Land the section's set keys in a fresh `[backlog]` table, in the yml
/// writer's key order. `skip_serializing_if` keeps unset keys out, so only
/// the `Some` fields of a partial section are written.
///
/// # Errors
///
/// `zero_padded_ids` does not fit a TOML integer.
fn insert_backlog_section(
    table: &mut toml_edit::Table,
    section: &BacklogSection,
) -> anyhow::Result<()> {
    if let Some(v) = &section.default_status {
        table.insert("default_status", toml_edit::value(v));
    }
    if let Some(list) = &section.statuses {
        let mut arr = toml_edit::Array::new();
        for status in list {
            arr.push(status);
        }
        table.insert("statuses", toml_edit::value(arr));
    }
    if let Some(n) = section.zero_padded_ids {
        // TOML integers are i64; a padding width that does not fit is a
        // config no integer file can express.
        let n = i64::try_from(n).context("zero_padded_ids exceeds the TOML integer range")?;
        table.insert("zero_padded_ids", toml_edit::value(n));
    }
    if let Some(v) = &section.task_prefix {
        table.insert("task_prefix", toml_edit::value(v));
    }
    if let Some(v) = &section.backlog_directory {
        table.insert("backlog_directory", toml_edit::value(v));
    }
    Ok(())
}

/// Resolve the workspace root (cwd) and the effective backlog config, open
/// the `.backlog` store, and run the action with stdout. `config` is the
/// already-loaded ops config — threading it keeps one config load per
/// invocation; only its `[backlog]` section feeds the backlog resolution.
///
/// # Errors
///
/// The cwd is unreadable, a config file is present but unparseable, the
/// `.backlog/tasks` tree is missing (the error names it), or a handler
/// failed — all bubble as anyhow context for `ops: error: …`.
pub fn run_backlog(cwd: &Path, config: &Config, action: BacklogAction) -> anyhow::Result<()> {
    // init is the one action that runs without a store — creating the tree
    // (which Store::open requires) is its job.
    if let BacklogAction::Init { backlog_md } = action {
        return run_backlog_init(cwd, backlog_md);
    }
    // create-review-tasks runs without a store too — the engine scans the
    // tree and writes the task files itself — and it needs the full config
    // to build the data registry holding the review_targets provider, so it
    // dispatches before the backlog-only config resolution below.
    if let BacklogAction::CreateReviewTasks { dry_run } = action {
        return crate::subcommands::run_create_review_tasks(config, dry_run);
    }
    let cfg = resolve_backlog_config(cwd, Some(&config.backlog))?;
    let backlog_root = cwd.join(&cfg.backlog_directory);
    let store = Store::open(&backlog_root)?;
    match action {
        // Unreachable past the early return above — the type cannot express
        // "not Init" — so the arm repeats the idempotent call rather than
        // panicking; if a refactor ever drops the early return, init still
        // lands here (after a spurious store-open failure).
        BacklogAction::Init { backlog_md } => run_backlog_init(cwd, backlog_md),
        // Same shape as Init: unreachable past the early return above, and
        // the repeated call keeps the arm total if that return ever moves.
        BacklogAction::CreateReviewTasks { dry_run } => {
            crate::subcommands::run_create_review_tasks(config, dry_run)
        }
        BacklogAction::Task { action } => run_task_action(&store, &cfg, cwd, action),
        BacklogAction::Search {
            query,
            modified_file,
            exclude_status,
            plain,
        } => {
            let _ = plain; // plain is the only renderer in scope
            let opts = cmd::SearchOptions {
                query,
                modified_file,
                exclude_status,
                plain: true,
            };
            cmd::run_search(&store, &opts, &mut std::io::stdout())
        }
        BacklogAction::Wave { action } => run_wave_action(&store, &cfg, action),
        BacklogAction::Cleanup {
            older_than,
            dry_run,
        } => {
            let opts = cmd::CleanupOptions {
                older_than_days: older_than,
                dry_run,
            };
            cmd::run_cleanup(&store, &cfg, &opts, &mut std::io::stdout())
        }
    }
}

/// `ops about backlog`: the same config + store preamble as [`run_backlog`],
/// then the read-only overview. Kept here so every path into the backlog
/// tree resolves `.backlog` through one config load.
///
/// # Errors
///
/// A config file is present but unparseable, the `.backlog/tasks` tree is
/// missing (the error names it), a task file anywhere in the tree does not
/// parse, or writing stdout failed.
pub fn run_about_backlog(cwd: &Path, section: &BacklogSection) -> anyhow::Result<()> {
    let cfg = resolve_backlog_config(cwd, Some(section))?;
    let backlog_root = cwd.join(&cfg.backlog_directory);
    let store = Store::open(&backlog_root)?;
    cmd::run_about_backlog(&store, &cfg, &mut std::io::stdout())
}

/// Map the clap edit args onto the handler options.
fn edit_options_from(edit: Box<crate::args::BacklogEditArgs>) -> cmd::EditOptions {
    let crate::args::BacklogEditArgs {
        task_id,
        status,
        assignee,
        add_label,
        append_notes,
        priority,
        title,
        description,
        ac,
        check_ac,
        uncheck_ac,
        dod,
        check_dod,
        uncheck_dod,
        parent,
        clear_parent,
        add_dep,
        remove_dep,
        plain: _,
    } = *edit;
    cmd::EditOptions {
        task_id,
        status,
        assignees: assignee,
        add_labels: add_label,
        append_notes,
        priority,
        title,
        description,
        ac,
        check_ac,
        uncheck_ac,
        dod,
        check_dod,
        uncheck_dod,
        parent,
        clear_parent,
        add_dep,
        remove_dep,
    }
}

/// Map the clap wave args onto the wave handlers.
fn run_wave_action(
    store: &Store,
    cfg: &BacklogConfig,
    action: crate::args::BacklogWaveAction,
) -> anyhow::Result<()> {
    use crate::args::BacklogWaveAction;

    match action {
        BacklogWaveAction::List {
            status,
            marker,
            plain: _,
            json,
        } => {
            let opts = cmd::WaveListOptions {
                marker,
                statuses: status,
                json,
            };
            cmd::run_wave_list(store, cfg, &opts, &mut std::io::stdout())
        }
        BacklogWaveAction::Members {
            wave_id,
            plain: _,
            json,
        } => {
            let opts = cmd::WaveMembersOptions { wave_id, json };
            cmd::run_wave_members(store, cfg, &opts, &mut std::io::stdout())
        }
        BacklogWaveAction::Migrate { marker, dry_run } => {
            let opts = cmd::WaveMigrateOptions { marker, dry_run };
            cmd::run_wave_migrate(store, &opts, &mut std::io::stdout())
        }
    }
}

fn run_task_action(
    store: &Store,
    cfg: &BacklogConfig,
    cwd: &Path,
    action: BacklogTaskAction,
) -> anyhow::Result<()> {
    match action {
        BacklogTaskAction::Create(create) => {
            let crate::args::BacklogCreateArgs {
                title,
                description,
                assignee,
                status,
                labels,
                priority,
                ac,
                dod,
                modified_file,
                plan,
                notes,
                depends_on,
                plain: _,
            } = *create;
            let opts = cmd::CreateOptions {
                title,
                description,
                assignees: assignee,
                status,
                labels,
                priority,
                ac,
                dod,
                modified_files: modified_file,
                plan,
                notes,
                dependencies: depends_on,
            };
            cmd::run_create(store, cfg, &opts, &mut std::io::stdout())
        }
        BacklogTaskAction::Edit(edit) => {
            let opts = edit_options_from(edit);
            cmd::run_edit(store, &opts, &mut std::io::stdout())
        }
        BacklogTaskAction::List {
            status,
            assignee,
            labels,
            parent,
            dependents,
            plain: _,
            json,
        } => {
            let opts = cmd::ListOptions {
                statuses: status,
                assignees: assignee,
                labels,
                parent,
                dependents,
                // `--plain` and the default render identically, so only
                // `--json` selects the format.
                format: if json {
                    cmd::OutputFormat::Json
                } else {
                    cmd::OutputFormat::Plain
                },
            };
            cmd::run_list(store, cfg, &opts, &mut std::io::stdout())
        }
        BacklogTaskAction::View {
            task_id,
            plain: _,
            json,
        } => {
            let opts = cmd::ViewOptions {
                task_id,
                format: if json {
                    cmd::OutputFormat::Json
                } else {
                    cmd::OutputFormat::Plain
                },
            };
            cmd::run_view(store, &opts, cwd, &mut std::io::stdout())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_init(dir: &std::path::Path, backlog_md: bool) -> String {
        let mut out = Vec::new();
        run_backlog_init_to(dir, backlog_md, &mut out).expect("init");
        String::from_utf8(out).expect("utf8")
    }

    /// The two crates each own a copy of the sane defaults; init writes the
    /// ops-core copy while every handler reads the ops-backlog copy, so they
    /// must agree.
    #[test]
    fn sane_defaults_agree_with_backlog_config_defaults() {
        let section = BacklogSection::sane_defaults();
        let defaults = BacklogConfig::default();
        assert_eq!(
            section.default_status.as_deref(),
            Some(defaults.default_status.as_str())
        );
        assert_eq!(section.statuses, Some(defaults.statuses.clone()));
        assert_eq!(section.zero_padded_ids, Some(defaults.zero_padded_ids));
        assert_eq!(
            section.task_prefix.as_deref(),
            Some(defaults.task_prefix.as_str())
        );
        assert_eq!(
            section.backlog_directory.as_deref(),
            Some(defaults.backlog_directory.as_str())
        );
    }

    /// A non-default `.ops.toml` section wins entirely; its unset keys fall
    /// back to the built-in defaults, never to the yml.
    #[test]
    fn ops_toml_section_wins_over_the_yml() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join(".ops.toml"),
            "[backlog]\ndefault_status = \"Triage\"\n",
        )
        .expect("toml");
        std::fs::write(
            dir.path().join("backlog.config.yml"),
            "default_status: \"To Do\"\ntask_prefix: \"ISSUE\"\n",
        )
        .expect("yml");
        let cfg = effective_backlog_config(dir.path()).expect("resolve");
        assert_eq!(cfg.default_status, "Triage", ".ops.toml must win");
        assert_eq!(cfg.task_prefix, "TASK", "unset key = built-in default");
    }

    /// An absent-or-empty section means the yml applies; with no yml either,
    /// the built-in defaults do.
    #[test]
    fn yml_applies_when_the_section_is_absent_or_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join(".ops.toml"),
            "[output]\ntheme = \"classic\"\n",
        )
        .expect("toml");
        std::fs::write(
            dir.path().join("backlog.config.yml"),
            "default_status: \"To Do\"\ntask_prefix: \"ISSUE\"\n",
        )
        .expect("yml");
        let cfg = effective_backlog_config(dir.path()).expect("resolve");
        assert_eq!(cfg.default_status, "To Do");
        assert_eq!(cfg.task_prefix, "ISSUE");

        // An explicitly empty section is still "not configured here".
        std::fs::write(dir.path().join(".ops.toml"), "[backlog]\n").expect("toml");
        let cfg = effective_backlog_config(dir.path()).expect("resolve");
        assert_eq!(
            cfg.default_status, "To Do",
            "empty section must not mask the yml"
        );
    }

    /// Fresh init in an empty workspace: `.ops.toml` with the sane-default
    /// section, plus the tasks tree.
    #[test]
    fn init_creates_ops_toml_section_and_tasks_tree() {
        let dir = tempfile::tempdir().expect("tempdir");
        let text = run_init(dir.path(), false);
        assert!(
            text.contains("Created .ops.toml with a [backlog] section"),
            "got: {text}"
        );
        assert!(text.contains("Created .backlog/tasks/"), "got: {text}");
        let toml = std::fs::read_to_string(dir.path().join(".ops.toml")).expect("read");
        assert!(toml.contains("[backlog]"), "got: {toml}");
        assert!(toml.contains("default_status = \"Triage\""));
        assert!(dir.path().join(".backlog/tasks").is_dir());
        // What init wrote is what the resolver reads back.
        assert_eq!(
            effective_backlog_config(dir.path()).expect("resolve"),
            BacklogConfig::default()
        );
    }

    /// An existing `.ops.toml` gains the section in place — its other
    /// content, formatting, and comments survive byte-for-byte.
    #[test]
    fn init_adds_the_section_to_an_existing_ops_toml_without_touching_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let existing = "# my config\n[output]\ntheme = \"compact\"\n";
        std::fs::write(dir.path().join(".ops.toml"), existing).expect("seed");
        let text = run_init(dir.path(), false);
        assert!(text.contains("Added [backlog] to .ops.toml"), "got: {text}");
        let toml = std::fs::read_to_string(dir.path().join(".ops.toml")).expect("read");
        assert!(
            toml.starts_with("# my config\n"),
            "comment must survive: {toml}"
        );
        assert!(toml.contains("theme = \"compact\""));
        assert!(toml.contains("[backlog]"));
    }

    /// A rerun is a no-op: both messages say "left unchanged", and nothing
    /// is rewritten.
    #[test]
    fn rerun_reports_and_changes_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");
        run_init(dir.path(), false);
        let first = std::fs::read_to_string(dir.path().join(".ops.toml")).expect("read");
        let text = run_init(dir.path(), false);
        assert!(
            text.contains(".ops.toml already configures [backlog], left unchanged"),
            "got: {text}"
        );
        assert!(!text.contains("Created"), "nothing new is created: {text}");
        assert_eq!(
            std::fs::read_to_string(dir.path().join(".ops.toml")).expect("reread"),
            first,
            "the file must be byte-unchanged"
        );
    }

    /// `--backlog.md` writes the yml instead — `.ops.toml` is never created —
    /// and the file carries exactly the five-key subset.
    #[test]
    fn backlog_md_writes_the_yml_and_never_touches_ops_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let text = run_init(dir.path(), true);
        assert!(text.contains("Created backlog.config.yml"), "got: {text}");
        assert!(text.contains("Created .backlog/tasks/"), "got: {text}");
        assert!(
            !dir.path().join(".ops.toml").exists(),
            "--backlog.md must not create .ops.toml"
        );
        let yml = std::fs::read_to_string(dir.path().join("backlog.config.yml")).expect("read");
        assert_eq!(
            yml,
            BacklogConfig::default().to_yaml(),
            "the yml is the five-key subset with the sane defaults"
        );
        // And the rerun refuses to overwrite it.
        std::fs::write(dir.path().join("backlog.config.yml"), "kept").expect("seed");
        let text = run_init(dir.path(), true);
        assert!(
            text.contains("backlog.config.yml already exists, left unchanged"),
            "got: {text}"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("backlog.config.yml")).expect("reread"),
            "kept"
        );
    }

    /// The tasks tree lands under the configured directory — an existing
    /// section's `backlog_directory` steers init even though it writes
    /// nothing.
    #[test]
    fn tasks_tree_honours_an_existing_sections_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join(".ops.toml"),
            "[backlog]\nbacklog_directory = \"tasks-tree\"\n",
        )
        .expect("seed");
        let text = run_init(dir.path(), false);
        assert!(
            dir.path().join("tasks-tree/tasks").is_dir(),
            "tree must honour the section, got: {text}"
        );
    }

    /// An existing `backlog.config.yml` keeps ownership: plain init must not
    /// write a `.ops.toml` section whose sane defaults would shadow the
    /// yml's real values (the section wins at resolve time).
    #[test]
    fn existing_yml_keeps_ownership_of_the_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("backlog.config.yml"),
            "default_status: \"To Do\"\ntask_prefix: \"ISSUE\"\n",
        )
        .expect("seed yml");
        let text = run_init(dir.path(), false);
        assert!(
            text.contains("backlog.config.yml already configures the backlog"),
            "got: {text}"
        );
        assert!(
            !dir.path().join(".ops.toml").exists(),
            "no .ops.toml section may be written beside an existing yml"
        );
        assert!(dir.path().join(".backlog/tasks").is_dir());
        assert_eq!(
            effective_backlog_config(dir.path())
                .expect("resolve")
                .task_prefix,
            "ISSUE",
            "the yml's values must stay in effect"
        );
    }

    /// A malformed `.ops.toml` is a hard error — init never edits through a
    /// document it cannot parse.
    #[test]
    fn malformed_ops_toml_is_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".ops.toml"), "not [ valid toml").expect("seed");
        let mut out = Vec::new();
        let err = run_backlog_init_to(dir.path(), false, &mut out).expect_err("must fail");
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("refusing to overwrite"),
            "error must state the refusal, got: {rendered}"
        );
    }
}
