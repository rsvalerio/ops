//! The Rust foundation: the shared config every Rust repo starts from.
//!
//! ops is the single source for `clippy.toml`, `deny.toml`, `rustfmt.toml`,
//! `.config/nextest.toml`, the `mise.toml` tool pins and the
//! `[workspace.lints]` policy. The
//! templates are embedded in the binary, [`scaffold`] writes them into a repo
//! (`ops init --rust`), and [`check`] reports where the repo has drifted from
//! the running ops version's copy (`ops init --rust --check`). Updates ship
//! with ops releases; there are no vendored copies to sync.
//!
//! The check is semantic (see [`compare`]): a template is a baseline, so
//! additions such as `advisories.ignore` entries are the repo's own, while a
//! changed or missing baseline key is drift unless a waiver in `.ops.toml`
//! records why.

pub mod compare;

use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::Path;

use anyhow::Context;
use indexmap::IndexMap;
use ops_core::config::atomic_write;
use toml::Value;

pub use compare::{Drift, Rule};

/// A foundation config file written verbatim at a fixed repo-relative path.
#[derive(Debug, Clone, Copy)]
pub struct ConfigFile {
    pub path: &'static str,
    pub template: &'static str,
}

/// The whole-file templates, in the order they are written and checked.
pub const FILES: [ConfigFile; 5] = [
    ConfigFile {
        path: "clippy.toml",
        template: include_str!("../templates/clippy.toml"),
    },
    ConfigFile {
        path: "deny.toml",
        template: include_str!("../templates/deny.toml"),
    },
    ConfigFile {
        path: "rustfmt.toml",
        template: include_str!("../templates/rustfmt.toml"),
    },
    ConfigFile {
        path: ".config/nextest.toml",
        template: include_str!("../templates/nextest.toml"),
    },
    ConfigFile {
        path: "mise.toml",
        template: include_str!("../templates/mise.toml"),
    },
];

/// The lint policy, written with `[lints.*]` headers. [`scaffold`] rewrites
/// them to `[workspace.lints.*]` for a workspace root.
pub const LINTS_TEMPLATE: &str = include_str!("../templates/lints.toml");

const MANIFEST: &str = "Cargo.toml";
const MISE: &str = "mise.toml";

/// What [`scaffold`] did with one target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The file did not exist and was written.
    Created,
    /// `--force` replaced what was there.
    Replaced,
    /// Already present and left alone (no `--force`).
    Kept,
    /// The lint policy or a member's opt-in was added to a manifest.
    Added,
}

/// One line of the scaffold report: `target` is repo-relative, with a
/// `:<table>` suffix for a manifest edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    pub target: String,
    pub outcome: Outcome,
}

/// The result of [`check`].
#[derive(Debug, Default)]
pub struct Report {
    /// Unwaived drift: the check fails when this is non-empty.
    pub drift: Vec<Drift>,
    /// Drift covered by a waiver, with the waiver's reason.
    pub waived: Vec<(Drift, String)>,
    /// Waivers that matched nothing: stale, and hiding nothing today.
    pub unused_waivers: Vec<String>,
}

/// Where the lint policy lives in the root manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// `[workspace.lints]`, with every member opting in.
    Workspace,
    /// A single package: `[lints]` directly.
    Package,
}

impl Shape {
    const fn lints_path(self) -> &'static [&'static str] {
        match self {
            Self::Workspace => &["workspace", "lints"],
            Self::Package => &["lints"],
        }
    }

    const fn lints_location(self) -> &'static str {
        match self {
            Self::Workspace => "Cargo.toml:workspace.lints",
            Self::Package => "Cargo.toml:lints",
        }
    }
}

/// The root manifest, read once and shared by scaffold and check.
struct Root {
    text: String,
    shape: Shape,
    rust_version: Option<String>,
    /// Workspace members relative to the root, excluding the root itself.
    members: Vec<String>,
}

impl Root {
    fn load(root: &Path) -> anyhow::Result<Self> {
        let path = root.join(MANIFEST);
        let text = std::fs::read_to_string(&path).with_context(|| {
            format!(
                "reading {} (run from the root of a Rust project)",
                path.display()
            )
        })?;
        let value: toml::Table =
            toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        let shape = if value.contains_key("workspace") {
            Shape::Workspace
        } else {
            Shape::Package
        };
        let rust_version = value
            .get("workspace")
            .and_then(|w| w.get("package")?.get("rust-version"))
            .or_else(|| value.get("package")?.get("rust-version"))
            .and_then(Value::as_str)
            .filter(|v| is_plain_version(v))
            .map(str::to_owned);
        let members = if shape == Shape::Workspace {
            workspace_members(root, &text, value.contains_key("package"))
        } else {
            Vec::new()
        };
        Ok(Self {
            text,
            shape,
            rust_version,
            members,
        })
    }
}

/// Resolve members through the same glob expander the about providers use,
/// dropping any entry that would escape the workspace (SEC-14). A root that
/// is also a package is its own member and opts in like any other.
fn workspace_members(root: &Path, text: &str, root_is_package: bool) -> Vec<String> {
    let mut members = match ops_cargo_toml::CargoToml::parse(text) {
        Ok(manifest) => ops_about_rust::resolved_workspace_members(&manifest, root)
            .into_iter()
            .filter(|m| ops_about_rust::member_path_is_workspace_safe_or_warn(m, "rust-foundation"))
            .collect(),
        Err(err) => {
            tracing::warn!(error = %err, "could not resolve workspace members");
            Vec::new()
        }
    };
    if root_is_package {
        members.insert(0, ".".to_owned());
    }
    members
}

/// Only digits and dots reach the rendered `msrv = "..."` line, so a hostile
/// `rust-version` cannot inject TOML into `clippy.toml`.
fn is_plain_version(v: &str) -> bool {
    !v.is_empty()
        && v.split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

/// The bytes written for `file`: the template, plus `msrv` for clippy when
/// the root declares a `rust-version`. `msrv` is the repo's own value, so it
/// is not part of the baseline the check compares.
fn render(file: &ConfigFile, rust_version: Option<&str>) -> String {
    let mut out = file.template.to_owned();
    if file.path == "clippy.toml" {
        if let Some(version) = rust_version {
            out.push_str(
                "\n# Keep equal to `rust-version` in the root Cargo.toml so\n\
                 # `clippy::incompatible_msrv` fails on any standard-library call newer than\n\
                 # the declared floor. This key is the repo's own, not part of the baseline.\n\
                 msrv = \"",
            );
            out.push_str(version);
            out.push_str("\"\n");
        }
    }
    out
}

fn render_lints(shape: Shape) -> String {
    match shape {
        Shape::Package => LINTS_TEMPLATE.to_owned(),
        Shape::Workspace => LINTS_TEMPLATE.replace("\n[lints.", "\n[workspace.lints."),
    }
}

/// Write the foundation into the Rust project at `root`.
///
/// Existing files, an existing lint table and existing member `[lints]`
/// tables are kept unless `force` is set; with `force` the files and the
/// lint table are replaced by the templates (member opt-ins are only ever
/// added, never rewritten).
///
/// # Errors
///
/// If `root` has no parseable `Cargo.toml`, `.config` is a symlink, or any
/// write fails.
pub fn scaffold(root: &Path, force: bool) -> anyhow::Result<Vec<Written>> {
    let manifest = Root::load(root)?;
    let mut written = Vec::new();
    for file in &FILES {
        let outcome = write_file(
            root,
            file.path,
            &render(file, manifest.rust_version.as_deref()),
            force,
        )?;
        written.push(Written {
            target: file.path.to_owned(),
            outcome,
        });
    }
    written.push(scaffold_lints(root, &manifest, force)?);
    for member in &manifest.members {
        if let Some(w) = scaffold_member_opt_in(root, member)? {
            written.push(w);
        }
    }
    Ok(written)
}

fn write_file(root: &Path, rel: &str, content: &str, force: bool) -> anyhow::Result<Outcome> {
    let path = root.join(rel);
    if let Some(parent) = Path::new(rel)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        ensure_real_dir(&root.join(parent))?;
    }
    if force {
        let existed = path.symlink_metadata().is_ok();
        atomic_write(&path, content.as_bytes()).with_context(|| format!("writing {rel}"))?;
        return Ok(if existed {
            Outcome::Replaced
        } else {
            Outcome::Created
        });
    }
    // `create_new` is the existence check: it fails on anything already at
    // the path, a dangling symlink included, so nothing is clobbered.
    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(mut f) => {
            f.write_all(content.as_bytes())
                .with_context(|| format!("writing {rel}"))?;
            f.sync_all().with_context(|| format!("syncing {rel}"))?;
            Ok(Outcome::Created)
        }
        Err(e) if e.kind() == ErrorKind::AlreadyExists => Ok(Outcome::Kept),
        Err(e) => Err(e).with_context(|| format!("creating {rel}")),
    }
}

/// Create `dir` if needed, refusing a symlink so a write cannot be steered
/// out of the workspace through it.
fn ensure_real_dir(dir: &Path) -> anyhow::Result<()> {
    match dir.symlink_metadata() {
        Ok(meta) if meta.file_type().is_symlink() => {
            anyhow::bail!(
                "{} is a symlink; refusing to write through it",
                dir.display()
            )
        }
        Ok(_) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => {
            std::fs::create_dir(dir).with_context(|| format!("creating {}", dir.display()))
        }
        Err(e) => Err(e).with_context(|| format!("inspecting {}", dir.display())),
    }
}

fn scaffold_lints(root: &Path, manifest: &Root, force: bool) -> anyhow::Result<Written> {
    let target = manifest.shape.lints_location().to_owned();
    let mut doc: toml_edit::DocumentMut = manifest.text.parse().context("parsing Cargo.toml")?;
    let present = get_path(doc.as_item(), manifest.shape.lints_path()).is_some();
    if present && !force {
        return Ok(Written {
            target,
            outcome: Outcome::Kept,
        });
    }
    if present {
        remove_path(&mut doc, manifest.shape.lints_path());
    }
    let mut text = doc.to_string();
    append_block(&mut text, &render_lints(manifest.shape));
    atomic_write(&root.join(MANIFEST), text.as_bytes()).context("writing Cargo.toml")?;
    Ok(Written {
        target,
        outcome: if present {
            Outcome::Replaced
        } else {
            Outcome::Added
        },
    })
}

/// Add `[lints] workspace = true` to a member that has no `[lints]` at all.
/// A member with its own `[lints]` is left for the check to report.
fn scaffold_member_opt_in(root: &Path, member: &str) -> anyhow::Result<Option<Written>> {
    let rel = member_manifest(member);
    let path = root.join(&rel);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e).with_context(|| format!("reading {rel}")),
    };
    let doc: toml_edit::DocumentMut = text.parse().with_context(|| format!("parsing {rel}"))?;
    if doc.contains_key("lints") {
        return Ok(None);
    }
    let mut text = text;
    append_block(&mut text, "[lints]\nworkspace = true\n");
    atomic_write(&path, text.as_bytes()).with_context(|| format!("writing {rel}"))?;
    Ok(Some(Written {
        target: format!("{rel}:lints"),
        outcome: Outcome::Added,
    }))
}

fn member_manifest(member: &str) -> String {
    if member == "." {
        MANIFEST.to_owned()
    } else {
        format!("{}/{MANIFEST}", member.trim_end_matches('/'))
    }
}

/// Append `block` after one blank line, whatever the file ended with.
fn append_block(text: &mut String, block: &str) {
    let trimmed = text.trim_end_matches('\n').len();
    text.truncate(trimmed);
    if !text.is_empty() {
        text.push_str("\n\n");
    }
    text.push_str(block);
}

fn get_path<'a>(item: &'a toml_edit::Item, path: &[&str]) -> Option<&'a toml_edit::Item> {
    path.iter().try_fold(item, |item, key| item.get(key))
}

fn remove_path(doc: &mut toml_edit::DocumentMut, path: &[&str]) {
    match path {
        [key] => {
            doc.remove(key);
        }
        [parent, key] => {
            if let Some(table) = doc
                .get_mut(parent)
                .and_then(toml_edit::Item::as_table_like_mut)
            {
                table.remove(key);
            }
        }
        _ => {}
    }
}

/// Compare the project at `root` with the embedded templates.
///
/// `waivers` maps a drift location (or a prefix of one: a file, or a table
/// such as `Cargo.toml:workspace.lints.clippy`) to the reason the divergence
/// is deliberate.
///
/// # Errors
///
/// If `root` has no parseable `Cargo.toml`, or a file exists but cannot be
/// read. A file or member manifest that is missing or does not parse is
/// drift, not an error.
pub fn check(root: &Path, waivers: &IndexMap<String, String>) -> anyhow::Result<Report> {
    let manifest = Root::load(root)?;
    let mut drift = Vec::new();
    for file in &FILES {
        check_file(root, file, &mut drift)?;
    }
    check_lints(&manifest, &mut drift)?;
    for member in &manifest.members {
        check_member_opt_in(root, member, &mut drift)?;
    }
    Ok(apply_waivers(drift, waivers))
}

fn check_file(root: &Path, file: &ConfigFile, out: &mut Vec<Drift>) -> anyhow::Result<()> {
    let text = match std::fs::read_to_string(root.join(file.path)) {
        Ok(t) => t,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            out.push(Drift::new(file.path, "missing"));
            return Ok(());
        }
        Err(e) => return Err(e).with_context(|| format!("reading {}", file.path)),
    };
    let actual = match toml::from_str::<toml::Table>(&text) {
        Ok(t) => Value::Table(t),
        Err(e) => {
            out.push(Drift::new(file.path, parse_failure(&text, &e)));
            return Ok(());
        }
    };
    let mut expected = template_value(file.template)?;
    if file.path == MISE {
        check_ops_pin(&mut expected, &actual, out);
    }
    compare::compare(&expected, Some(&actual), file.path, Rule::Exact, out);
    Ok(())
}

/// The drift message for a hand-edited file that is not valid TOML: the
/// reason plus the line and column it was found at, on one line.
fn parse_failure(text: &str, err: &toml::de::Error) -> String {
    let reason = err.message();
    let Some(before) = err.span().and_then(|span| text.get(..span.start)) else {
        return format!("does not parse: {reason}");
    };
    let line = before.matches('\n').count().saturating_add(1);
    let column = before
        .rsplit('\n')
        .next()
        .map_or(0, |l| l.chars().count())
        .saturating_add(1);
    format!("does not parse at line {line}, column {column}: {reason}")
}

/// The ops pin is a floor, not an equality: the template names the oldest ops
/// the pipeline works with, and the ops running the check is the one the repo
/// pins, so a repo on a newer release must pass. Takes the pin out
/// of `expected` so the exact comparison skips it.
fn check_ops_pin(expected: &mut Value, actual: &Value, out: &mut Vec<Drift>) {
    let Some(floor) = expected
        .get_mut("tools")
        .and_then(Value::as_table_mut)
        .and_then(|tools| tools.remove("ops"))
    else {
        return;
    };
    let location = format!("{MISE}:tools.ops");
    let Some(pin) = actual.get("tools").and_then(|tools| tools.get("ops")) else {
        out.push(Drift::new(location, "missing"));
        return;
    };
    let version = |v: &Value| parse_version(v.as_str().or_else(|| v.get("version")?.as_str())?);
    if version(pin)
        .zip(version(&floor))
        .is_none_or(|(have, want)| have < want)
    {
        out.push(Drift::new(
            location,
            format!("expected {floor} or later, found {pin}"),
        ));
    }
}

/// `1.2.3` as `[1, 2, 3]`, which orders the way versions do. Anything else,
/// such as `latest` or a partial `77`, has no place against a floor.
fn parse_version(v: &str) -> Option<[u64; 3]> {
    let mut parts = v.split('.').map(|part| part.parse().ok());
    let version = [parts.next()??, parts.next()??, parts.next()??];
    parts.next().is_none().then_some(version)
}

fn check_lints(manifest: &Root, out: &mut Vec<Drift>) -> anyhow::Result<()> {
    let expected = template_value(LINTS_TEMPLATE)?;
    let expected = expected
        .get("lints")
        .context("lints template has no [lints] table")?;
    let actual = Value::Table(toml::from_str(&manifest.text).context("parsing Cargo.toml")?);
    let actual = manifest
        .shape
        .lints_path()
        .iter()
        .try_fold(&actual, |v, key| v.get(key));
    compare::compare(
        expected,
        actual,
        manifest.shape.lints_location(),
        Rule::LintLevel,
        out,
    );
    Ok(())
}

fn check_member_opt_in(root: &Path, member: &str, out: &mut Vec<Drift>) -> anyhow::Result<()> {
    let rel = member_manifest(member);
    let text = match std::fs::read_to_string(root.join(&rel)) {
        Ok(t) => t,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e).with_context(|| format!("reading {rel}")),
    };
    let manifest = match toml::from_str::<toml::Table>(&text) {
        Ok(t) => t,
        Err(e) => {
            out.push(Drift::new(rel, parse_failure(&text, &e)));
            return Ok(());
        }
    };
    let opted_in = manifest
        .get("lints")
        .and_then(|lints| lints.get("workspace"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !opted_in {
        out.push(Drift::new(
            format!("{rel}:lints.workspace"),
            "missing `[lints] workspace = true`, so the workspace lint policy does not apply",
        ));
    }
    Ok(())
}

fn template_value(template: &str) -> anyhow::Result<Value> {
    toml::from_str::<toml::Table>(template)
        .map(Value::Table)
        .context("embedded foundation template does not parse")
}

/// A waiver covers a location when it names it exactly or names a file or
/// table that contains it.
fn waiver_covers(waiver: &str, location: &str) -> bool {
    location
        .strip_prefix(waiver)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(':') || rest.starts_with('.'))
}

fn apply_waivers(drift: Vec<Drift>, waivers: &IndexMap<String, String>) -> Report {
    let mut report = Report::default();
    let mut used = vec![false; waivers.len()];
    for d in drift {
        let hit = waivers
            .iter()
            .enumerate()
            .find(|(_, (key, _))| waiver_covers(key, &d.location));
        match hit {
            Some((i, (_, reason))) => {
                if let Some(u) = used.get_mut(i) {
                    *u = true;
                }
                report.waived.push((d, reason.clone()));
            }
            None => report.drift.push(d),
        }
    }
    report.unused_waivers = waivers
        .keys()
        .zip(used)
        .filter(|(_, used)| !used)
        .map(|(key, _)| key.clone())
        .collect();
    report
}

#[cfg(test)]
mod tests;
