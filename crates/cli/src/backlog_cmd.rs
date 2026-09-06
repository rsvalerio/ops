//! `ops backlog …` dispatch: loads the backlog config, opens the store,
//! and maps the clap actions onto the ops-backlog handlers.

use std::path::Path;

use ops_backlog::cmd;
use ops_backlog::config::BacklogConfig;
use ops_backlog::store::Store;

use crate::args::{BacklogAction, BacklogTaskAction};

/// Resolve the workspace root (cwd), load `backlog.config.yml`, open the
/// `.backlog` store, and run the action with stdout.
///
/// # Errors
///
/// The cwd is unreadable, the config file is present but unparseable, the
/// `.backlog/tasks` tree is missing (the error names it), or a handler
/// failed — all bubble as anyhow context for `ops: error: …`.
pub fn run_backlog(cwd: &Path, action: BacklogAction) -> anyhow::Result<()> {
    let cfg = BacklogConfig::load(cwd)?;
    let backlog_root = cwd.join(&cfg.backlog_directory);
    let store = Store::open(&backlog_root)?;
    match action {
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
    }
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
        parent,
        clear_parent,
        add_dep,
        remove_dep,
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
                plain: true,
                json,
            };
            cmd::run_list(store, cfg, &opts, &mut std::io::stdout())
        }
        BacklogTaskAction::View {
            task_id,
            plain,
            json,
        } => {
            let opts = cmd::ViewOptions {
                task_id,
                plain,
                json,
            };
            cmd::run_view(store, &opts, cwd, &mut std::io::stdout())
        }
    }
}
