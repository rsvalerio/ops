//! Plain and JSON output for the backlog commands.
//!
//! The plain shapes are contracts: the code-review skills parse them
//! (`task view --plain`'s first `File:` line is extracted with
//! `sed -n '1s/^File: //p'`, triage greps `task list --plain` rows for
//! `code-review-plan-wave`), so every byte here is pinned by golden tests
//! against the backlog.md CLI v1.51.0's observed output. The JSON envelopes
//! mirror the CLI's `schemaVersion: 1` documents with the same field names
//! and order; they are rendered by hand (not a serde derive) exactly because
//! field order is part of the contract.

use std::io::Write;
use std::path::Path;

use crate::model::TaskDoc;
use crate::store::TaskEntry;

/// `=` × 50, the fixed rule under the view header.
const HEAVY_RULE: &str = "==================================================";
/// `-` × 50, the fixed rule under section headers.
const LIGHT_RULE: &str = "--------------------------------------------------";

/// Priority display rank: higher sorts earlier in `task list` rows.
pub fn priority_rank(priority: Option<&str>) -> u8 {
    match priority.map(str::to_ascii_lowercase).as_deref() {
        Some("critical") => 4,
        Some("high") => 3,
        Some("medium") => 2,
        Some("low") => 1,
        _ => 0,
    }
}

/// First-letter-capitalized form for the view body: `Priority: Low`.
fn priority_capitalized(priority: &str) -> String {
    let mut chars = priority.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + chars.as_str()
    })
}

/// TIME-5 / TASK-2103: an RFC 3339 timestamp is emitted only when the raw
/// frontmatter value actually parses as one — both the `HH:MM` and the
/// `HH:MM:SS` form, via [`crate::cmd::cleanup::parse_frontmatter_date`].
/// A value that does not parse yields `None` (rendered as JSON `null`): the
/// pre-fix string surgery reshaped any scalar into something timestamp-shaped
/// (`"back then"` → `"backTthenZ"`), which fails downstream parsing far from
/// its cause — worse than an honest null.
fn json_date(raw: &str) -> Option<String> {
    crate::cmd::cleanup::parse_frontmatter_date(raw)
        .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

/// `Some(date)` as a JSON string literal, `None` as `null` — the envelope's
/// null convention shared by every optional scalar.
fn opt_jstr_date(date: Option<String>) -> String {
    date.map_or_else(|| "null".to_string(), |d| jstr(&d))
}

/// A JSON string literal, escaped by `serde_json` (infallible for `&str`).
fn jstr(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

/// Append one `"name": value` field line to a hand-rendered JSON object.
fn push_field(s: &mut String, name: &str, value: &str, last: bool) {
    s.push_str("    ");
    s.push_str(&jstr(name));
    s.push_str(": ");
    s.push_str(value);
    if !last {
        s.push(',');
    }
    s.push('\n');
}

/// Relative path of a task file under the workspace root, as the view JSON's
/// `path` field carries it (`.backlog/tasks/task-2069 - ....md`). Falls back
/// to the absolute path when the file does not live under `root`.
fn relative_path(entry: &TaskEntry, root: &Path) -> String {
    entry.path.strip_prefix(root).map_or_else(
        |_| entry.path.to_string_lossy().into_owned(),
        |p| p.to_string_lossy().into_owned(),
    )
}

/// Readiness over the dependency list.
///
/// A task is ready when it is not itself Done and every dependency resolved
/// in `tasks/` is Done (vacuously ready with no dependencies — except a Done
/// task, which is finished, not ready); unresolved dependencies are reported
/// missing, not blocking.
pub struct Readiness {
    pub is_ready: bool,
    pub blocking: Vec<String>,
    pub missing: Vec<String>,
}

/// Compute readiness for `doc` against the other scanned tasks (id → status).
pub fn readiness_of(doc: &TaskDoc, statuses: &dyn Fn(&str) -> Option<String>) -> Readiness {
    let mut blocking = Vec::new();
    let mut missing = Vec::new();
    for dep in &doc.frontmatter.dependencies {
        match statuses(dep) {
            Some(status) if status != "Done" => blocking.push(dep.clone()),
            Some(_) => {}
            None => missing.push(dep.clone()),
        }
    }
    Readiness {
        is_ready: blocking.is_empty() && doc.frontmatter.status != "Done",
        blocking,
        missing,
    }
}

// ---------------------------------------------------------------------------
// task view
// ---------------------------------------------------------------------------

/// Render `task view --plain`. The FIRST line is the absolute file path —
/// the run-wave skill extracts it with `sed -n '1s/^File: //p'`.
///
/// # Errors
///
/// Writing to `out` failed.
pub fn view_plain<W: Write>(
    w: &mut W,
    entry: &TaskEntry,
    readiness: &Readiness,
) -> std::io::Result<()> {
    let fm = &entry.doc.frontmatter;
    writeln!(w, "File: {}", entry.path.display())?;
    writeln!(w)?;
    writeln!(w, "Task {} - {}", fm.id, fm.title)?;
    writeln!(w, "{HEAVY_RULE}")?;
    writeln!(w)?;
    let mark = if fm.status.eq_ignore_ascii_case("Done") {
        "✔"
    } else {
        "○"
    };
    writeln!(w, "Status: {mark} {}", fm.status)?;
    if let Some(priority) = &fm.priority {
        writeln!(w, "Priority: {}", priority_capitalized(priority))?;
    }
    writeln!(w, "Created: {} (UTC)", fm.created_date)?;
    if let Some(updated) = &fm.updated_date {
        writeln!(w, "Updated: {updated} (UTC)")?;
    }
    if !fm.labels.is_empty() {
        writeln!(w, "Labels: {}", fm.labels.join(", "))?;
    }
    writeln!(w)?;
    writeln!(w, "Description:")?;
    writeln!(w, "{LIGHT_RULE}")?;
    match entry.doc.body.description() {
        Some(desc) if !desc.is_empty() => writeln!(w, "{desc}")?,
        _ => writeln!(w, "No description provided")?,
    }
    let ac = entry.doc.body.ac_items();
    if !ac.is_empty() {
        writeln!(w)?;
        writeln!(w, "Acceptance Criteria:")?;
        writeln!(w, "{LIGHT_RULE}")?;
        for (idx, item) in ac.iter().enumerate() {
            let mark = if item.checked { "x" } else { " " };
            writeln!(w, "- [{mark}] #{} {}", idx.saturating_add(1), item.text)?;
        }
    }
    if let Some(notes) = entry.doc.body.notes() {
        if !notes.is_empty() {
            writeln!(w)?;
            writeln!(w, "Implementation Notes:")?;
            writeln!(w, "{LIGHT_RULE}")?;
            writeln!(w, "{notes}")?;
        }
    }
    if let Some(plan) = entry.doc.body.plan() {
        if !plan.is_empty() {
            writeln!(w)?;
            writeln!(w, "Implementation Plan:")?;
            writeln!(w, "{LIGHT_RULE}")?;
            writeln!(w, "{plan}")?;
        }
    }
    // The CLI always renders a DoD section, with the placeholder line when
    // the task carries no items of its own (project-level DoD defaults are
    // still out of scope — see docs/backlog.md).
    let dod = entry.doc.body.dod_items();
    writeln!(w)?;
    writeln!(w, "Definition of Done:")?;
    writeln!(w, "{LIGHT_RULE}")?;
    if dod.is_empty() {
        writeln!(w, "No Definition of Done items defined")?;
    } else {
        for (idx, item) in dod.iter().enumerate() {
            let mark = if item.checked { "x" } else { " " };
            writeln!(w, "- [{mark}] #{} {}", idx.saturating_add(1), item.text)?;
        }
    }
    if !readiness.missing.is_empty() {
        writeln!(w)?;
        writeln!(w, "Missing dependencies: {}", readiness.missing.join(", "))?;
    }
    if !fm.modified_files.is_empty() {
        writeln!(w)?;
        writeln!(w, "Modified files: {}", fm.modified_files.join(", "))?;
    }
    writeln!(w)?;
    Ok(())
}

/// Render `task view --json` — the `schemaVersion: 1` / `kind: task-view`
/// envelope with the CLI's field order.
///
/// # Errors
///
/// Writing to `out` failed.
pub fn view_json<W: Write>(
    w: &mut W,
    entry: &TaskEntry,
    root: &Path,
    readiness: &Readiness,
    resolved: &dyn Fn(&str) -> Option<String>,
) -> std::io::Result<()> {
    let fm = &entry.doc.frontmatter;
    let parent = fm
        .extras
        .iter()
        .find(|(k, _)| k == "parent_task_id")
        .map(|(_, v)| match v {
            crate::model::FmValue::Scalar(s) => s.clone(),
            crate::model::FmValue::List(items) => items.join(","),
        });
    let task_type = fm
        .extras
        .iter()
        .find(|(k, _)| k == "type")
        .map(|(_, v)| match v {
            crate::model::FmValue::Scalar(s) => s.clone(),
            crate::model::FmValue::List(items) => items.join(","),
        });
    let ordinal = fm.ordinal.as_deref().and_then(|o| o.parse::<u64>().ok());

    let mut s = String::with_capacity(1024);
    s.push_str("{\n  \"schemaVersion\": 1,\n  \"kind\": \"task-view\",\n  \"task\": {\n");
    push_scalar_fields(
        &mut s,
        entry,
        root,
        readiness,
        &ExtrasScalars {
            task_type: task_type.as_deref(),
            parent: parent.as_deref(),
            ordinal,
        },
    );
    push_field(&mut s, "dependencies", &str_list(&fm.dependencies), false);
    push_dependency_graph(&mut s, entry, resolved);
    s.push_str("    ");
    s.push_str(&jstr("readiness"));
    s.push_str(": {\n");
    s.push_str("      \"isReady\": ");
    s.push_str(&readiness.is_ready.to_string());
    s.push_str(",\n      \"isBlocked\": ");
    s.push_str(&(!readiness.blocking.is_empty()).to_string());
    s.push_str(",\n      \"blockingDependencies\": ");
    s.push_str(&str_list_at(&readiness.blocking, 6));
    s.push_str(",\n      \"missingDependencies\": ");
    s.push_str(&str_list_at(&readiness.missing, 6));
    s.push_str("\n    },\n");
    push_field(&mut s, "documentation", "[]", false);
    push_field(&mut s, "subtasks", "[]", false);
    // acceptanceCriteria and definitionOfDone arrays — the same item shape
    // (`index` / `text` / `checked`) in both sections.
    push_check_array(&mut s, "acceptanceCriteria", &entry.doc.body.ac_items());
    push_check_array(&mut s, "definitionOfDone", &entry.doc.body.dod_items());
    push_field(
        &mut s,
        "implementationPlan",
        &entry
            .doc
            .body
            .plan()
            .map_or_else(|| "null".to_string(), jstr),
        false,
    );
    push_field(
        &mut s,
        "implementationNotes",
        &entry
            .doc
            .body
            .notes()
            .map_or_else(|| "null".to_string(), jstr),
        false,
    );
    push_field(&mut s, "comments", "[]", false);
    push_field(&mut s, "finalSummary", "null", true);
    s.push_str("  }\n}\n");
    w.write_all(s.as_bytes())
}

/// The extras-derived scalars `view_json` pulls out of the frontmatter —
/// `type`, `parent_task_id`, and the parsed `ordinal`. Grouped so the two
/// adjacent `Option<&str>`s (`task_type`, `parent`) cannot be swapped at a
/// call site by mistake.
struct ExtrasScalars<'a> {
    task_type: Option<&'a str>,
    parent: Option<&'a str>,
    ordinal: Option<u64>,
}

/// The identity-through-description scalar fields of the task-view JSON
/// object, in the CLI's order (everything before `dependencies`).
fn push_scalar_fields(
    s: &mut String,
    entry: &TaskEntry,
    root: &Path,
    readiness: &Readiness,
    extras: &ExtrasScalars<'_>,
) {
    let fm = &entry.doc.frontmatter;
    let ac = entry.doc.body.ac_items();
    let done = ac.iter().filter(|item| item.checked).count();
    push_field(s, "id", &jstr(&fm.id), false);
    push_field(s, "title", &jstr(&fm.title), false);
    push_field(s, "status", &jstr(&fm.status), false);
    push_field(s, "type", &opt_str(extras.task_type), false);
    push_field(s, "priority", &opt_str(fm.priority.as_deref()), false);
    push_field(s, "project", "null", false);
    push_field(s, "assignees", &str_list(&fm.assignees), false);
    push_field(s, "reporter", "null", false);
    push_field(s, "labels", &str_list(&fm.labels), false);
    push_field(s, "milestone", "null", false);
    push_field(s, "parentTaskId", &opt_str(extras.parent), false);
    push_field(s, "acceptanceCriteriaCompleted", &done.to_string(), false);
    push_field(s, "acceptanceCriteriaCount", &ac.len().to_string(), false);
    push_field(s, "references", "[]", false);
    push_field(s, "modifiedFiles", &str_list(&fm.modified_files), false);
    push_field(
        s,
        "ordinal",
        &extras
            .ordinal
            .map_or_else(|| "null".to_string(), |o| o.to_string()),
        false,
    );
    push_field(
        s,
        "createdAt",
        &opt_jstr_date(json_date(&fm.created_date)),
        false,
    );
    push_field(
        s,
        "updatedAt",
        &opt_jstr_date(fm.updated_date.as_ref().and_then(|d| json_date(d))),
        false,
    );
    push_field(s, "dueDate", "null", false);
    push_field(s, "isReady", &readiness.is_ready.to_string(), false);
    push_field(s, "path", &jstr(&relative_path(entry, root)), false);
    push_field(
        s,
        "description",
        &entry
            .doc
            .body
            .description()
            .map_or_else(|| "null".to_string(), jstr),
        false,
    );
}

/// Append the `dependencyGraph` sub-object to a hand-rendered task JSON.
fn push_dependency_graph(
    s: &mut String,
    entry: &TaskEntry,
    resolved: &dyn Fn(&str) -> Option<String>,
) {
    let fm = &entry.doc.frontmatter;
    s.push_str("    ");
    s.push_str(&jstr("dependencyGraph"));
    s.push_str(": {\n      \"root\": ");
    s.push_str(&jstr(&fm.id));
    s.push_str(",\n      \"nodes\": [\n");
    let mut push_node = |id: &str, title: &str, status: &str, last: bool| {
        let completed = status.eq_ignore_ascii_case("Done");
        let state = if completed { "resolved" } else { "pending" };
        s.push_str("        {\n          \"id\": ");
        s.push_str(&jstr(id));
        s.push_str(",\n          \"title\": ");
        s.push_str(&jstr(title));
        s.push_str(",\n          \"status\": ");
        s.push_str(&jstr(status));
        s.push_str(",\n          \"state\": ");
        s.push_str(&jstr(state));
        s.push_str(",\n          \"completed\": ");
        s.push_str(&completed.to_string());
        s.push_str(
            ",\n          \"dependencyDepth\": 0,\n          \"dependentDepth\": 0\n        }",
        );
        if !last {
            s.push(',');
        }
        s.push('\n');
    };
    push_node(&fm.id, &fm.title, &fm.status, fm.dependencies.is_empty());
    let last_dep = fm.dependencies.len().saturating_sub(1);
    for (idx, dep) in fm.dependencies.iter().enumerate() {
        let status = resolved(dep).unwrap_or_default();
        push_node(dep, "", &status, idx == last_dep);
    }
    s.push_str("      ],\n");
    if fm.dependencies.is_empty() {
        s.push_str("      \"edges\": []\n    },\n");
    } else {
        s.push_str("      \"edges\": [\n");
        let last_idx = fm.dependencies.len().saturating_sub(1);
        for (idx, dep) in fm.dependencies.iter().enumerate() {
            s.push_str("        { \"from\": ");
            s.push_str(&jstr(dep));
            s.push_str(", \"to\": ");
            s.push_str(&jstr(&fm.id));
            s.push_str(" }");
            if idx == last_idx {
                s.push('\n');
            } else {
                s.push_str(",\n");
            }
        }
        s.push_str("      ]\n    },\n");
    }
}

/// Append the `acceptanceCriteria` array to a hand-rendered task JSON.
fn push_check_array(s: &mut String, name: &str, ac: &[crate::model::AcItem]) {
    s.push_str("    ");
    s.push_str(&jstr(name));
    s.push_str(": [\n");
    for (idx, item) in ac.iter().enumerate() {
        let last = idx == ac.len().saturating_sub(1);
        s.push_str("      {\n        \"index\": ");
        s.push_str(&idx.saturating_add(1).to_string());
        s.push_str(",\n        \"text\": ");
        s.push_str(&jstr(&item.text));
        s.push_str(",\n        \"checked\": ");
        s.push_str(&item.checked.to_string());
        s.push_str("\n      }");
        if !last {
            s.push(',');
        }
        s.push('\n');
    }
    s.push_str("    ],\n");
}

fn opt_str(value: Option<&str>) -> String {
    value.map_or_else(|| "null".to_string(), jstr)
}

fn str_list(items: &[String]) -> String {
    str_list_at(items, 4)
}

/// Pretty JSON array: one item per line, items at `field_indent` + 2, the
/// closing bracket back at `field_indent` — matching the CLI's rendering.
fn str_list_at(items: &[String], field_indent: usize) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    let pad = " ".repeat(field_indent);
    let item_pad = " ".repeat(field_indent.saturating_add(2));
    let rendered: Vec<String> = items
        .iter()
        .map(|i| format!("{item_pad}{}", jstr(i)))
        .collect();
    format!("[\n{}\n{pad}]", rendered.join(",\n"))
}

// ---------------------------------------------------------------------------
// task list
// ---------------------------------------------------------------------------

/// Group entries by status (config statuses first, then any unknown
/// statuses in first-seen order) and order each group by priority rank
/// descending, then id ascending — the ordering both listings share.
fn order_for_listing<'a>(
    entries: &'a [TaskEntry],
    statuses: &[String],
) -> Vec<(String, Vec<&'a TaskEntry>)> {
    let mut order: Vec<String> = statuses.to_vec();
    for entry in entries {
        let status = entry.doc.frontmatter.status.clone();
        if !order.contains(&status) {
            order.push(status);
        }
    }
    let mut grouped = Vec::new();
    for status in &order {
        let mut group: Vec<&TaskEntry> = entries
            .iter()
            .filter(|e| &e.doc.frontmatter.status == status)
            .collect();
        if group.is_empty() {
            continue;
        }
        group.sort_by(|a, b| {
            priority_rank(b.doc.frontmatter.priority.as_deref())
                .cmp(&priority_rank(a.doc.frontmatter.priority.as_deref()))
                .then_with(|| a.doc.frontmatter.id.cmp(&b.doc.frontmatter.id))
        });
        grouped.push((status.clone(), group));
    }
    grouped
}

/// Render `task list --plain`: one `Status:` header per status (config
/// order), rows `  [HIGH] TASK-1656 - title (ac: 5/5)` sorted by priority
/// rank then id.
///
/// # Errors
///
/// Writing to `out` failed.
pub fn list_plain<W: Write>(
    w: &mut W,
    entries: &[TaskEntry],
    statuses: &[String],
) -> std::io::Result<()> {
    for (status, group) in &order_for_listing(entries, statuses) {
        writeln!(w, "{status}:")?;
        for entry in group {
            let fm = &entry.doc.frontmatter;
            let ac = entry.doc.body.ac_items();
            let done = ac.iter().filter(|item| item.checked).count();
            // Bracket tag: the priority when present; a type (stored
            // lowercase, e.g. `enhancement`) fills the slot for unprioritized
            // typed tasks — the CLI's observed shape for the one typed file
            // in the corpus.
            let tag = match (&fm.priority, fm.extra_scalar("type")) {
                (Some(p), _) => format!("[{}] ", p.to_ascii_uppercase()),
                (None, Some(t)) => format!("[{t}] "),
                (None, None) => String::new(),
            };
            // The CLI omits the ac suffix entirely for tasks with no
            // criteria — `(ac: 0/0)` never appears in its output.
            let ac_suffix = if ac.is_empty() {
                String::new()
            } else {
                format!(" (ac: {}/{})", done, ac.len())
            };
            writeln!(w, "  {tag}{} - {}{}", fm.id, fm.title, ac_suffix)?;
        }
        writeln!(w)?;
    }
    Ok(())
}

/// Render `task list --json`: the `kind: task-list` envelope with one task
/// object per row.
///
/// Rows carry the view's field set minus the heavy bodies, ordered by the
/// same status-group / priority / id ordering as the plain listing.
///
/// # Errors
///
/// Writing to `out` failed.
pub fn list_json<W: Write>(
    w: &mut W,
    entries: &[TaskEntry],
    statuses: &[String],
) -> std::io::Result<()> {
    let ordered: Vec<&TaskEntry> = order_for_listing(entries, statuses)
        .into_iter()
        .flat_map(|(_, group)| group)
        .collect();
    // The same lookup `run_view` hands `readiness_of`, so list and view
    // agree: a dependency missing from the scanned set (moved to
    // `completed/` or archived) does not block, only an unresolved non-Done
    // dependency does.
    let status_of = |id: &str| -> Option<String> {
        entries
            .iter()
            .find(|e| e.doc.frontmatter.id == id)
            .map(|e| e.doc.frontmatter.status.clone())
    };
    let mut s = String::with_capacity(256usize.saturating_mul(ordered.len().saturating_add(1)));
    s.push_str("{\n  \"schemaVersion\": 1,\n  \"kind\": \"task-list\",\n  \"tasks\": [\n");
    for (idx, entry) in ordered.iter().enumerate() {
        let last = idx == ordered.len().saturating_sub(1);
        let fm = &entry.doc.frontmatter;
        let ac = entry.doc.body.ac_items();
        let done = ac.iter().filter(|item| item.checked).count();
        let ordinal = fm.ordinal.as_deref().and_then(|o| o.parse::<u64>().ok());
        let ready = readiness_of(&entry.doc, &status_of).is_ready;
        s.push_str("    {\n");
        fn push_row_field(s: &mut String, name: &str, value: &str, last: bool) {
            s.push_str("      ");
            s.push_str(&jstr(name));
            s.push_str(": ");
            s.push_str(value);
            if !last {
                s.push(',');
            }
            s.push('\n');
        }
        push_row_field(&mut s, "id", &jstr(&fm.id), false);
        push_row_field(&mut s, "title", &jstr(&fm.title), false);
        push_row_field(&mut s, "status", &jstr(&fm.status), false);
        push_row_field(&mut s, "type", &opt_str(fm.extra_scalar("type")), false);
        push_row_field(&mut s, "priority", &opt_str(fm.priority.as_deref()), false);
        push_row_field(&mut s, "project", "null", false);
        push_row_field(&mut s, "assignees", &str_list_at(&fm.assignees, 6), false);
        push_row_field(&mut s, "reporter", "null", false);
        push_row_field(&mut s, "labels", &str_list_at(&fm.labels, 6), false);
        push_row_field(&mut s, "milestone", "null", false);
        push_row_field(
            &mut s,
            "parentTaskId",
            &opt_str(fm.extra_scalar("parent_task_id")),
            false,
        );
        push_row_field(
            &mut s,
            "acceptanceCriteriaCompleted",
            &done.to_string(),
            false,
        );
        push_row_field(
            &mut s,
            "acceptanceCriteriaCount",
            &ac.len().to_string(),
            false,
        );
        push_row_field(&mut s, "references", "[]", false);
        push_row_field(
            &mut s,
            "modifiedFiles",
            &str_list_at(&fm.modified_files, 6),
            false,
        );
        push_row_field(
            &mut s,
            "ordinal",
            &ordinal.map_or_else(|| "null".to_string(), |o| o.to_string()),
            false,
        );
        push_row_field(
            &mut s,
            "createdAt",
            &opt_jstr_date(json_date(&fm.created_date)),
            false,
        );
        push_row_field(
            &mut s,
            "updatedAt",
            &opt_jstr_date(fm.updated_date.as_ref().and_then(|d| json_date(d))),
            false,
        );
        push_row_field(&mut s, "dueDate", "null", false);
        push_row_field(&mut s, "isReady", &ready.to_string(), true);
        s.push_str("    }");
        if !last {
            s.push(',');
        }
        s.push('\n');
    }
    s.push_str("  ]\n}\n");
    w.write_all(s.as_bytes())
}

// ---------------------------------------------------------------------------
// search
// ---------------------------------------------------------------------------

/// One scored search hit.
pub struct SearchHit<'a> {
    pub entry: &'a TaskEntry,
    pub score: f64,
}

/// Deterministic keyword scoring (ours — scores are explicitly not part of
/// the backlog.md contract).
///
/// Exact id match scores 1.000; otherwise each whitespace token contributes
/// its first field hit (id .35 / title .30 / labels .15 / description .10 /
/// notes .05) and the token contributions are averaged. Sort: score
/// descending, then id ascending.
#[must_use = "scoring is pure; the caller sorts and renders"]
pub fn score_entry(entry: &TaskEntry, query: &str) -> f64 {
    let fm = &entry.doc.frontmatter;
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return 0.0;
    }
    if fm.id.to_ascii_lowercase() == q {
        return 1.0;
    }
    let mut total = 0.0f64;
    let mut tokens = 0usize;
    for token in q.split_whitespace() {
        tokens = tokens.saturating_add(1);
        if fm.id.to_ascii_lowercase().contains(token) {
            total += 0.35;
        } else if fm.title.to_ascii_lowercase().contains(token) {
            total += 0.30;
        } else if fm
            .labels
            .iter()
            .any(|l| l.to_ascii_lowercase().contains(token))
        {
            total += 0.15;
        } else if entry
            .doc
            .body
            .description()
            .is_some_and(|d| d.to_ascii_lowercase().contains(token))
        {
            total += 0.10;
        } else if entry
            .doc
            .body
            .notes()
            .is_some_and(|n| n.to_ascii_lowercase().contains(token))
        {
            total += 0.05;
        }
    }
    let Some(count) = u32::try_from(tokens).ok() else {
        // A query with more than u32::MAX tokens cannot exist; the guard
        // only keeps the conversion silent-lossless.
        return total;
    };
    total / f64::from(count)
}

/// Rank hits: score descending, numeric id ascending.
pub fn sort_hits(hits: &mut [SearchHit<'_>]) {
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.entry.doc.frontmatter.id.cmp(&b.entry.doc.frontmatter.id))
    });
}

/// Render `search --plain`: `Tasks:` header, rows
/// `  TASK-1766 - title (Done) [LOW] [score 0.671]`; the score suffix only
/// when a query was given.
///
/// # Errors
///
/// Writing to `out` failed.
pub fn search_plain<W: Write>(
    w: &mut W,
    hits: &[SearchHit<'_>],
    with_score: bool,
) -> std::io::Result<()> {
    writeln!(w, "Tasks:")?;
    for hit in hits {
        let fm = &hit.entry.doc.frontmatter;
        let tag = fm
            .priority
            .as_ref()
            .map(|p| format!(" [{}]", p.to_ascii_uppercase()))
            .unwrap_or_default();
        let score = if with_score {
            format!(" [score {:.3}]", hit.score)
        } else {
            String::new()
        };
        writeln!(w, "  {} - {} ({}){tag}{score}", fm.id, fm.title, fm.status)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TaskDoc;

    fn doc_from(src: &str) -> TaskEntry {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("task-0001 - x.md");
        // Leak the tempdir path's file only for the lifetime of the test:
        // TaskEntry needs an owned path, and the tempdir itself is dropped
        // harmlessly after (nothing reads the file back).
        std::mem::forget(dir);
        TaskEntry {
            path,
            doc: TaskDoc::parse(src).expect("parse"),
        }
    }

    const VIEW_SAMPLE: &str = "\
---
id: TASK-2069
title: 'DUP-3: scaffolds'
status: Done
assignee: []
created_date: '2026-08-29 18:21'
updated_date: '2026-08-31 17:38'
labels:
  - code-review-rust
dependencies: []
modified_files:
  - crates/foo/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/foo/src/lib.rs:42`
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 first
- [ ] #2 second
<!-- AC:END -->
";

    #[test]
    fn view_plain_first_line_is_the_absolute_path() {
        let entry = doc_from(VIEW_SAMPLE);
        let mut out = Vec::new();
        let readiness = Readiness {
            is_ready: true,
            blocking: vec![],
            missing: vec![],
        };
        view_plain(&mut out, &entry, &readiness).expect("render");
        let text = String::from_utf8(out).expect("utf8");
        let mut lines = text.lines();
        let first = lines.next().expect("first line");
        let path = first.strip_prefix("File: ").expect("File: prefix");
        assert!(
            path.contains("task-0001"),
            "path must name the file: {path}"
        );
        assert!(path.starts_with('/'), "path must be absolute: {path}");
        assert_eq!(lines.next(), Some(""), "blank line after File:");
        assert_eq!(lines.next(), Some("Task TASK-2069 - DUP-3: scaffolds"));
        assert_eq!(lines.next().map(str::len), Some(50), "= rule is 50 wide");
        assert!(text.contains("Status: ✔ Done"));
        assert!(text.contains("Priority: Low"));
        assert!(text.contains("Created: 2026-08-29 18:21 (UTC)"));
        assert!(text.contains("Updated: 2026-08-31 17:38 (UTC)"));
        assert!(text.contains("Labels: code-review-rust"));
        assert!(text.contains("Modified files: crates/foo/src/lib.rs"));
        assert!(text.contains("- [x] #1 first"));
    }

    #[test]
    fn dod_renders_in_plain_and_json_when_the_task_has_items() {
        let src = format!(
            "{VIEW_SAMPLE}\n## Definition of Done\n<!-- DOD:BEGIN -->\n- [x] #1 gate one\n- [ ] #2 gate two\n<!-- DOD:END -->\n"
        );
        let entry = doc_from(&src);
        let readiness = Readiness {
            is_ready: true,
            blocking: vec![],
            missing: vec![],
        };

        let mut out = Vec::new();
        view_plain(&mut out, &entry, &readiness).expect("render");
        let text = String::from_utf8(out).expect("utf8");
        assert!(text.contains("Definition of Done:"));
        assert!(text.contains("- [x] #1 gate one"));
        assert!(text.contains("- [ ] #2 gate two"));
        assert!(!text.contains("No Definition of Done items defined"));

        let mut out = Vec::new();
        view_json(&mut out, &entry, Path::new("/ws"), &readiness, &|_| None).expect("render");
        let text = String::from_utf8(out).expect("utf8");
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid json");
        assert_eq!(
            value["task"]["definitionOfDone"],
            serde_json::json!([
                { "index": 1, "text": "gate one", "checked": true },
                { "index": 2, "text": "gate two", "checked": false },
            ])
        );
    }

    #[test]
    fn dod_placeholder_stands_for_a_task_without_items() {
        let entry = doc_from(VIEW_SAMPLE);
        let readiness = Readiness {
            is_ready: true,
            blocking: vec![],
            missing: vec![],
        };
        let mut out = Vec::new();
        view_plain(&mut out, &entry, &readiness).expect("render");
        let text = String::from_utf8(out).expect("utf8");
        assert!(text.contains("No Definition of Done items defined"));
    }

    #[test]
    fn non_done_status_uses_the_open_marker() {
        let src = VIEW_SAMPLE.replacen("status: Done", "status: In Progress", 1);
        let entry = doc_from(&src);
        let mut out = Vec::new();
        let readiness = Readiness {
            is_ready: true,
            blocking: vec![],
            missing: vec![],
        };
        view_plain(&mut out, &entry, &readiness).expect("render");
        let text = String::from_utf8(out).expect("utf8");
        assert!(text.contains("Status: ○ In Progress"));
    }

    #[test]
    fn view_json_envelope_field_order() {
        let entry = doc_from(VIEW_SAMPLE);
        let readiness = Readiness {
            is_ready: true,
            blocking: vec![],
            missing: vec![],
        };
        let mut out = Vec::new();
        view_json(&mut out, &entry, Path::new("/ws"), &readiness, &|_| None).expect("render");
        let text = String::from_utf8(out).expect("utf8");
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid json");
        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["kind"], "task-view");
        assert_eq!(value["task"]["id"], "TASK-2069");
        assert_eq!(value["task"]["createdAt"], "2026-08-29T18:21:00Z");
        assert_eq!(value["task"]["priority"], "low");
        assert_eq!(
            value["task"]["modifiedFiles"],
            serde_json::json!(["crates/foo/src/lib.rs"])
        );
        assert_eq!(value["task"]["acceptanceCriteriaCompleted"], 1);
        assert_eq!(value["task"]["acceptanceCriteriaCount"], 2);
        // Field order pins: id before title before status.
        let id_pos = text.find("\"id\"").expect("id");
        let title_pos = text.find("\"title\"").expect("title");
        let status_pos = text.find("\"status\"").expect("status");
        assert!(id_pos < title_pos && title_pos < status_pos);
    }

    /// One dependency: the graph's node array must close without a trailing
    /// comma — strict parsers (`serde_json`) reject `},]`.
    #[test]
    fn view_json_dependency_graph_with_edges_parses() {
        let src = VIEW_SAMPLE.replacen("dependencies: []", "dependencies:\n  - TASK-0001", 1);
        let entry = doc_from(&src);
        let readiness = Readiness {
            is_ready: true,
            blocking: vec![],
            missing: vec![],
        };
        let mut out = Vec::new();
        view_json(&mut out, &entry, Path::new("/ws"), &readiness, &|_| None).expect("render");
        let text = String::from_utf8(out).expect("utf8");
        let value: serde_json::Value =
            serde_json::from_str(&text).unwrap_or_else(|e| panic!("must be valid json: {e}"));
        let graph = &value["task"]["dependencyGraph"];
        assert_eq!(
            graph["nodes"].as_array().map(Vec::len),
            Some(2),
            "the task plus its dependency, got: {graph}"
        );
        assert_eq!(graph["edges"].as_array().map(Vec::len), Some(1));
    }

    #[test]
    fn json_date_handles_seconds_and_minutes() {
        assert_eq!(
            json_date("2026-08-29 18:21").as_deref(),
            Some("2026-08-29T18:21:00Z")
        );
        assert_eq!(
            json_date("2026-04-10 07:15:00").as_deref(),
            Some("2026-04-10T07:15:00Z")
        );
    }

    /// TIME-5 / TASK-2103: an unparseable frontmatter date must not be
    /// reshaped into a pseudo-timestamp — pre-fix, `"back then"` rendered as
    /// `"backTthenZ"` in the JSON envelopes' `createdAt`/`updatedAt`.
    #[test]
    fn json_date_rejects_unparseable_dates() {
        assert_eq!(json_date("back then"), None);
    }

    #[test]
    fn list_plain_groups_and_orders_rows() {
        let a = doc_from(
            "---\nid: TASK-0001\ntitle: 'low one'\nstatus: To Do\nassignee: []\ncreated_date: '2026-01-01 00:00'\nlabels: []\ndependencies: []\npriority: low\n---\n",
        );
        let b = doc_from(
            "---\nid: TASK-0002\ntitle: 'high one'\nstatus: To Do\nassignee: []\ncreated_date: '2026-01-01 00:00'\nlabels: []\ndependencies: []\npriority: high\n---\n",
        );
        let c = doc_from(
            "---\nid: TASK-0003\ntitle: 'done one'\nstatus: Done\nassignee: []\ncreated_date: '2026-01-01 00:00'\nlabels: []\ndependencies: []\n---\n",
        );
        let mut out = Vec::new();
        list_plain(
            &mut out,
            &[a, b, c],
            &[
                "Triage".to_string(),
                "To Do".to_string(),
                "Done".to_string(),
            ],
        )
        .expect("render");
        let text = String::from_utf8(out).expect("utf8");
        let expected = "\
To Do:
  [HIGH] TASK-0002 - high one
  [LOW] TASK-0001 - low one

Done:
  TASK-0003 - done one

";
        assert_eq!(text, expected);
    }

    #[test]
    fn score_exact_id_beats_title_token_hits() {
        let hit = doc_from(VIEW_SAMPLE);
        let exact = score_entry(&hit, "task-2069");
        let token = score_entry(&hit, "scaffolds");
        assert!(
            (exact - 1.0).abs() < 1e-12,
            "exact id scores 1.0, got {exact}"
        );
        assert!(
            (token - 0.30).abs() < 1e-9,
            "title token scores 0.30, got {token}"
        );
        assert!(exact > token);
    }

    #[test]
    fn search_plain_row_shape() {
        let entry = doc_from(VIEW_SAMPLE);
        let hits = [SearchHit {
            entry: &entry,
            score: 0.671,
        }];
        let mut out = Vec::new();
        search_plain(&mut out, &hits, true).expect("render");
        let text = String::from_utf8(out).expect("utf8");
        assert_eq!(
            text,
            "Tasks:\n  TASK-2069 - DUP-3: scaffolds (Done) [LOW] [score 0.671]\n"
        );
        // No query → no score suffix.
        let mut out = Vec::new();
        search_plain(&mut out, &hits, false).expect("render");
        let text = String::from_utf8(out).expect("utf8");
        assert_eq!(
            text,
            "Tasks:\n  TASK-2069 - DUP-3: scaffolds (Done) [LOW]\n"
        );
    }
}
