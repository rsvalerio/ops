//! Workspace-member glob resolution and member-path safety.
//!
//! ARCH-1 / TASK-1791: extracted from the former `query.rs`. Everything here is
//! a pure function over a parsed [`CargoToml`] and a workspace-root path — no
//! cache, no `Context`, no IO beyond the `read_dir` the glob expander needs.
//!
//! ## Why not `MetadataProvider`?
//!
//! `cargo metadata` is the canonical source for resolved workspace members,
//! but invoking it requires running `cargo` (slow, network-touching, and
//! requires a fully-resolvable lockfile). The about/identity/units/coverage
//! providers run on every `ops about` invocation and need to be cheap and
//! offline-tolerant. They therefore parse `Cargo.toml` directly and resolve
//! workspace globs with the static expander below. `MetadataProvider` is used
//! by `deps_provider` where dependency graph data is unavoidable.

use ops_cargo_toml::CargoToml;
use std::path::Path;

/// Resolve `[workspace].members` globs to concrete member paths, honoring
/// `[workspace].exclude`. Members without a `*` are passed through verbatim.
///
/// Supports the simple `prefix/*` shape Cargo workspaces use in practice.
/// More elaborate patterns (`prefix/*/suffix`, `**`, `?`, character classes)
/// are not expanded — they are passed through unchanged and a `tracing::warn!`
/// is emitted so the unsupported shape is visible in logs rather than silently
/// producing a wrong member list.
///
/// `workspace_root` must be the *resolved* workspace root (CL-3 / TASK-1762),
/// not the process cwd: glob expansion `read_dir`s `workspace_root/prefix`, and
/// passing a subdirectory silently expands every glob to nothing.
///
/// Public so sibling Rust-stack extension crates (e.g.
/// create-review-tasks-rust) can enumerate the same member view the about
/// providers use, instead of each maintaining a glob expander.
pub fn resolved_workspace_members(manifest: &CargoToml, workspace_root: &Path) -> Vec<String> {
    let Some(ws) = manifest.workspace.as_ref() else {
        return Vec::new();
    };

    let exclude: std::collections::HashSet<&str> = ws.exclude.iter().map(String::as_str).collect();

    let mut resolved = Vec::new();
    for member in &ws.members {
        // SEC-14 / TASK-1246 AC #2: reject absolute and `..`-traversal
        // member entries before they reach any join.
        if !member_path_is_workspace_safe(member) {
            tracing::warn!(
                member = %member,
                "SEC-14 / TASK-1246: workspace member is absolute or contains `..`; dropping"
            );
            continue;
        }
        match classify_member(member) {
            MemberShape::Literal => resolved.push(member.clone()),
            MemberShape::Unsupported => {
                tracing::warn!(
                    pattern = %member,
                    "workspace member glob shape not supported by ops about; passing through unchanged"
                );
                resolved.push(member.clone());
            }
            MemberShape::Glob { prefix } => {
                let parent = workspace_root.join(prefix);
                resolved.extend(expand_member_glob(member, &parent, workspace_root));
            }
        }
    }

    resolved.retain(|m| !exclude.contains(m.as_str()));
    resolved.sort();
    // PATTERN-1 (TASK-1042): dedup the resolved member list. Cargo treats
    // `[workspace].members` with set semantics — overlapping entries like
    // `members = ["crates/foo", "crates/*"]` resolve to a single `crates/foo`
    // crate. Without this dedup the about pipeline would double-count a member
    // in `module_count` (identity provider) and emit duplicate `ProjectUnit`s
    // in the units / coverage providers, diverging from `cargo metadata`.
    resolved.dedup();
    resolved
}

/// FN-1 / TASK-1156: classified shape of one `[workspace].members` entry,
/// dispatched as a state machine in [`resolved_workspace_members`].
enum MemberShape<'a> {
    /// Pass through verbatim (no glob characters).
    Literal,
    /// Pass through verbatim and emit a warn — shape isn't supported.
    Unsupported,
    /// Expand via `read_dir(workspace_root.join(prefix))`.
    Glob { prefix: &'a str },
}

/// Classify a `[workspace].members` entry into a [`MemberShape`]. Centralises
/// the metacharacter scan so [`resolved_workspace_members`] reads as a flat
/// dispatch instead of a nested-if state machine.
fn classify_member(member: &str) -> MemberShape<'_> {
    // ERR-5 / TASK-1491: split on the first `*` with `let-else` so the
    // happy path falls through without an `is_none()` + `.expect()`
    // round-trip whose "checked above" invariant a future edit could
    // silently invalidate. `split_once` also hands both halves out
    // directly, so no byte index is ever sliced back into `member`.
    let Some((prefix, after_star)) = member.split_once('*') else {
        // PATTERN-1 (TASK-0803): detect glob shapes that lack `*` but still
        // contain class/alternation metacharacters (`crates/{core,cli}`,
        // `crates/[abc]`).
        return if contains_unsupported_glob_meta(member) {
            MemberShape::Unsupported
        } else {
            MemberShape::Literal
        };
    };
    if is_unsupported_glob(member, after_star) {
        return MemberShape::Unsupported;
    }
    MemberShape::Glob { prefix }
}

/// Expand a `prefix/*` glob by walking `parent` and returning UTF-8
/// workspace-relative paths to each subdirectory containing a `Cargo.toml`.
/// FN-1 / TASK-1156: extracted from [`resolved_workspace_members`] so the
/// orchestrator stays at the dispatch level and the `read_dir` + per-entry
/// boundary handling sits in one place.
pub fn expand_member_glob(member: &str, parent: &Path, workspace_root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(e) => {
            // ERR-7 (TASK-0941): Debug-format pattern / parent / error so
            // embedded newlines / ANSI escapes in attacker-controlled
            // `[workspace].members` entries cannot forge log records.
            tracing::warn!(
                pattern = ?member,
                parent = ?parent.display(),
                error = ?e,
                "workspace glob prefix unreadable; member skipped"
            );
            return out;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(
                    parent = ?parent.display(),
                    error = ?e,
                    "workspace glob entry unreadable; skipped"
                );
                continue;
            }
        };
        let path = entry.path();
        if !(path.is_dir() && path.join("Cargo.toml").exists()) {
            continue;
        }
        let Ok(rel) = path.strip_prefix(workspace_root) else {
            continue;
        };
        // READ-5 (TASK-0946): non-UTF-8 member paths must not be lossily
        // collapsed to U+FFFD.
        match rel.to_str() {
            Some(s) => out.push(s.to_string()),
            None => {
                tracing::warn!(
                    parent = ?parent.display(),
                    relpath = ?rel,
                    "workspace glob member relpath is not valid UTF-8; skipping"
                );
            }
        }
    }
    out
}

/// Returns true if the glob shape goes beyond a single trailing `*` after
/// the prefix — anything we cannot expand correctly with the simple
/// `read_dir(prefix)` approach.
///
/// PATTERN-1 (TASK-0803): also flag character-class closers (`]`) and brace
/// alternation (`{`, `}`). A pattern like `crates/{core,cli}` lacks `*`,
/// `?`, and `[`, so without these checks it would slip through as
/// "supported" and silently produce an empty member list when `read_dir`
/// failed on the literal-as-directory path.
fn is_unsupported_glob(member: &str, after_star: &str) -> bool {
    if !after_star.is_empty() {
        return true;
    }
    contains_unsupported_glob_meta(member)
}

fn contains_unsupported_glob_meta(member: &str) -> bool {
    member.contains(['?', '[', ']', '{', '}'])
}

/// Whether a `[workspace].members` entry is safe to join onto the root.
///
/// SEC-14 / TASK-1246: a workspace member must be a relative path with
/// no `..` segments. `Path::join` discards the root when the operand is
/// absolute and walks parents on `..`, so a hostile root `Cargo.toml`
/// could otherwise drive `read_capped_to_string` and tracing
/// breadcrumbs at any filesystem location reachable from the workspace
/// root. Rejecting those shapes up front matches the
/// `append_tree_directory` (SEC-14 / TASK-0811) and `scrub_path_segments`
/// (SEC-14 / TASK-1111) policies on the rendering side.
#[must_use]
pub fn member_path_is_workspace_safe(member: &str) -> bool {
    use std::path::Component;
    let p = Path::new(member);
    if p.is_absolute() {
        return false;
    }
    // Reject any segment equal to `..`. We accept `.` segments because
    // they are inert under `Path::join` and Cargo itself emits them in
    // some manifests (e.g. `members = ["./crates/foo"]`).
    !p.components().any(|c| matches!(c, Component::ParentDir))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ERR-7 (TASK-0941) / TEST-25 (TASK-1773): the glob expander's
    /// unreadable-prefix breadcrumb must Debug-format the attacker-controlled
    /// `[workspace].members` pattern, so embedded newlines / ANSI escapes
    /// cannot forge log records.
    ///
    /// This drives `expand_member_glob` itself and asserts on the *captured*
    /// log line: swapping `pattern = ?member` for `pattern = %member` makes it
    /// fail. The previous shape asserted only that `std`'s
    /// `Debug for Path::Display` escapes control characters, which stayed green
    /// through exactly that regression.
    #[test]
    fn glob_prefix_warn_debug_escapes_control_characters() {
        let dir = tempfile::tempdir().expect("tempdir");
        let member = "a\nb\u{1b}[31mc/*";
        let buf = ops_about::test_support::TracingBuf::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf.clone())
            .with_max_level(tracing::Level::WARN)
            .with_ansi(false)
            .finish();

        // The parent does not exist, so `read_dir` fails and the warn fires.
        let expanded = tracing::subscriber::with_default(subscriber, || {
            expand_member_glob(member, &dir.path().join("a\nb\u{1b}[31mc"), dir.path())
        });
        assert!(expanded.is_empty(), "unreadable prefix expands to nothing");

        let logs = buf.captured();
        assert!(
            logs.contains("workspace glob prefix unreadable"),
            "expected the unreadable-prefix warn, got: {logs}"
        );
        assert!(
            !logs.contains('\u{1b}'),
            "raw ESC must not reach the log line: {logs:?}"
        );
        assert!(
            logs.contains("\\n") && logs.contains("\\u{1b}"),
            "pattern must be Debug-escaped in the log line, got: {logs}"
        );
    }

    fn manifest_with_members(members: &[&str]) -> CargoToml {
        let toml_str = format!(
            "[workspace]\nmembers = [{}]\n",
            members
                .iter()
                .map(|m| format!("\"{m}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );
        toml::from_str(&toml_str).expect("parse manifest")
    }

    fn manifest_with_members_and_exclude(members: &[&str], exclude: &[&str]) -> CargoToml {
        let toml_str = format!(
            "[workspace]\nmembers = [{}]\nexclude = [{}]\n",
            members
                .iter()
                .map(|m| format!("\"{m}\""))
                .collect::<Vec<_>>()
                .join(", "),
            exclude
                .iter()
                .map(|m| format!("\"{m}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );
        toml::from_str(&toml_str).expect("parse manifest")
    }

    #[test]
    fn resolves_simple_glob() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        std::fs::create_dir_all(root.join("crates/foo")).unwrap();
        std::fs::write(
            root.join("crates/foo/Cargo.toml"),
            "[package]\nname=\"foo\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("crates/bar")).unwrap();
        std::fs::write(
            root.join("crates/bar/Cargo.toml"),
            "[package]\nname=\"bar\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("crates/not-a-crate")).unwrap();

        let manifest = manifest_with_members(&["crates/*"]);
        let resolved = resolved_workspace_members(&manifest, root);

        assert_eq!(
            resolved,
            vec!["crates/bar".to_string(), "crates/foo".to_string()]
        );
    }

    /// PATTERN-1 (TASK-0803): unsupported glob shapes (brace alternation,
    /// character classes, `?`) must pass through unchanged so downstream
    /// rendering surfaces them as-is rather than producing a silently-empty
    /// member list.
    #[test]
    fn unsupported_glob_shapes_pass_through() {
        let root = std::path::Path::new("/nonexistent");
        for pattern in [
            "crates/{core,cli}",
            "crates/[a-z]*",
            "crates/foo?",
            "crates/foo]",
        ] {
            let manifest = manifest_with_members(&[pattern]);
            let resolved = resolved_workspace_members(&manifest, root);
            assert_eq!(
                resolved,
                vec![pattern.to_string()],
                "expected `{pattern}` to pass through unchanged"
            );
        }
    }

    #[test]
    fn passthrough_non_glob_members() {
        let manifest = manifest_with_members(&["crates/core", "crates/cli"]);
        let resolved = resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent"));
        assert_eq!(
            resolved,
            vec!["crates/cli".to_string(), "crates/core".to_string()]
        );
    }

    #[test]
    fn empty_when_no_workspace() {
        let manifest: CargoToml =
            toml::from_str("[package]\nname=\"x\"\nversion=\"0.1.0\"\n").expect("parse");
        let resolved = resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent"));
        assert!(resolved.is_empty());
    }

    #[test]
    fn nonexistent_glob_parent_yields_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let manifest = manifest_with_members(&["crates/*"]);
        let resolved = resolved_workspace_members(&manifest, dir.path());
        assert!(resolved.is_empty());
    }

    #[test]
    fn exclude_filters_explicit_members() {
        let manifest = manifest_with_members_and_exclude(
            &["crates/core", "crates/experimental"],
            &["crates/experimental"],
        );
        let resolved = resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent"));
        assert_eq!(resolved, vec!["crates/core".to_string()]);
    }

    #[test]
    fn exclude_filters_glob_results() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        for name in ["foo", "bar", "experimental"] {
            std::fs::create_dir_all(root.join(format!("crates/{name}"))).unwrap();
            std::fs::write(
                root.join(format!("crates/{name}/Cargo.toml")),
                "[package]\nname=\"x\"\n",
            )
            .unwrap();
        }

        let manifest = manifest_with_members_and_exclude(&["crates/*"], &["crates/experimental"]);
        let resolved = resolved_workspace_members(&manifest, root);
        assert_eq!(
            resolved,
            vec!["crates/bar".to_string(), "crates/foo".to_string()]
        );
    }

    /// Suffix-after-`*` (e.g. `crates/*/sub`) is not supported by the simple
    /// expander. The pattern is passed through unchanged with a warn-log
    /// rather than silently producing a wrong member list (TASK-0410).
    #[test]
    fn unsupported_suffix_after_star_passes_through() {
        let manifest = manifest_with_members(&["crates/*/sub"]);
        let resolved = resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent"));
        assert_eq!(resolved, vec!["crates/*/sub".to_string()]);
    }

    /// PATTERN-1 (TASK-1042): overlapping `[workspace].members` entries
    /// (literal + glob covering the same crate) must collapse to a single
    /// resolved member. Cargo itself dedups, so the about pipeline must too —
    /// otherwise `module_count` and the units / coverage providers would
    /// double-count the duplicated crate.
    #[test]
    fn duplicate_member_from_literal_and_glob_is_deduped() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        std::fs::create_dir_all(root.join("crates/foo")).unwrap();
        std::fs::write(
            root.join("crates/foo/Cargo.toml"),
            "[package]\nname=\"foo\"\n",
        )
        .unwrap();

        let manifest = manifest_with_members(&["crates/foo", "crates/*"]);
        let resolved = resolved_workspace_members(&manifest, root);

        assert_eq!(resolved, vec!["crates/foo".to_string()]);
    }

    #[test]
    fn unsupported_globstar_passes_through() {
        let manifest = manifest_with_members(&["crates/**"]);
        let resolved = resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent"));
        assert_eq!(resolved, vec!["crates/**".to_string()]);
    }

    /// SEC-14 / TASK-1246: absolute and `..`-traversal member entries are
    /// dropped before any join.
    #[test]
    fn member_path_safety_rejects_absolute_and_traversal() {
        assert!(member_path_is_workspace_safe("crates/foo"));
        assert!(member_path_is_workspace_safe("./crates/foo"));
        assert!(!member_path_is_workspace_safe("/abs"));
        assert!(!member_path_is_workspace_safe("../escape"));
        assert!(!member_path_is_workspace_safe("crates/../../escape"));
    }
}
