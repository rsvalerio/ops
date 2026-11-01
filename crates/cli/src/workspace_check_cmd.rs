//! `ops check-workspaces`: the nested workspaces in `[cargo] workspaces`
//! agree with the root one.
//!
//! A second cargo workspace beside the root — cargo-fuzz's `fuzz/`, excluded
//! from the root `[workspace]` — is a workspace root of its own, so it
//! inherits nothing: it resolves its own `Cargo.lock` from scratch and copies
//! the root lint policy by hand. Nothing makes cargo keep either in step, and
//! a `cargo update` in one does not touch the other. A fuzzer built on a
//! different patch release than the shipped binary reports findings that do
//! not reproduce, and a lint added to the root leaves the least-read code in
//! the repo silently outside the floor.
//!
//! Two checks run per configured workspace, both read-only:
//!
//! - **Lock agreement.** Every `(name, version, source)` the nested lockfile
//!   pins for a crate the root lockfile also holds must be the triple the
//!   root pins — the nested graph may be smaller and carry crates the root
//!   has none of, but not resolve `rand 0.8.8` where the root resolves
//!   `0.8.7`. Beyond versions, the dependency *edges* of a shared package
//!   must agree: an edge the nested lockfile records and the root does not
//!   is drift, and for a crate whose manifest lives in this repo every
//!   required (non-optional, non-dev) dependency the manifest declares must
//!   be an edge in *both* lockfiles — the half that catches a lockfile that
//!   pins every version right and still records the wrong graph. Edges are
//!   compared by crate name; versions are the first rule's job.
//! - **Lints sync.** The nested `[lints]` tables must be the root
//!   `[workspace.lints]` tables, in both directions, modulo the exemptions
//!   recorded under `[cargo] lint_exemptions`. Values are compared as
//!   policies — key order and whitespace do not count — so a reflowed copy
//!   of a table is the same table. An exemption licenses *omitting* a root
//!   lint in one named workspace, nothing else: keeping a lint the root has
//!   dropped is still reported.
//!
//! To fix lockfile drift, re-seed the nested lockfile from the root one and
//! let cargo extend it: `cp Cargo.lock fuzz/Cargo.lock`, then any cargo
//! command inside `fuzz/`. Lint drift is fixed by copying the root table
//! across, or by recording the exemption with its reason.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Context as _;

use ops_core::config::CargoSection;

/// How many times a lockfile is re-read until two consecutive reads agree.
/// A cargo command in another shell rewrites these files whenever it
/// re-resolves, and a parse that spans such a write compares half of the old
/// graph against the new one and reports drift that is not there. Two
/// identical reads is not a lock — nothing here can take one over a file
/// cargo owns — but it turns a read across a write into a retry and then a
/// loud failure.
const SNAPSHOT_ATTEMPTS: usize = 3;

/// Directories whose contents are never a manifest worth mapping: build
/// output, the checkout's own state, and dot-directories.
const SKIP_DIRS: &[&str] = &["target", ".git", "node_modules"];

/// One `[[package]]` record of a lockfile.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct LockPkg {
    name: String,
    version: String,
    /// The `source` registry/git URL; `None` is a path dependency, recorded
    /// as `"local"` — the same name and version served from a different
    /// source is a different crate.
    source: Option<String>,
    /// Dependency names as the lockfile spells them (`"toml"` or
    /// `"toml 0.8.23"`); the name is the first whitespace-separated field.
    deps: BTreeSet<String>,
}

impl LockPkg {
    /// The identity the version rule compares: name, version, source — with
    /// a path dependency's lack of a source read as `local`.
    fn triple(&self) -> (&str, &str, &str) {
        (self.name.as_str(), self.version.as_str(), self.source.as_deref().unwrap_or("local"))
    }
}

/// Read `path` until two consecutive reads agree, then return the text.
fn snapshot(path: &Path) -> anyhow::Result<String> {
    for _ in 0..SNAPSHOT_ATTEMPTS {
        let a =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let b =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        if a == b {
            return Ok(a);
        }
    }
    anyhow::bail!("{}: changed under every read (is a cargo command running?)", path.display());
}

/// Parse a lockfile's `[[package]]` records. Lockfile v4 is TOML; a record's
/// `dependencies` entries are plain strings.
fn parse_lock(text: &str) -> anyhow::Result<Vec<LockPkg>> {
    let table: toml::Table = text.parse().context("parsing a Cargo.lock")?;
    let Some(packages) = table.get("package").and_then(toml::Value::as_array) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(packages.len());
    for pkg in packages {
        let Some(pkg) = pkg.as_table() else { continue };
        let (Some(name), Some(version)) = (
            pkg.get("name").and_then(toml::Value::as_str),
            pkg.get("version").and_then(toml::Value::as_str),
        ) else {
            anyhow::bail!("a Cargo.lock package record without a name or version");
        };
        let deps = pkg
            .get("dependencies")
            .and_then(toml::Value::as_array)
            .map(|deps| {
                deps.iter()
                    .filter_map(toml::Value::as_str)
                    .filter_map(|d| d.split_whitespace().next())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        out.push(LockPkg {
            name: name.to_owned(),
            version: version.to_owned(),
            source: pkg.get("source").and_then(toml::Value::as_str).map(str::to_owned),
            deps,
        });
    }
    Ok(out)
}

/// `[package] name` of a manifest, if it has one.
fn package_name(manifest: &toml::Table) -> Option<&str> {
    manifest.get("package")?.get("name")?.as_str()
}

/// The required (non-optional, non-dev) dependency names a manifest declares,
/// as the crate names the lockfile records edges under: the keys of its
/// `[dependencies]`, `[build-dependencies]` and `[target.'...'.dependencies]`
/// tables, minus the entries carrying `optional = true`, with a rename
/// (`my-serde = { package = "serde" }`) resolved to the crate's real name —
/// through `[workspace.dependencies]` too, for an inherited rename.
/// Dev-dependency tables are skipped: a lockfile records them only for a
/// workspace's own members, so their absence on one side is legitimate.
fn required_deps(manifest: &toml::Table, workspace_deps: Option<&toml::Table>) -> Vec<String> {
    let mut out = Vec::new();
    let mut visit = |table: &toml::Table| {
        for (key, spec) in table {
            let entry = spec.as_table();
            let optional = entry
                .is_some_and(|t| t.get("optional").and_then(toml::Value::as_bool) == Some(true));
            if optional {
                continue;
            }
            // A rename: the edge is recorded under the crate's real name. An
            // inherited `workspace = true` entry renames only if the
            // workspace dependency itself does.
            let renamed =
                entry.and_then(|t| t.get("package")).and_then(toml::Value::as_str).or_else(|| {
                    let inherited =
                        entry?.get("workspace").and_then(toml::Value::as_bool) == Some(true);
                    if !inherited {
                        return None;
                    }
                    workspace_deps
                        .and_then(|w| w.get(key))
                        .and_then(toml::Value::as_table)?
                        .get("package")
                        .and_then(toml::Value::as_str)
                });
            out.push(renamed.unwrap_or(key).to_owned());
        }
    };
    if let Some(deps) = manifest.get("dependencies").and_then(toml::Value::as_table) {
        visit(deps);
    }
    if let Some(deps) = manifest.get("build-dependencies").and_then(toml::Value::as_table) {
        visit(deps);
    }
    if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
        for target in targets.values() {
            let Some(target) = target.as_table() else { continue };
            for table in ["dependencies", "build-dependencies"] {
                if let Some(deps) = target.get(table).and_then(toml::Value::as_table) {
                    visit(deps);
                }
            }
        }
    }
    out
}

/// `<group>.<key>` → policy for every entry under a manifest's lint tables:
/// the `[workspace.lints]` sub-table of a root manifest, the `[lints]` table
/// of a nested one. A value is compared as the policy it configures — level
/// and priority, the priority defaulting the way cargo defaults it — so
/// `"deny"` and `{ level = "deny" }` are the same lint, and a reflowed copy
/// of a table is the same table.
fn lints_of(lints: Option<&toml::Value>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Some(lints) = lints.and_then(toml::Value::as_table) else { return out };
    for (key, value) in lints {
        match value {
            // A bare entry under the table itself (cargo wants groups; a
            // stray entry is compared as written rather than ignored).
            toml::Value::Table(t) => {
                for (lint, value) in t {
                    out.insert(format!("{key}.{lint}"), policy(value));
                }
            }
            value => {
                out.insert(key.clone(), policy(value));
            }
        }
    }
    out
}

/// A lint value as the policy it sets: a bare string is a level at cargo's
/// default priority, a table is its `level` and `priority`. Anything else a
/// lint table may one day hold is compared structurally.
fn policy(value: &toml::Value) -> String {
    match value {
        toml::Value::String(level) => format!("{level} (priority 0)"),
        toml::Value::Table(t) => {
            let level = t.get("level").and_then(toml::Value::as_str).unwrap_or_default();
            let priority = t.get("priority").and_then(toml::Value::as_integer).unwrap_or(0);
            format!("{level} (priority {priority})")
        }
        other => normalise(other),
    }
}

/// A lint value as one comparable string: strings quoted, tables as sorted
/// `key=value` pairs, scalars as text.
fn normalise(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => format!("\"{s}\""),
        toml::Value::Table(t) => {
            let mut parts: Vec<String> =
                t.iter().map(|(k, v)| format!("{k}={}", normalise(v))).collect();
            parts.sort();
            format!("{{{}}}", parts.join(","))
        }
        other => other.to_string(),
    }
}

/// Every manifest under `root`, as `[package] name` → the paths claiming it.
/// The manifests the missing-edge rule can consult are the ones in this
/// repo, wherever in the tree they sit; build output and dot-directories are
/// not looked into, and a symlinked `Cargo.toml` is skipped unread — the
/// walk never follows a link, in line with the repo's rule that nothing
/// follows a symlink out of the project. A name claimed by more than one
/// manifest (a fixture, a vendored copy) is ambiguous, and the caller skips
/// it rather than trusting one arbitrary copy.
fn manifests_in(root: &Path) -> anyhow::Result<BTreeMap<String, Vec<PathBuf>>> {
    fn walk(dir: &Path, out: &mut BTreeMap<String, Vec<PathBuf>>) -> anyhow::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if file_type.is_dir() {
                if name.starts_with('.') || SKIP_DIRS.contains(&name.as_ref()) {
                    continue;
                }
                walk(&entry.path(), out)?;
            } else if name == "Cargo.toml" {
                let text = std::fs::read_to_string(entry.path())?;
                if let Ok(manifest) = text.parse::<toml::Table>() {
                    if let Some(name) = package_name(&manifest) {
                        out.entry(name.to_owned()).or_default().push(entry.path());
                    }
                }
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    walk(root, &mut out).with_context(|| format!("walking {}", root.display()))?;
    Ok(out)
}

/// What every [`check_lock`] pass walks with: the root lockfile's packages,
/// the repo's manifests by package name, and the root
/// `[workspace.dependencies]` for resolving an inherited rename.
struct LockCtx<'a> {
    root_pkgs: &'a [LockPkg],
    manifests: &'a BTreeMap<String, Vec<PathBuf>>,
    workspace_deps: Option<&'a toml::Table>,
}

/// Append the lockfile findings for one nested workspace: version triples,
/// then edges, each as one human-readable line.
fn check_lock(
    root: &Path,
    entry: &str,
    ctx: &LockCtx<'_>,
    out: &mut Vec<String>,
) -> anyhow::Result<()> {
    let nested_path = root.join(entry).join("Cargo.lock");
    let nested_pkgs = parse_lock(&snapshot(&nested_path)?)?;

    let root_triples: BTreeSet<(&str, &str, &str)> =
        ctx.root_pkgs.iter().map(LockPkg::triple).collect();
    let root_by_triple: BTreeMap<(&str, &str, &str), &LockPkg> =
        ctx.root_pkgs.iter().map(|p| (p.triple(), p)).collect();
    let root_versions: BTreeMap<&str, Vec<&str>> = {
        let mut map: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for pkg in ctx.root_pkgs {
            map.entry(pkg.name.as_str()).or_default().push(&pkg.version);
        }
        map
    };

    // Rule 1 — versions. One-directional on purpose: a crate the root graph
    // does not hold at all is allowed, a shared crate at another version or
    // from another source is not.
    for pkg in &nested_pkgs {
        let Some(root_pins) = root_versions.get(pkg.name.as_str()) else { continue };
        if !root_triples.contains(&pkg.triple()) {
            let pins = root_pins.join(", ");
            out.push(format!(
                "{}: pins {} {} ({}), root Cargo.lock pins {}",
                entry,
                pkg.name,
                pkg.version,
                pkg.source.as_deref().unwrap_or("local"),
                pins
            ));
        }
    }

    let nested_by_triple: BTreeMap<(&str, &str, &str), &LockPkg> =
        nested_pkgs.iter().map(|p| (p.triple(), p)).collect();

    // Rule 2 — no extra edge. Root-only edges are allowed: a lockfile records
    // dev-dependency edges only for its own members and prunes optional
    // dependencies nothing activates, so the root record of a shared crate
    // legitimately carries more.
    for pkg in &nested_pkgs {
        let Some(root_pkg) = root_by_triple.get(&pkg.triple()) else { continue };
        for dep in &pkg.deps {
            if !root_pkg.deps.contains(dep) {
                out.push(format!(
                    "{}: {} {} records a dependency on {dep}, root does not",
                    entry, pkg.name, pkg.version
                ));
            }
        }
    }

    // Rule 3 — no missing edge. For a crate whose manifest is in this repo,
    // every required dependency must be an edge of its record in both
    // lockfiles; a stale lockfile on either side is caught.
    for pkg in &nested_pkgs {
        if pkg.source.is_some() {
            continue;
        }
        if !root_triples.contains(&pkg.triple()) {
            continue;
        }
        // A name claimed by several manifests is ambiguous — a fixture or a
        // vendored copy — and the intended edge set is not knowable from an
        // arbitrary one of them, so the crate is skipped.
        let [manifest_path] = ctx.manifests.get(pkg.name.as_str()).map_or(&[][..], Vec::as_slice)
        else {
            continue;
        };
        let manifest: toml::Table = std::fs::read_to_string(manifest_path)?
            .parse()
            .with_context(|| format!("parsing {}", manifest_path.display()))?;
        for dep in required_deps(&manifest, ctx.workspace_deps) {
            let Some(root_record) = root_by_triple.get(&pkg.triple()) else { continue };
            if !root_record.deps.contains(&dep) {
                out.push(format!(
                    "{}: {} requires {dep}, root Cargo.lock records no such edge",
                    manifest_path.display(),
                    pkg.name
                ));
            }
            if !nested_by_triple.get(&pkg.triple()).is_some_and(|r| r.deps.contains(&dep)) {
                out.push(format!(
                    "{}: {} requires {dep}, {entry}/Cargo.lock records no such edge",
                    manifest_path.display(),
                    pkg.name
                ));
            }
        }
    }
    Ok(())
}

/// Append the lint findings for one nested workspace: the nested `[lints]`
/// tables as a copy of the root `[workspace.lints]`, in both directions,
/// modulo `exemptions` (`<group>.<lint>` keys).
fn check_lints(
    root: &Path,
    entry: &str,
    exemptions: &BTreeMap<String, String>,
    out: &mut Vec<String>,
) -> anyhow::Result<()> {
    let root_manifest: toml::Table = snapshot(&root.join("Cargo.toml"))?.parse()?;
    let nested_manifest: toml::Table = snapshot(&root.join(entry).join("Cargo.toml"))?.parse()?;
    let root_lints = lints_of(root_manifest.get("workspace").and_then(|w| w.get("lints")));
    let nested_lints = lints_of(nested_manifest.get("lints"));

    // Root → nested: missing, or set to a different value. An exempt key must
    // be absent — the exemption licenses omitting the lint, not diverging
    // from it.
    for (key, value) in &root_lints {
        let nested_value = nested_lints.get(key);
        if let Some(reason) = exemptions.get(key) {
            if let Some(nested) = nested_value {
                out.push(format!(
                    "{entry}: {key} is exempt (\"{reason}\"), but its [lints] sets it to {nested} \
                     — drop the lint, or drop the exemption"
                ));
            }
            continue;
        }
        match nested_value {
            None => {
                out.push(format!(
                    "{entry}: root sets {key} = {value}, its [lints] does not set it"
                ));
            }
            Some(nested) if nested != value => {
                out.push(format!("{entry}: root sets {key} = {value}, its [lints] sets {nested}"));
            }
            Some(_) => {}
        }
    }

    // Nested → root: set here and nowhere else. Exemptions play no part in
    // this direction — consulting one would license keeping a lint the root
    // has since dropped, which is exactly what this pass exists to catch.
    for (key, value) in &nested_lints {
        if !root_lints.contains_key(key) {
            out.push(format!("{entry}: its [lints] sets {key} = {value}, root does not set it"));
        }
    }
    Ok(())
}

/// Run `ops check-workspaces` in `root`, or only describe it under `dry_run`.
///
/// # Errors
///
/// A lockfile or manifest cannot be read or parsed.
pub fn run_workspace_check(
    root: &Path,
    cargo: &CargoSection,
    w: &mut dyn Write,
) -> anyhow::Result<ExitCode> {
    let entries: Vec<&str> =
        cargo.workspaces().iter().map(String::as_str).filter(|e| *e != ".").collect();
    if entries.is_empty() {
        writeln!(w, "[cargo] workspaces names no extra workspace; nothing to check")?;
        return Ok(ExitCode::SUCCESS);
    }
    let mut findings = Vec::new();
    let root_pkgs = parse_lock(&snapshot(&root.join("Cargo.lock")).context(
        "the root Cargo.lock is the baseline every nested lockfile is checked against",
    )?)?;
    let root_manifest: toml::Table = snapshot(&root.join("Cargo.toml"))?.parse()?;
    let workspace_deps = root_manifest
        .get("workspace")
        .and_then(|w| w.get("dependencies"))
        .and_then(toml::Value::as_table);
    let manifests = manifests_in(root)?;
    let ctx = LockCtx { root_pkgs: &root_pkgs, manifests: &manifests, workspace_deps };
    for &entry in &entries {
        check_lock(root, entry, &ctx, &mut findings)?;
        let exemptions: BTreeMap<String, String> = cargo
            .lint_exemptions
            .as_ref()
            .and_then(|map| map.get(entry))
            .map(|map| map.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        check_lints(root, entry, &exemptions, &mut findings)?;
    }
    if findings.is_empty() {
        for entry in entries {
            writeln!(
                w,
                "{entry}/Cargo.lock agrees with Cargo.lock, and its [lints] with [workspace.lints]"
            )?;
        }
        return Ok(ExitCode::SUCCESS);
    }
    writeln!(w, "nested workspace drift:")?;
    for finding in &findings {
        writeln!(w, "  {finding}")?;
    }
    writeln!(w)?;
    writeln!(
        w,
        "Re-seed a lockfile:  cp Cargo.lock <workspace>/Cargo.lock, then run any cargo command \
         inside it. Copy a [lints] table across, or record the exemption under [cargo] \
         lint_exemptions with its reason."
    )?;
    Ok(ExitCode::FAILURE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    fn section(workspaces: &[&str], exemptions: &[(&str, &str, &str)]) -> CargoSection {
        CargoSection {
            workspaces: Some(workspaces.iter().map(|s| (*s).to_owned()).collect()),
            lint_exemptions: if exemptions.is_empty() {
                None
            } else {
                let mut map: IndexMap<String, IndexMap<String, String>> = IndexMap::new();
                for (ws, lint, reason) in exemptions {
                    map.entry((*ws).to_owned())
                        .or_default()
                        .insert((*lint).to_owned(), (*reason).to_owned());
                }
                Some(map)
            },
            ..CargoSection::default()
        }
    }

    /// A repo whose nested workspace agrees with the root, plus knobs the
    /// individual tests perturb. The root holds crate `app` depending on
    /// `rand`; the nested `fuzz` workspace holds `fuzz-target` depending on
    /// `app` by path, so `rand` is shared.
    fn repo(dir: &Path, root_lock: &str, nested_lock: &str, nested_lints: &str) {
        std::fs::write(
            dir.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n\
             [workspace.lints.rust]\nunsafe_code = \"forbid\"\n\
             [workspace.lints.clippy]\nall = { level = \"deny\", priority = -1 }\n",
        )
        .unwrap();
        std::fs::write(dir.join("Cargo.lock"), root_lock).unwrap();
        std::fs::create_dir_all(dir.join("crates/app")).unwrap();
        std::fs::write(
            dir.join("crates/app/Cargo.toml"),
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nrand = \"0.8\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.join("fuzz")).unwrap();
        std::fs::write(
            dir.join("fuzz/Cargo.toml"),
            format!("[workspace]\n\n[lints.rust]\n{nested_lints}"),
        )
        .unwrap();
        std::fs::write(dir.join("fuzz/Cargo.lock"), nested_lock).unwrap();
    }

    fn root_lock() -> String {
        "[[package]]\nname = \"app\"\nversion = \"0.1.0\"\ndependencies = [\"rand\"]\n\n\
         [[package]]\nname = \"rand\"\nversion = \"0.8.7\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\ndependencies = [\"rand_core\"]\n\n\
         [[package]]\nname = \"rand_core\"\nversion = \"0.6.4\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n"
            .to_owned()
    }

    /// A nested lockfile agreeing with the root: the shared crates carry the
    /// same triples and edges, and the extra crates are allowed.
    fn agreeing_nested_lock() -> String {
        "[[package]]\nname = \"app\"\nversion = \"0.1.0\"\ndependencies = [\"rand\"]\n\n\
         [[package]]\nname = \"fuzz-target\"\nversion = \"0.1.0\"\n\n\
         [[package]]\nname = \"libfuzzer-sys\"\nversion = \"4.0.0\"\n\n\
         [[package]]\nname = \"rand\"\nversion = \"0.8.7\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\ndependencies = [\"rand_core\"]\n\n\
         [[package]]\nname = \"rand_core\"\nversion = \"0.6.4\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n"
            .to_owned()
    }

    fn agreeing_nested_lints() -> String {
        "unsafe_code = \"forbid\"\n".to_owned()
            + "[lints.clippy]\nall = { level = \"deny\", priority = -1 }\n"
    }

    fn run(dir: &Path, cargo: &CargoSection) -> (ExitCode, String) {
        let mut out = Vec::new();
        let code = run_workspace_check(dir, cargo, &mut out).unwrap();
        (code, String::from_utf8(out).unwrap())
    }

    #[test]
    fn an_agreeing_workspace_passes_and_names_it() {
        let dir = tempfile::tempdir().unwrap();
        repo(dir.path(), &root_lock(), &agreeing_nested_lock(), &agreeing_nested_lints());
        let (code, out) = run(dir.path(), &section(&[".", "fuzz"], &[]));
        assert_eq!(code, ExitCode::SUCCESS, "{out}");
        assert!(out.contains("fuzz/Cargo.lock agrees"), "{out}");
    }

    #[test]
    fn no_extra_workspace_is_a_clean_no_op() {
        let dir = tempfile::tempdir().unwrap();
        let (code, out) = run(dir.path(), &section(&["."], &[]));
        assert_eq!(code, ExitCode::SUCCESS, "{out}");
        assert!(out.contains("nothing to check"), "{out}");
    }

    #[test]
    fn a_shared_crate_at_another_version_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let mut nested = agreeing_nested_lock();
        nested = nested.replace(
            "name = \"rand\"\nversion = \"0.8.7\"",
            "name = \"rand\"\nversion = \"0.8.8\"",
        );
        repo(dir.path(), &root_lock(), &nested, &agreeing_nested_lints());
        let (code, out) = run(dir.path(), &section(&[".", "fuzz"], &[]));
        assert_eq!(code, ExitCode::FAILURE, "{out}");
        assert!(out.contains("pins rand 0.8.8"), "{out}");
        assert!(out.contains("root Cargo.lock pins 0.8.7"), "{out}");
    }

    #[test]
    fn a_nested_only_edge_on_a_shared_crate_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let nested = agreeing_nested_lock().replace(
            "dependencies = [\"rand_core\"]",
            "dependencies = [\"rand_core\", \"getrandom\"]",
        );
        repo(dir.path(), &root_lock(), &nested, &agreeing_nested_lints());
        let (code, out) = run(dir.path(), &section(&[".", "fuzz"], &[]));
        assert_eq!(code, ExitCode::FAILURE, "{out}");
        assert!(out.contains("records a dependency on getrandom, root does not"), "{out}");
    }

    #[test]
    fn a_required_dependency_missing_from_a_lockfile_edge_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        // The root's `app` record lost the `rand` edge its manifest declares;
        // both lockfiles agree on every version.
        let root = root_lock().replace(
            "name = \"app\"\nversion = \"0.1.0\"\ndependencies = [\"rand\"]",
            "name = \"app\"\nversion = \"0.1.0\"",
        );
        repo(dir.path(), &root, &agreeing_nested_lock(), &agreeing_nested_lints());
        let (code, out) = run(dir.path(), &section(&[".", "fuzz"], &[]));
        assert_eq!(code, ExitCode::FAILURE, "{out}");
        assert!(out.contains("app requires rand, root Cargo.lock records no such edge"), "{out}");
    }

    #[test]
    fn a_missing_lint_a_changed_lint_and_an_extra_lint_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        let lints = "unsafe_code = \"allow\"\nmissing_docs = \"warn\"\n[lints.clippy]\nall = { level = \"deny\", priority = -1 }\n".to_owned();
        repo(dir.path(), &root_lock(), &agreeing_nested_lock(), &lints);
        let (code, out) = run(dir.path(), &section(&[".", "fuzz"], &[]));
        assert_eq!(code, ExitCode::FAILURE, "{out}");
        assert!(
            out.contains("root sets rust.unsafe_code = forbid (priority 0), its [lints] sets allow (priority 0)"),
            "{out}"
        );
        assert!(out.contains("its [lints] sets rust.missing_docs"), "{out}");
    }

    #[test]
    fn a_lint_the_nested_tables_drop_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let lints = "unsafe_code = \"forbid\"\n".to_owned();
        repo(dir.path(), &root_lock(), &agreeing_nested_lock(), &lints);
        let (code, out) = run(dir.path(), &section(&[".", "fuzz"], &[]));
        assert_eq!(code, ExitCode::FAILURE, "{out}");
        assert!(out.contains("clippy.all"), "{out}");
        assert!(out.contains("does not set it"), "{out}");
    }

    #[test]
    fn an_exemption_licenses_omitting_a_lint_but_not_setting_it() {
        let dir = tempfile::tempdir().unwrap();
        // Omitted under a recorded exemption: clean.
        let lints = "unsafe_code = \"forbid\"\n".to_owned();
        repo(dir.path(), &root_lock(), &agreeing_nested_lock(), &lints);
        let cargo = section(&[".", "fuzz"], &[("fuzz", "clippy.all", "the reason")]);
        let (code, out) = run(dir.path(), &cargo);
        assert_eq!(code, ExitCode::SUCCESS, "{out}");
        // Set anyway under the exemption: drift, in both the exempt key and
        // a lint the root never had.
        let lints = "unsafe_code = \"forbid\"\ndeprecated = \"allow\"\n[lints.clippy]\nall = { level = \"deny\", priority = -1 }\n".to_owned();
        repo(dir.path(), &root_lock(), &agreeing_nested_lock(), &lints);
        let (code, out) = run(dir.path(), &cargo);
        assert_eq!(code, ExitCode::FAILURE, "{out}");
        assert!(
            out.contains("clippy.all is exempt (\"the reason\"), but its [lints] sets it to"),
            "{out}"
        );
        assert!(out.contains("its [lints] sets rust.deprecated"), "{out}");
    }

    #[test]
    fn a_lint_the_root_dropped_is_reported_even_when_exempt() {
        let dir = tempfile::tempdir().unwrap();
        // The root holds only `unsafe_code`; the nested tables keep
        // `missing_docs` too, and even an exemption recorded for it does not
        // license that — an exemption licenses omitting, never keeping.
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n[workspace.lints.rust]\nunsafe_code = \"forbid\"\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("Cargo.lock"), "version = 4\n").unwrap();
        std::fs::create_dir_all(dir.path().join("crates/app")).unwrap();
        std::fs::write(
            dir.path().join("crates/app/Cargo.toml"),
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("fuzz")).unwrap();
        std::fs::write(
            dir.path().join("fuzz/Cargo.toml"),
            "[workspace]\n\n[lints.rust]\nunsafe_code = \"forbid\"\nmissing_docs = \"warn\"\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("fuzz/Cargo.lock"), "version = 4\n").unwrap();
        let cargo = section(&[".", "fuzz"], &[("fuzz", "rust.missing_docs", "kept on purpose")]);
        let (code, out) = run(dir.path(), &cargo);
        assert_eq!(code, ExitCode::FAILURE, "{out}");
        assert!(
            out.contains(
                "its [lints] sets rust.missing_docs = warn (priority 0), root does not set it"
            ),
            "{out}"
        );
    }

    #[test]
    fn a_reflowed_lint_table_is_the_same_policy() {
        let dir = tempfile::tempdir().unwrap();
        // Same clippy.all policy with its keys in the other order.
        let lints =
            "unsafe_code = \"forbid\"\n[lints.clippy]\nall = { priority = -1, level = \"deny\" }\n"
                .to_owned();
        repo(dir.path(), &root_lock(), &agreeing_nested_lock(), &lints);
        let (code, out) = run(dir.path(), &section(&[".", "fuzz"], &[]));
        assert_eq!(code, ExitCode::SUCCESS, "{out}");
    }

    #[test]
    fn a_shorthand_lint_and_its_table_form_are_the_same_policy() {
        let dir = tempfile::tempdir().unwrap();
        // The root spells the level as a bare string, the nested tables as
        // a table. One policy, not drift.
        let lints = "unsafe_code = { level = \"forbid\" }\n[lints.clippy]\nall = { level = \"deny\", priority = -1 }\n".to_owned();
        repo(dir.path(), &root_lock(), &agreeing_nested_lock(), &lints);
        let (code, out) = run(dir.path(), &section(&[".", "fuzz"], &[]));
        assert_eq!(code, ExitCode::SUCCESS, "{out}");
    }

    #[test]
    fn a_renamed_dependency_is_required_under_its_crate_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("crates/app")).unwrap();
        std::fs::create_dir_all(dir.path().join("fuzz")).unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n[workspace.lints.rust]\nunsafe_code = \"forbid\"\n",
        )
        .unwrap();
        // The manifest renames serde; both lockfiles record the edge under
        // the crate's real name, which is the one required.
        std::fs::write(
            dir.path().join("crates/app/Cargo.toml"),
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nmy-serde = { package = \"serde\", version = \"1\" }\n",
        )
        .unwrap();
        let lock = "[[package]]\nname = \"app\"\nversion = \"0.1.0\"\ndependencies = [\"serde\"]\n"
            .to_owned();
        std::fs::write(dir.path().join("Cargo.lock"), &lock).unwrap();
        std::fs::write(
            dir.path().join("fuzz/Cargo.toml"),
            "[workspace]\n\n[lints.rust]\nunsafe_code = \"forbid\"\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("fuzz/Cargo.lock"), &lock).unwrap();
        let (code, out) = run(dir.path(), &section(&[".", "fuzz"], &[]));
        assert_eq!(code, ExitCode::SUCCESS, "{out}");
    }

    #[test]
    fn a_symlinked_manifest_is_skipped_unread() {
        let dir = tempfile::tempdir().unwrap();
        // A `Cargo.toml` symlinked out of the repo: reading it would follow
        // the link (and here, fail on a directory); the walk skips it.
        std::fs::create_dir_all(dir.path().join("decoy")).unwrap();
        std::os::unix::fs::symlink("/", dir.path().join("decoy/Cargo.toml")).unwrap();
        let (code, out) = run(dir.path(), &section(&["."], &[]));
        assert_eq!(code, ExitCode::SUCCESS, "{out}");
    }

    #[test]
    fn an_ambiguous_package_name_skips_the_missing_edge_rule() {
        let dir = tempfile::tempdir().unwrap();
        // Two manifests claim `app`. Trusting either would consult a
        // manifest the lockfiles may not describe, so the crate is skipped:
        // the unambiguous missing edge (`left-out`) is not reported.
        std::fs::create_dir_all(dir.path().join("crates/app")).unwrap();
        std::fs::create_dir_all(dir.path().join("vendor/app")).unwrap();
        std::fs::create_dir_all(dir.path().join("fuzz")).unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n[workspace.lints.rust]\nunsafe_code = \"forbid\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("crates/app/Cargo.toml"),
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\nleft-out = \"1\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("vendor/app/Cargo.toml"),
            "[package]\nname = \"app\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let lock = "[[package]]\nname = \"app\"\nversion = \"0.1.0\"\n".to_owned();
        std::fs::write(dir.path().join("Cargo.lock"), &lock).unwrap();
        std::fs::write(
            dir.path().join("fuzz/Cargo.toml"),
            "[workspace]\n\n[lints.rust]\nunsafe_code = \"forbid\"\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("fuzz/Cargo.lock"), &lock).unwrap();
        let (code, out) = run(dir.path(), &section(&[".", "fuzz"], &[]));
        assert_eq!(code, ExitCode::SUCCESS, "{out}");
    }
}
