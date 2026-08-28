//! Rust-specific `review_targets` data provider.
//!
//! Supported project shapes:
//!
//! - **Cargo workspace**: one target per resolved `[workspace].members` entry
//!   (the same member view the about providers use), named by cargo package
//!   name with a display-name fallback for members whose manifest cannot be
//!   read or parsed.
//! - **Single-package project** (a `Cargo.toml` with `[package]` and no
//!   `[workspace]` table, ERR-6 / TASK-1812): the root package itself is the
//!   single review target, at path `.`.
//!
//! A manifest that declares neither reviewable shape — no resolvable
//! `[workspace]` members and no `[package].name` — is a typed error naming
//! that condition, never an empty target list (the engine would misread an
//! empty list as "nothing to review").

use ops_about::cards::format_unit_name;
use ops_about_rust::{
    member_path_is_workspace_safe, read_crate_metadata, resolved_workspace_members,
};
use ops_cargo_toml::{find_workspace_root_strict, CargoToml, CargoTomlProvider};
use ops_extension::{Context, DataProvider, DataProviderError};

/// Review skill the subtask titles reference.
pub const SKILL_NAME: &str = "code-review-rust";

/// Member path reported for the root package of a single-package project
/// (ERR-6 / TASK-1812). `ReviewTarget::path` is summary context only, so the
/// workspace-root-relative `.` is the accurate answer here.
const ROOT_PACKAGE_PATH: &str = ".";

pub struct RustReviewTargetsProvider;

impl DataProvider for RustReviewTargetsProvider {
    fn name(&self) -> &'static str {
        ops_create_review_tasks::DATA_PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let root = find_workspace_root_strict(ctx.working_directory.as_path())
            .map_err(DataProviderError::computation_error)?;
        let manifest = CargoTomlProvider::with_root(root.clone()).provide_typed(ctx)?;
        let members = resolved_workspace_members(&manifest, &root);

        let mut targets: Vec<(String, String)> = if members.is_empty() {
            vec![(
                root_package_name(&manifest, &root)?,
                ROOT_PACKAGE_PATH.to_string(),
            )]
        } else {
            members
                .iter()
                .map(|member| (member_target_name(member, &root), member.clone()))
                .collect()
        };
        disambiguate_target_names(&mut targets);

        let targets: Vec<serde_json::Value> = targets
            .into_iter()
            .map(|(name, path)| serde_json::json!({ "name": name, "path": path }))
            .collect();

        // PERF-3 / TASK-1819: `json!` already evaluates to a `Value`; wrapping
        // it in `serde_json::to_value` deep-copied the whole payload and added
        // an error branch `Value`'s infallible `Serialize` impl can never take.
        Ok(serde_json::json!({
            "skill": SKILL_NAME,
            "targets": targets,
        }))
    }
}

/// Review-target name for a single-package project: the root `[package].name`.
///
/// ERR-6 / TASK-1812: `find_workspace_root_strict` accepts the first
/// `Cargo.toml` it finds even when that manifest declares no `[workspace]`,
/// and `resolved_workspace_members` returns an empty `Vec` for such a
/// manifest. Returning that empty list made the engine report "nothing to
/// review" for the most common Rust project shape. The root package is the
/// review target; only a manifest with neither shape is an error, and it says
/// so in its own words rather than through an empty-list sentinel.
fn root_package_name(
    manifest: &CargoToml,
    root: &std::path::Path,
) -> Result<String, DataProviderError> {
    manifest.package_name().map(str::to_string).ok_or_else(|| {
        DataProviderError::computation_failed(format!(
            "manifest at {:?} declares no [workspace] members and no [package].name; \
             create-review-tasks needs at least one review target",
            root.display()
        ))
    })
}

/// Review-target name for one workspace member: its cargo package name, or
/// the capitalized display name when the manifest is absent, unreadable, or
/// unparseable.
///
/// DUP-3 / TASK-1814: the manifest read/parse/log policy is
/// `ops_about_rust::read_crate_metadata`, not a second copy of it here, and
/// the SEC-14 / TASK-1246 member-path guard is applied before the join —
/// `Path::join` discards `root` when `member` is absolute and walks parents
/// on `..`, which would otherwise drive the read and its tracing breadcrumbs
/// at an arbitrary filesystem location. `resolved_workspace_members` already
/// drops such members today; the guard is the defence-in-depth layer that
/// keeps that true if this provider is ever fed a member list from elsewhere.
fn member_target_name(member: &str, root: &std::path::Path) -> String {
    // SEC-11 / TASK-1822: `member` is untrusted `Cargo.toml` content, so every
    // tracing field carrying it uses the `?` (Debug) formatter — embedded
    // newlines and ANSI escapes are escaped and cannot forge log records.
    if !member_path_is_workspace_safe(member) {
        tracing::warn!(
            member = ?member,
            "SEC-14 / TASK-1246: workspace member is absolute or contains `..`; \
             not reading its manifest"
        );
        return format_unit_name(member);
    }
    let member_toml = root.join(member).join("Cargo.toml");
    read_crate_metadata(&member_toml).name.unwrap_or_else(|| {
        tracing::warn!(
            member = ?member,
            "no package name for workspace member; falling back to display name"
        );
        format_unit_name(member)
    })
}

/// Make every target name unique by appending the member path to names that
/// repeat.
///
/// PATTERN-1 / TASK-1839: `ops_create_review_tasks::ReviewTarget::name` is
/// documented as "a display name (unique per workspace)", and it is the *only*
/// identity the created backlog subtask carries — the title is
/// `REVIEW: Run skill {skill} against {name}` and the member path never
/// reaches the written file. The display-name fallback keeps only the last
/// path segment (`crates/parser` and `tools/parser` both become `Parser`), so
/// two members can otherwise produce byte-identical subtasks with no way to
/// tell which crate either one means. Disambiguating on collision leaves the
/// happy path — unique cargo package names — untouched.
fn disambiguate_target_names(targets: &mut [(String, String)]) {
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut duplicates: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (name, _) in targets.iter() {
        if !seen.insert(name.as_str()) {
            duplicates.insert(name.clone());
        }
    }
    for (name, path) in targets.iter_mut() {
        if duplicates.contains(name.as_str()) {
            *name = format!("{name} ({path})");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `tracing` caches a callsite's `Interest` globally. A test that reaches
    // the fallback `warn!` with no subscriber installed can cache "never" for
    // that callsite while `member_breadcrumb_debug_escapes_control_characters`
    // is capturing on another thread, silently emptying its buffer. Every test
    // that reaches the callsite therefore shares the
    // `serial_test::serial(fallback_breadcrumb)` group.

    /// Build a scratch Cargo workspace: root manifest with `members` plus
    /// one member crate per (dir, package-name) pair. Members whose name is
    /// `None` get no manifest at all.
    fn scratch_workspace(
        members: &str,
        crates: &[(&str, Option<&str>)],
    ) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().to_path_buf();
        std::fs::write(
            root.join("Cargo.toml"),
            format!("[workspace]\nmembers = [{members}]\n"),
        )
        .expect("root manifest");
        for (dir_name, pkg_name) in crates {
            let crate_dir = root.join(dir_name);
            std::fs::create_dir_all(&crate_dir).expect("member dir");
            if let Some(pkg) = pkg_name {
                std::fs::write(
                    crate_dir.join("Cargo.toml"),
                    format!("[package]\nname = \"{pkg}\"\nversion = \"0.1.0\"\n"),
                )
                .expect("member manifest");
            }
        }
        (dir, root)
    }

    fn provide(root: &std::path::Path) -> Result<serde_json::Value, DataProviderError> {
        let mut ctx = Context::test_context(root.to_path_buf());
        RustReviewTargetsProvider.provide(&mut ctx)
    }

    #[serial_test::serial(fallback_breadcrumb)]
    #[test]
    fn payload_lists_every_member_with_package_names() {
        let (_dir, root) = scratch_workspace(
            "\"crates/*\"",
            &[
                ("crates/core", Some("ops-core")),
                ("crates/cli", Some("ops-cli")),
                ("crates/not-a-crate", None),
            ],
        );
        let value = provide(&root).expect("provide");
        assert_eq!(
            value,
            serde_json::json!({
                "skill": "code-review-rust",
                "targets": [
                    { "name": "ops-cli", "path": "crates/cli" },
                    { "name": "ops-core", "path": "crates/core" }
                ]
            })
        );
    }

    /// TEST-5 / TASK-1816: the payload's real contract is the consumer's
    /// type, not a hand-written JSON literal. Decoding it here turns a field
    /// rename or shape change on `ReviewTargets` into a test failure instead
    /// of a runtime failure in the middle of writing backlog files.
    #[test]
    fn payload_decodes_into_the_consumers_review_targets_type() {
        let (_dir, root) = scratch_workspace(
            "\"crates/*\"",
            &[
                ("crates/core", Some("ops-core")),
                ("crates/cli", Some("ops-cli")),
            ],
        );
        let value = provide(&root).expect("provide");
        let decoded: ops_create_review_tasks::ReviewTargets =
            serde_json::from_value(value).expect("payload must decode into ReviewTargets");
        assert_eq!(decoded.skill, SKILL_NAME);
        let pairs: Vec<(&str, &str)> = decoded
            .targets
            .iter()
            .map(|t| (t.name.as_str(), t.path.as_str()))
            .collect();
        assert_eq!(
            pairs,
            vec![("ops-cli", "crates/cli"), ("ops-core", "crates/core")]
        );
    }

    /// TEST-5 / TASK-1816: the registration key is a string constant shared
    /// across a crate boundary; a mismatch degrades into a `NotFound` the
    /// engine reports as a missing extension.
    #[test]
    fn provider_name_matches_the_engines_registration_key() {
        assert_eq!(
            RustReviewTargetsProvider.name(),
            ops_create_review_tasks::DATA_PROVIDER_NAME
        );
    }

    /// A member without a usable manifest must not disappear — it falls back
    /// to the capitalized display name so it still gets a review subtask.
    #[serial_test::serial(fallback_breadcrumb)]
    #[test]
    fn member_without_manifest_falls_back_to_display_name() {
        let (_dir, root) = scratch_workspace("\"crates/orphan\"", &[("crates/orphan", None)]);
        let value = provide(&root).expect("provide");
        assert_eq!(
            value["targets"][0],
            serde_json::json!({ "name": "Orphan", "path": "crates/orphan" })
        );
    }

    #[serial_test::serial(fallback_breadcrumb)]
    #[test]
    fn member_with_malformed_manifest_falls_back_to_display_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/broken\"]\n",
        )
        .expect("root manifest");
        std::fs::create_dir_all(root.join("crates/broken")).expect("member dir");
        std::fs::write(
            root.join("crates/broken/Cargo.toml"),
            "[package\nname = \"unterminated\n",
        )
        .expect("malformed manifest");
        let value = provide(root).expect("provide");
        assert_eq!(
            value["targets"][0],
            serde_json::json!({ "name": "Broken", "path": "crates/broken" })
        );
    }

    /// PATTERN-1 / TASK-1839: two members whose last path segment is equal
    /// and whose manifests are unparseable both take the display-name
    /// fallback; the emitted names must still address distinct crates.
    #[serial_test::serial(fallback_breadcrumb)]
    #[test]
    fn same_leaf_named_members_get_distinct_target_names() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/parser\", \"tools/parser\"]\n",
        )
        .expect("root manifest");
        for member in ["crates/parser", "tools/parser"] {
            std::fs::create_dir_all(root.join(member)).expect("member dir");
            std::fs::write(
                root.join(member).join("Cargo.toml"),
                "[package\nname = \"x\n",
            )
            .expect("malformed manifest");
        }
        let value = provide(root).expect("provide");
        let decoded: ops_create_review_tasks::ReviewTargets =
            serde_json::from_value(value).expect("decode");
        let names: Vec<&str> = decoded.targets.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names.len(), 2, "both members must survive");
        assert_ne!(
            names[0], names[1],
            "colliding display names must be disambiguated, got {names:?}"
        );
        for name in names {
            assert!(
                name.contains("parser"),
                "disambiguated name must still name the member path, got {name}"
            );
        }
    }

    /// ERR-6 / TASK-1812: a `Cargo.toml` with `[package]` and no
    /// `[workspace]` is an ordinary single-package project, not an empty
    /// review run.
    #[test]
    fn single_package_project_yields_the_root_package_as_the_only_target() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"solo\"\nversion = \"0.1.0\"\n",
        )
        .expect("root manifest");
        let value = provide(root).expect("provide");
        let decoded: ops_create_review_tasks::ReviewTargets =
            serde_json::from_value(value).expect("decode");
        assert_eq!(decoded.skill, SKILL_NAME);
        assert_eq!(decoded.targets.len(), 1);
        assert_eq!(decoded.targets[0].name, "solo");
        assert_eq!(decoded.targets[0].path, ROOT_PACKAGE_PATH);
    }

    /// ERR-6 / TASK-1812: neither shape is a typed error naming the actual
    /// condition — never an empty target list.
    #[test]
    fn manifest_with_neither_workspace_nor_package_is_a_typed_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[profile.release]\nopt-level = 3\n",
        )
        .expect("root manifest");
        let err = provide(root).expect_err("must fail");
        let rendered = err.to_string();
        assert!(
            rendered.contains("[workspace]") && rendered.contains("[package]"),
            "error must name the missing tables, got: {rendered}"
        );
    }

    /// No Cargo.toml anywhere at or above the start: the provider must
    /// surface the typed workspace-root error, not an empty target list
    /// (which the engine would misread as "nothing to review").
    #[test]
    fn missing_workspace_manifest_is_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let result = provide(dir.path());
        let err = result.expect_err("must fail");
        assert!(
            err.to_string().contains("Cargo.toml"),
            "error must mention the missing manifest, got: {err:#}"
        );
    }

    /// DUP-3 / TASK-1814 + SEC-14 / TASK-1246: an unsafe member entry that
    /// reaches the name resolver is rejected before any join, so no manifest
    /// outside the workspace root is read — proven by planting a readable
    /// manifest at the escape target and asserting its package name never
    /// surfaces.
    #[test]
    fn unsafe_member_entries_are_never_read_outside_the_workspace_root() {
        let dir = tempfile::tempdir().expect("tempdir");
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(&outside).expect("outside dir");
        std::fs::write(
            outside.join("Cargo.toml"),
            "[package]\nname = \"escaped\"\nversion = \"0.1.0\"\n",
        )
        .expect("outside manifest");
        let root = dir.path().join("workspace");
        std::fs::create_dir_all(&root).expect("workspace dir");

        for member in ["../outside", outside.to_str().expect("utf-8 path")] {
            let name = member_target_name(member, &root);
            assert_ne!(
                name, "escaped",
                "unsafe member {member} must not be read outside the root"
            );
            assert_eq!(name, format_unit_name(member));
        }
    }

    /// SEC-11 / TASK-1822: the fallback breadcrumb carries the raw
    /// `[workspace].members` entry, which this process does not control.
    /// Captured from the real call site — a `%member` here would put the raw
    /// newline and ESC into the log stream and let a member entry forge a log
    /// record.
    #[serial_test::serial(fallback_breadcrumb)]
    #[test]
    fn member_breadcrumb_debug_escapes_control_characters() {
        let member = "a\nb\u{1b}31mc";
        // TOML string escapes, not Rust `Debug` ones: `\u001B` is the ESC.
        let (_dir, root) = scratch_workspace(r#""a\nb\u001B31mc""#, &[(member, None)]);

        let buf = ops_about::test_support::TracingBuf::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf.clone())
            .with_max_level(tracing::Level::WARN)
            .with_ansi(false)
            .finish();
        let value = tracing::subscriber::with_default(subscriber, || provide(&root))
            .expect("provide must fall back, not fail");
        assert_eq!(
            value["targets"][0]["path"], member,
            "the member with the hostile name must be the one that fell back"
        );

        let logs = buf.captured();
        assert!(
            logs.contains("falling back to display name"),
            "the fallback breadcrumb must have been emitted, got: {logs:?}"
        );
        ops_about::test_support::assert_debug_escapes_control_chars(member);
        assert!(
            !logs.contains('\u{1b}'),
            "raw ANSI ESC leaked into the breadcrumb: {logs:?}"
        );
        assert!(
            logs.contains("\\n") && logs.contains("\\u{1b}"),
            "member must be rendered through the `?` formatter, got: {logs:?}"
        );
    }
}
