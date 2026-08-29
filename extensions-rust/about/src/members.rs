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
/// FEAT / TASK-2040: `exclude` entries carry globs too, and are matched with
/// the same single-`*` semantics — see [`ExcludeSet`].
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

    let exclude = ExcludeSet::from_entries(&ws.exclude);

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

    resolved.retain(|m| !exclude.excludes(m));
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

/// FEAT / TASK-2040: `[workspace].exclude` entries, matched with the same
/// single-`*` semantics `[workspace].members` entries get.
///
/// Cargo accepts glob shapes in `exclude` exactly as it does in `members`, but
/// this module used to apply `exclude` as an exact string-set membership test
/// against the *resolved* member list. A glob exclude therefore matched no
/// resolved member and every directory the user meant to drop was still
/// counted — silently, and biased toward over-counting: `module_count`
/// (identity provider) and the `ProjectUnit` list (units / coverage providers)
/// diverged from `cargo metadata` with no warn to explain it.
///
/// Patterns are matched against the already-resolved member strings rather
/// than expanded on disk. An excluded path is by definition *not* a workspace
/// member, so `read_dir`-ing it would add IO and a second family of
/// unreadable-path failure modes without telling us anything the resolved list
/// does not already say.
///
/// FEAT / TASK-2055: literal entries match by *path segment*, not by string
/// equality, and a leading `./` on either side is ignored. Cargo excludes a
/// path together with everything under it — `exclude = ["crates/foo"]` also
/// drops a member listed as `crates/foo/bar` — and accepts a `./` prefix in
/// both lists.
struct ExcludeSet<'a> {
    /// Entries with no expandable `*`: matched as a path prefix, so the entry
    /// excludes itself and every member nested beneath it. Stored as a `Vec`
    /// rather than a set because matching is no longer an equality test and
    /// exclude lists are a handful of entries.
    literals: Vec<&'a str>,
    /// The text before the `*` of a single-`*` pattern. A member matches when
    /// it extends the prefix by exactly one path segment, which is what the
    /// `read_dir(prefix)` expansion means for `members` — so `crates/*`
    /// excludes `crates/foo` but not `crates/foo/bar`, and `crates/gen-*`
    /// excludes `crates/gen-a`. Unlike `members`, a partial-segment prefix is
    /// supported here: matching is string work, so it needs no directory to
    /// `read_dir`.
    prefixes: Vec<&'a str>,
}

impl<'a> ExcludeSet<'a> {
    fn from_entries(entries: &'a [String]) -> Self {
        let mut literals = Vec::new();
        let mut prefixes = Vec::new();
        for entry in entries {
            match entry.split_once('*') {
                Some((prefix, after_star)) if !is_unsupported_glob(entry, after_star) => {
                    prefixes.push(strip_dot_prefix(prefix));
                }
                // A shape we cannot expand (`**`, `?`, `[…]`, `{…}`) is kept
                // as a literal — the pre-TASK-2040 behaviour — and announced,
                // mirroring the `MemberShape::Unsupported` passthrough rather
                // than dropping the entry on the floor. ERR-7 (TASK-0941):
                // Debug-format the manifest-controlled pattern so embedded
                // newlines / ANSI escapes cannot forge log records.
                Some(_) => {
                    tracing::warn!(
                        pattern = ?entry,
                        "workspace exclude glob shape not supported by ops about; matching it literally"
                    );
                    literals.push(entry.as_str());
                }
                None => {
                    if contains_unsupported_glob_meta(entry) {
                        tracing::warn!(
                            pattern = ?entry,
                            "workspace exclude glob shape not supported by ops about; matching it literally"
                        );
                    }
                    literals.push(entry.as_str());
                }
            }
        }
        Self { literals, prefixes }
    }

    /// Whether `member` — a resolved, workspace-relative member path — is
    /// excluded.
    fn excludes(&self, member: &str) -> bool {
        let member = strip_dot_prefix(member);
        if self
            .literals
            .iter()
            .any(|entry| path_is_at_or_under(entry, member))
        {
            return true;
        }
        self.prefixes.iter().any(|prefix| {
            member
                .strip_prefix(prefix)
                .is_some_and(|rest| !rest.is_empty() && !rest.contains(std::path::is_separator))
        })
    }
}

/// FEAT / TASK-2055: drop any leading `./` segments so `./crates/foo` and
/// `crates/foo` compare equal. Cargo accepts either spelling in both
/// `[workspace].members` and `[workspace].exclude`, and this module compares
/// the two lists as strings, so an unnormalised `./` on one side alone used to
/// silently defeat the match.
fn strip_dot_prefix(path: &str) -> &str {
    let mut rest = path;
    while let Some(after_dot) = rest.strip_prefix('.') {
        let Some(after_separator) = after_dot.strip_prefix(std::path::is_separator) else {
            // `.hidden` or a bare `.` — not a `./` segment, leave it alone.
            break;
        };
        rest = after_separator.trim_start_matches(std::path::is_separator);
    }
    rest
}

/// Split a workspace-relative path into its meaningful segments, discarding
/// empty ones (repeated or trailing separators) and `.` segments.
fn path_segments(path: &str) -> impl Iterator<Item = &str> {
    path.split(std::path::is_separator)
        .filter(|segment| !segment.is_empty() && *segment != ".")
}

/// FEAT / TASK-2055: whether `path` *is* `ancestor` or is nested beneath it,
/// compared segment by segment. `cargo` excludes a listed path together with
/// everything under it, so `crates/foo` must also drop `crates/foo/bar` —
/// while never letting a partial segment match (`crates/foo` does not drop
/// `crates/foobar`, which whole-string prefix matching would).
///
/// An `ancestor` with no segments at all (`""`, `"."`, `"/"`) matches nothing:
/// treating it as the root would silently exclude the entire workspace.
fn path_is_at_or_under(ancestor: &str, path: &str) -> bool {
    let mut ancestor_segments = path_segments(ancestor).peekable();
    if ancestor_segments.peek().is_none() {
        return false;
    }
    let mut candidate = path_segments(path);
    ancestor_segments.all(|segment| candidate.next() == Some(segment))
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
    // The `*` must stand for a whole path segment: `expand_member_glob`
    // `read_dir`s `workspace_root.join(prefix)` and treats every child
    // directory as a match, which is only what Cargo means when the prefix is
    // empty (`*`) or ends at a separator (`crates/*`). A partial-segment
    // pattern like `crates/f*` would otherwise `read_dir` the non-existent
    // path `crates/f` and be reported as an unreadable prefix; classify it as
    // unsupported so the warn names the real problem.
    if prefix.is_empty() || prefix.ends_with(std::path::is_separator) {
        MemberShape::Glob { prefix }
    } else {
        MemberShape::Unsupported
    }
}

/// Expand a `prefix/*` glob by walking `parent` and returning UTF-8
/// workspace-relative paths to each subdirectory containing a `Cargo.toml`.
/// FN-1 / TASK-1156: extracted from [`resolved_workspace_members`] so the
/// orchestrator stays at the dispatch level and the `read_dir` + per-entry
/// boundary handling sits in one place.
pub fn expand_member_glob(member: &str, parent: &Path, workspace_root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    // SEC-14 / TASK-1246 (extended): resolve the workspace root once so each
    // directory entry can be tested for containment against a fully resolved
    // anchor. Unlike the ancestor walk in `find_workspace_root_strict` (see
    // TASK-2026), this check is *not* vacuous: `read_dir` hands back entries
    // that may themselves be symlinks pointing anywhere on the filesystem, so
    // `canonicalize` genuinely moves the path before it is compared.
    let canonical_root = match std::fs::canonicalize(workspace_root) {
        Ok(root) => root,
        Err(e) => {
            tracing::warn!(
                pattern = ?member,
                workspace_root = ?workspace_root.display(),
                error = ?e,
                "workspace root could not be resolved; glob member skipped"
            );
            return out;
        }
    };
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
        // SEC-14: a workspace member reached through a symlink must still
        // live inside the workspace. Resolve the entry, require containment
        // in the canonical root, and then keep using the *resolved* path for
        // the directory / `Cargo.toml` probes and the relative member string,
        // so every downstream manifest read follows the path we validated
        // rather than the symlink we were handed.
        let canonical = match std::fs::canonicalize(&path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    parent = ?parent.display(),
                    entry = ?path.display(),
                    error = ?e,
                    "workspace glob entry could not be resolved; skipped"
                );
                continue;
            }
        };
        if !canonical.starts_with(&canonical_root) {
            tracing::warn!(
                parent = ?parent.display(),
                entry = ?path.display(),
                resolved = ?canonical.display(),
                "workspace glob entry resolves outside the workspace root; skipped"
            );
            continue;
        }
        if !(canonical.is_dir() && canonical.join("Cargo.toml").exists()) {
            continue;
        }
        let Ok(rel) = canonical.strip_prefix(&canonical_root) else {
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
        // The parent does not exist, so `read_dir` fails and the warn fires.
        let (logs, expanded) =
            ops_about::test_support::capture_tracing(tracing::Level::WARN, || {
                expand_member_glob(member, &dir.path().join("a\nb\u{1b}[31mc"), dir.path())
            });
        assert!(expanded.is_empty(), "unreadable prefix expands to nothing");

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

    /// A `*` that stands for only part of a path segment (`crates/f*`) is not
    /// the `prefix/*` shape `expand_member_glob` implements: it used to be
    /// classified as a glob and `read_dir` the non-existent directory
    /// `crates/f`, reporting an "unreadable prefix" that misnamed the problem
    /// and dropped the entry. It must be classified as an unsupported shape
    /// and pass through unchanged instead — while the whole-segment shapes
    /// `crates/*` and `*` stay globs.
    #[test]
    fn partial_segment_glob_is_unsupported_and_passes_through() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::create_dir_all(root.join("crates/foo")).unwrap();
        std::fs::write(
            root.join("crates/foo/Cargo.toml"),
            "[package]\nname=\"foo\"\n",
        )
        .unwrap();

        for pattern in ["crates/f*", "crates*"] {
            assert!(
                matches!(classify_member(pattern), MemberShape::Unsupported),
                "`{pattern}` must classify as unsupported"
            );
            let manifest = manifest_with_members(&[pattern]);
            assert_eq!(
                resolved_workspace_members(&manifest, root),
                vec![pattern.to_string()],
                "expected `{pattern}` to pass through unchanged"
            );
        }

        // Whole-segment shapes still expand.
        for pattern in ["crates/*", "*"] {
            assert!(
                matches!(classify_member(pattern), MemberShape::Glob { .. }),
                "`{pattern}` must still classify as a glob"
            );
        }
        assert_eq!(
            resolved_workspace_members(&manifest_with_members(&["crates/*"]), root),
            vec!["crates/foo".to_string()]
        );
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

    /// FEAT / TASK-2040: a glob `exclude` used to match nothing, because
    /// exclusion was an exact string-set test against the resolved member
    /// list. `crates/generated-*` must drop exactly the members it names —
    /// whole-segment (`vendor/*`) and partial-segment (`crates/generated-*`)
    /// prefixes alike — and leave everything else in place.
    #[test]
    fn exclude_globs_drop_the_members_they_match() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        for path in [
            "crates/core",
            "crates/generated-a",
            "crates/generated-b",
            "vendor/one",
            "vendor/two",
        ] {
            std::fs::create_dir_all(root.join(path)).unwrap();
            std::fs::write(
                root.join(path).join("Cargo.toml"),
                "[package]\nname=\"x\"\n",
            )
            .unwrap();
        }

        let manifest = manifest_with_members_and_exclude(
            &["crates/*", "vendor/*"],
            &["crates/generated-*", "vendor/*"],
        );

        assert_eq!(
            resolved_workspace_members(&manifest, root),
            vec!["crates/core".to_string()],
            "glob excludes must drop the members they match"
        );
    }

    /// The exclude prefix must consume exactly one path segment, so a glob
    /// exclude cannot reach members it does not name: `crates/*` excludes
    /// `crates/foo` but neither `crates/foo/nested` nor `crates-extra/foo`.
    #[test]
    fn exclude_glob_matches_one_segment_only() {
        let exclude = vec!["crates/*".to_string()];
        let set = ExcludeSet::from_entries(&exclude);

        assert!(set.excludes("crates/foo"));
        assert!(!set.excludes("crates/foo/nested"));
        assert!(!set.excludes("crates-extra/foo"));
        assert!(!set.excludes("crates/"), "an empty segment is not a member");
        assert!(!set.excludes("crates"));
    }

    /// FEAT / TASK-2055 AC #1: Cargo excludes a literal path *and everything
    /// under it*, so `exclude = ["crates/foo"]` must also drop a member listed
    /// as `crates/foo/bar`. Whole-segment comparison is what makes that safe:
    /// a sibling whose name merely starts with the entry (`crates/foobar`)
    /// stays a member.
    #[test]
    fn literal_exclude_drops_nested_members() {
        let exclude = vec!["crates/foo".to_string()];
        let set = ExcludeSet::from_entries(&exclude);

        assert!(set.excludes("crates/foo"), "the entry itself");
        assert!(set.excludes("crates/foo/bar"), "a member nested one level");
        assert!(set.excludes("crates/foo/bar/baz"), "and deeper");
        assert!(
            !set.excludes("crates/foobar"),
            "a partial segment is a different crate"
        );
        assert!(!set.excludes("crates"), "an ancestor is not excluded");
        assert!(!set.excludes("other/foo"));
    }

    /// FEAT / TASK-2055 AC #1, end to end: the nested member must be gone from
    /// the resolved list `module_count` and the `ProjectUnit` providers read,
    /// not merely from `ExcludeSet::excludes`.
    #[test]
    fn nested_member_under_literal_exclude_is_resolved_away() {
        let manifest = manifest_with_members_and_exclude(
            &["crates/core", "crates/foo", "crates/foo/bar"],
            &["crates/foo"],
        );
        assert_eq!(
            resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent")),
            vec!["crates/core".to_string()],
        );
    }

    /// FEAT / TASK-2055 AC #2: Cargo accepts a `./` prefix in both `members`
    /// and `exclude`, so a leading `./` on either side must not defeat the
    /// match — including for glob entries, which match by string prefix.
    #[test]
    fn leading_dot_slash_does_not_prevent_a_match() {
        let literal = vec!["./crates/foo".to_string()];
        let set = ExcludeSet::from_entries(&literal);
        assert!(set.excludes("crates/foo"), "dotted entry, plain member");
        assert!(set.excludes("./crates/foo"), "dotted on both sides");
        assert!(set.excludes("./crates/foo/bar"), "and nested");

        let plain = vec!["crates/foo".to_string()];
        let set = ExcludeSet::from_entries(&plain);
        assert!(set.excludes("./crates/foo"), "plain entry, dotted member");

        let glob = vec!["./crates/*".to_string()];
        let set = ExcludeSet::from_entries(&glob);
        assert!(set.excludes("crates/foo"), "dotted glob entry");
        assert!(set.excludes("./crates/foo"), "dotted on both sides");
        assert!(!set.excludes("./crates/foo/nested"), "still one segment");
    }

    /// An exclude entry that normalises away to nothing (`.`, `./`, an empty
    /// string) must exclude *nothing*. Read as "the workspace root" it would
    /// silently drop every member.
    #[test]
    fn an_empty_exclude_entry_excludes_nothing() {
        for entry in ["", ".", "./", "/"] {
            let exclude = vec![entry.to_string()];
            let set = ExcludeSet::from_entries(&exclude);
            assert!(
                !set.excludes("crates/foo"),
                "exclude entry {entry:?} must not drop every member"
            );
        }
    }

    /// An exclude shape the expander cannot interpret (`**`, `?`, `[…]`,
    /// `{…}`) keeps the pre-TASK-2040 literal behaviour and says so, instead
    /// of being silently reinterpreted as a prefix that would over-exclude.
    #[test]
    fn unsupported_exclude_shapes_stay_literal_and_warn() {
        let exclude = vec![
            "crates/**".to_string(),
            "crates/{core,cli}".to_string(),
            "crates/foo?".to_string(),
        ];
        let (logs, excluded_literally) =
            ops_about::test_support::capture_tracing(tracing::Level::WARN, || {
                let set = ExcludeSet::from_entries(&exclude);
                (
                    set.excludes("crates/**"),
                    set.excludes("crates/core"),
                    set.excludes("crates/foo"),
                )
            });

        assert_eq!(
            excluded_literally,
            (true, false, false),
            "unsupported shapes must match literally, never as a prefix"
        );
        assert_eq!(
            logs.matches("workspace exclude glob shape not supported")
                .count(),
            3,
            "every unsupported exclude shape must be announced, got: {logs}"
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

    /// SEC-14 / TASK-1246 (extended): a glob member that is a symlink out of
    /// the workspace must be dropped, while its ordinary sibling still
    /// resolves. `..`-rejection only covers the *textual* member entries; a
    /// `crates/*` expansion never sees them, so containment has to be
    /// enforced on the resolved directory instead.
    #[cfg(unix)]
    #[test]
    fn glob_member_symlinked_outside_the_workspace_is_skipped() {
        let root = tempfile::tempdir().expect("root");
        let outside = tempfile::tempdir().expect("outside");

        let crates = root.path().join("crates");
        std::fs::create_dir_all(crates.join("inside")).expect("inside");
        std::fs::write(
            crates.join("inside/Cargo.toml"),
            "[package]\nname=\"inside\"\nversion=\"0.1.0\"\n",
        )
        .expect("inside manifest");

        let evil = outside.path().join("evil");
        std::fs::create_dir_all(&evil).expect("evil");
        std::fs::write(
            evil.join("Cargo.toml"),
            "[package]\nname=\"evil\"\nversion=\"0.1.0\"\n",
        )
        .expect("evil manifest");
        std::os::unix::fs::symlink(&evil, crates.join("escapee")).expect("symlink");

        let expanded = expand_member_glob("crates/*", &crates, root.path());
        assert_eq!(
            expanded,
            vec!["crates/inside".to_string()],
            "the symlinked-out member must not be expanded"
        );
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
