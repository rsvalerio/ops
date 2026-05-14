//! Stack detection and stack-specific defaults.
//!
//! A "stack" represents a language, framework, or toolchain (Rust, Node, Go, etc.).
//! Each stack has:
//! - Manifest files used for detection (Cargo.toml, package.json, go.mod)
//! - Default commands (build, test, lint) loaded from embedded `.default.<stack>.ops.toml`
//! - Default data directory location

use crate::config::{CommandSpec, Config};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;
use strum::{IntoEnumIterator, VariantNames};

mod detect;
mod metadata;

/// READ-6 (TASK-1404): the enum is the single source of truth for both the
/// list of stacks accepted in `config.stack` overrides (`Stack::VARIANTS`,
/// derived by `strum::VariantNames`) and the priority order used by
/// `Stack::detect` (declaration order, iterated via `strum::EnumIter`).
/// Variant order matters: detection probes earlier variants first, so
/// `JavaGradle` is declared before `JavaMaven` to win on mixed Gradle/Maven
/// workspaces (see `gradle_prioritized_over_maven` test).
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    strum::EnumString,
    strum::IntoStaticStr,
    strum::EnumIter,
    strum::VariantNames,
)]
#[strum(serialize_all = "lowercase")]
pub enum Stack {
    Rust,
    Node,
    Go,
    Python,
    Terraform,
    Ansible,
    #[strum(serialize = "java-gradle")]
    JavaGradle,
    #[strum(serialize = "java-maven")]
    JavaMaven,
    Generic,
}

impl Stack {
    pub fn as_str(&self) -> &'static str {
        (*self).into()
    }

    pub fn manifest_files(&self) -> &[&str] {
        metadata::metadata(*self).0
    }

    /// DUP-001: Resolve stack from config override or auto-detection.
    ///
    /// Shared by `CommandRunner::new()` and `extensions::resolve_stack()`.
    ///
    /// ERR-2 (TASK-0540): an unparseable `config.stack` value emits a
    /// `tracing::warn!` event and a user-visible `ui::warn` listing accepted
    /// stack names before falling back to filesystem detection. Without the
    /// diagnostic the override is silently dropped and users debugging
    /// "wrong stack detected" have no signal that their config typo was
    /// rejected.
    pub fn resolve(config_stack: Option<&str>, workspace_root: &Path) -> Option<Self> {
        if let Some(raw) = config_stack {
            match raw.parse::<Self>() {
                Ok(stack) => return Some(stack),
                Err(_) => {
                    let accepted = Self::VARIANTS.join(", ");
                    tracing::warn!(
                        value = raw,
                        accepted = %accepted,
                        "unrecognised stack override in config; falling back to detection"
                    );
                    crate::ui::warn(format!(
                        "config.stack = \"{raw}\" is not a recognised stack; falling back to auto-detection (accepted: {accepted})"
                    ));
                }
            }
        }
        Self::detect(workspace_root)
    }

    /// Maximum number of parent directories `detect` walks before giving up.
    ///
    /// Stack manifests realistically live within a few dozen levels of any
    /// project root; capping the walk guards against pathological cwds
    /// (thousands of components on FUSE/network mounts) and accidental
    /// symlink loops above the cwd that could otherwise trigger an
    /// unbounded chain of `Path::join` + `exists` syscalls per CLI
    /// invocation.
    pub const MAX_DETECT_DEPTH: usize = 64;

    pub fn detect(start: &Path) -> Option<Self> {
        detect::detect(start)
    }

    /// Embedded TOML content for this stack's default commands, or None for Generic.
    fn default_commands_toml(&self) -> Option<&'static str> {
        metadata::metadata(*self).1
    }

    /// PERF-3 (TASK-1409): the parsed default-commands map is memoized
    /// per-process. First call eagerly parses every variant's embedded
    /// TOML once; subsequent calls clone from the cache (`IndexMap<String,
    /// CommandSpec>` clone is O(n) on entries, but avoids re-running the
    /// TOML parser on every `ops <cmd>` dispatch).
    pub fn default_commands(&self) -> IndexMap<String, CommandSpec> {
        Self::default_commands_cache()
            .get(self)
            .cloned()
            .unwrap_or_default()
    }

    fn default_commands_cache() -> &'static HashMap<Stack, IndexMap<String, CommandSpec>> {
        static CACHE: OnceLock<HashMap<Stack, IndexMap<String, CommandSpec>>> = OnceLock::new();
        CACHE.get_or_init(|| {
            Self::iter()
                .map(|stack| {
                    let commands = match stack.default_commands_toml() {
                        Some(toml) => {
                            parse_default_commands(stack, toml, &mut std::io::stderr().lock())
                        }
                        None => IndexMap::new(),
                    };
                    (stack, commands)
                })
                .collect()
        })
    }
}

/// Parse an embedded `.default.<stack>.ops.toml` payload, falling back to an
/// empty `IndexMap` on parse failure.
///
/// ERR-1 (TASK-1413): the embedded TOML is validated by
/// [`tests::all_embedded_default_tomls_parse`]; reaching the failure branch
/// at runtime means the CI gate was bypassed and the next `ops init` would
/// otherwise scaffold an empty command section with no operator-visible
/// signal. Emit both a structured `tracing::warn` (for logs / debugging) and
/// a `crate::ui::warn` so the user sees the failure on stderr regardless of
/// `OPS_LOG_LEVEL`. The `ui_writer` parameter is the test seam: production
/// routes through stderr, tests pass a `Vec<u8>` and assert the captured
/// output.
fn parse_default_commands<W: std::io::Write>(
    stack: Stack,
    toml: &str,
    ui_writer: &mut W,
) -> IndexMap<String, CommandSpec> {
    match toml::from_str::<Config>(toml) {
        Ok(c) => c.commands,
        Err(e) => {
            tracing::warn!(
                stack = ?stack,
                error = %e,
                "embedded default commands TOML failed to parse; returning empty command map"
            );
            crate::ui::emit_to(
                "warning",
                &format!(
                    "embedded default commands TOML for stack `{}` failed to parse: {e}; \
                     `ops init` will scaffold an empty command section",
                    stack.as_str()
                ),
                ui_writer,
            );
            IndexMap::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_from_str_roundtrip() {
        for stack in [
            Stack::Rust,
            Stack::Node,
            Stack::Go,
            Stack::Python,
            Stack::Terraform,
            Stack::Ansible,
            Stack::JavaMaven,
            Stack::JavaGradle,
            Stack::Generic,
        ] {
            assert_eq!(stack.as_str().parse::<Stack>(), Ok(stack));
        }
    }

    #[test]
    fn stack_from_str_unknown() {
        assert!("unknown".parse::<Stack>().is_err());
    }

    /// ERR-7 (TASK-0945): tracing fields for stack-detection paths flow
    /// through the `?` formatter so an attacker-controlled CWD ancestor
    /// path containing newlines or ANSI escapes cannot forge log records.
    #[test]
    fn stack_detection_path_debug_escapes_control_characters() {
        let p = Path::new("a\nb\u{1b}[31mc/Cargo.toml");
        let rendered = format!("{:?}", p.display());
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains('\u{1b}'));
        assert!(rendered.contains("\\n"));
    }

    /// ERR-1 (TASK-0935): a per-entry `read_dir` error during the
    /// extension-based detection (e.g. Terraform `*.tf` walk) must leave a
    /// `tracing::debug` breadcrumb rather than silently falling through to
    /// the next stack. Mirrors the `manifest_present` policy and the
    /// TASK-0517 / TASK-0556 sweep. Tests the debug-level emission via the
    /// in-process fmt subscriber pattern already used by
    /// `resolve_unknown_config_override_emits_tracing_warning`.
    /// TEST-19 (TASK-1033): the chmod-0o000 mechanism this test uses to
    /// provoke a per-entry IO error only fires for non-root callers; root's
    /// DAC bypass means the locked dir is readable, no error is emitted,
    /// and the assertion `detected == Some(Stack::Terraform)` would pass
    /// for the wrong reason (it reduces to "Terraform detection still
    /// works at all"). Skipping under root keeps the breadcrumb-emission
    /// contract pinned to a meaningful fs configuration. DO NOT strip this
    /// guard without first replacing chmod with a deterministic error
    /// source (e.g. injected `read_dir` failure via a trait seam).
    #[cfg(unix)]
    #[test]
    fn extension_walk_per_entry_error_logs_debug_breadcrumb() {
        use std::os::unix::fs::PermissionsExt;

        if crate::test_utils::is_root_euid() {
            return;
        }

        // Drop a `.tf` file inside a 0o000 dir: read_dir on the *parent*
        // succeeds and yields the dir entry, but stat-ing it (or any deeper
        // operation triggered by readdir on some platforms) may surface as
        // a per-entry IO error. The contract under test is: even if this
        // particular fs configuration does not synthesize a per-entry
        // error, the function must not panic and must still resolve other
        // entries normally. The new explicit `match` arm is exercised
        // structurally by the source change; this test pins the behavioral
        // contract that detection remains monotonic.
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("main.tf"), "").expect("write tf");
        let locked = dir.path().join("locked");
        std::fs::create_dir(&locked).expect("locked dir");
        let mut perms = std::fs::metadata(&locked).unwrap().permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&locked, perms).unwrap();

        let (_log, detected) =
            crate::test_utils::capture_tracing(tracing::Level::DEBUG, || Stack::detect(dir.path()));

        // Restore permissions for tempdir cleanup.
        let mut restore = std::fs::metadata(&locked).unwrap().permissions();
        restore.set_mode(0o755);
        std::fs::set_permissions(&locked, restore).unwrap();

        assert_eq!(
            detected,
            Some(Stack::Terraform),
            "Terraform detection must remain monotonic; .tf in start dir wins"
        );
    }

    #[test]
    fn resolve_config_override_wins_over_detect() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Filesystem would detect Rust, but config override picks Node.
        std::fs::write(dir.path().join("Cargo.toml"), "").expect("write");
        let resolved = Stack::resolve(Some("node"), dir.path());
        assert_eq!(resolved, Some(Stack::Node));
    }

    #[test]
    fn resolve_unknown_config_override_falls_back_to_detect() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("Cargo.toml"), "").expect("write");
        let resolved = Stack::resolve(Some("not-a-real-stack"), dir.path());
        assert_eq!(resolved, Some(Stack::Rust));
    }

    /// ERR-2 (TASK-0540): a typo in `config.stack` must surface a tracing
    /// warning rather than silently fall through to detection. Captures the
    /// tracing fmt output in-process and asserts the offending value plus the
    /// list of accepted names appears.
    #[test]
    fn resolve_unknown_config_override_emits_tracing_warning() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (captured, ()) = crate::test_utils::capture_tracing(tracing::Level::WARN, || {
            let _ = Stack::resolve(Some("not-a-stack"), dir.path());
        });

        assert!(
            captured.contains("not-a-stack"),
            "warning must include offending value, got: {captured}"
        );
        assert!(
            captured.contains("rust") && captured.contains("generic"),
            "warning must list accepted stack names, got: {captured}"
        );
    }

    /// Companion to the typo case: an accepted value must not emit a warning.
    #[test]
    fn resolve_known_config_override_silent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (captured, ()) = crate::test_utils::capture_tracing(tracing::Level::WARN, || {
            let _ = Stack::resolve(Some("rust"), dir.path());
        });
        assert!(captured.is_empty());
    }

    #[test]
    fn resolve_none_config_falls_back_to_detect() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("go.mod"), "module x").expect("write");
        let resolved = Stack::resolve(None, dir.path());
        assert_eq!(resolved, Some(Stack::Go));
    }

    #[test]
    fn resolve_generic_override_returns_generic() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Even with a Rust manifest, explicit "generic" override takes precedence.
        std::fs::write(dir.path().join("Cargo.toml"), "").expect("write");
        let resolved = Stack::resolve(Some("generic"), dir.path());
        assert_eq!(resolved, Some(Stack::Generic));
    }

    #[test]
    fn resolve_none_and_no_manifest_returns_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(Stack::resolve(None, dir.path()), None);
    }

    #[test]
    fn rust_has_default_commands() {
        let cmds = Stack::Rust.default_commands();
        assert!(cmds.contains_key("build"));
        assert!(cmds.contains_key("test"));
        assert!(cmds.contains_key("verify"));
    }

    #[test]
    fn generic_has_no_default_commands() {
        let cmds = Stack::Generic.default_commands();
        assert!(cmds.is_empty());
    }

    #[test]
    fn detect_returns_none_without_manifest() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(Stack::detect(dir.path()), None);
    }

    #[test]
    fn detect_finds_cargo_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("Cargo.toml"), "").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Rust));
    }

    #[test]
    fn detect_finds_package_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("package.json"), "{}").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Node));
    }

    #[test]
    fn detect_finds_go_mod() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("go.mod"), "module test").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Go));
    }

    #[test]
    fn detect_finds_pyproject_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("pyproject.toml"), "").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Python));
    }

    #[test]
    fn detect_finds_terraform() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("main.tf"), "").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Terraform));
    }

    #[test]
    fn detect_finds_terraform_by_extension() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("network.tf"), "").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Terraform));
    }

    #[test]
    fn detect_finds_ansible() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("site.yml"), "").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Ansible));
    }

    #[test]
    fn detect_caps_walk_at_max_depth() {
        // ERR-1 / TASK-0529: pathological cwd depths must terminate. We
        // build a deep tree with no manifests and ensure detect bails out
        // after MAX_DETECT_DEPTH instead of walking to /.
        let dir = tempfile::tempdir().expect("tempdir");
        let mut deep = dir.path().to_path_buf();
        for i in 0..(Stack::MAX_DETECT_DEPTH + 4) {
            deep = deep.join(format!("d{i}"));
        }
        std::fs::create_dir_all(&deep).expect("create_dir_all");
        // No manifest exists in any ancestor inside the tempdir.
        assert_eq!(Stack::detect(&deep), None);
    }

    #[test]
    fn detect_walks_up_directories() {
        let dir = tempfile::tempdir().expect("tempdir");
        let subdir = dir.path().join("src").join("lib");
        std::fs::create_dir_all(&subdir).expect("create_dir");
        std::fs::write(dir.path().join("Cargo.toml"), "").expect("write");
        assert_eq!(Stack::detect(&subdir), Some(Stack::Rust));
    }

    /// SEC-25 / TASK-0902: detection through a symlinked cwd must resolve
    /// to the same Stack as the canonical path. Without canonicalization
    /// the lexical `..` walk would yield ancestors outside the canonical
    /// workspace and pick up an unrelated parent project's manifest.
    #[cfg(unix)]
    #[test]
    fn detect_resolves_symlinked_cwd_to_same_stack() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Real workspace: <dir>/real/Cargo.toml + <dir>/real/src/
        let real_root = dir.path().join("real");
        std::fs::create_dir_all(real_root.join("src")).expect("mkdir real/src");
        std::fs::write(real_root.join("Cargo.toml"), "").expect("write Cargo.toml");

        // Symlink: <dir>/alias -> <dir>/real
        let alias_root = dir.path().join("alias");
        std::os::unix::fs::symlink(&real_root, &alias_root).expect("symlink");

        // Detection through the alias must find the Rust workspace.
        let canonical = Stack::detect(&real_root.join("src"));
        let via_alias = Stack::detect(&alias_root.join("src"));
        assert_eq!(canonical, Some(Stack::Rust));
        assert_eq!(via_alias, canonical);
    }

    #[test]
    fn rust_prioritized_over_node() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("Cargo.toml"), "").expect("write");
        std::fs::write(dir.path().join("package.json"), "{}").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Rust));
    }

    #[test]
    fn detect_prioritizes_go_over_python() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("go.mod"), "module test").expect("write");
        std::fs::write(dir.path().join("pyproject.toml"), "").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Go));
    }

    #[test]
    fn detect_prioritizes_node_over_terraform() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("package.json"), "{}").expect("write");
        std::fs::write(dir.path().join("main.tf"), "").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Node));
    }

    // DUP-011 regression: detect must check all manifest_files()
    #[test]
    fn detect_finds_requirements_txt() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("requirements.txt"), "flask\n").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Python));
    }

    #[test]
    fn detect_finds_ansible_cfg() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("ansible.cfg"), "[defaults]\n").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Ansible));
    }

    #[test]
    fn detect_finds_pom_xml() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("pom.xml"), "<project/>").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::JavaMaven));
    }

    #[test]
    fn detect_finds_build_gradle() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("build.gradle"), "").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::JavaGradle));
    }

    #[test]
    fn detect_finds_build_gradle_kts() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("build.gradle.kts"), "").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::JavaGradle));
    }

    #[test]
    fn gradle_prioritized_over_maven() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("build.gradle"), "").expect("write");
        std::fs::write(dir.path().join("pom.xml"), "<project/>").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::JavaGradle));
    }

    /// ERR-5 regression: every embedded `.default.<stack>.ops.toml` must
    /// parse cleanly. `Stack::default_commands` panics if it does not, and
    /// that panic is reachable from production via `ops init`, so this test
    /// guards against regressions in the checked-in TOML files.
    #[test]
    fn all_embedded_default_tomls_parse() {
        for stack in [
            Stack::Rust,
            Stack::Node,
            Stack::Go,
            Stack::Python,
            Stack::Terraform,
            Stack::Ansible,
            Stack::JavaMaven,
            Stack::JavaGradle,
        ] {
            let toml = stack
                .default_commands_toml()
                .unwrap_or_else(|| panic!("stack {} must ship a default TOML", stack.as_str()));
            toml::from_str::<Config>(toml).unwrap_or_else(|e| {
                panic!("stack {} default TOML failed to parse: {e}", stack.as_str())
            });
        }
    }

    #[test]
    fn each_stack_default_toml_parses_and_includes_verify() {
        for stack in [
            Stack::Rust,
            Stack::Node,
            Stack::Go,
            Stack::Python,
            Stack::Terraform,
            Stack::Ansible,
            Stack::JavaMaven,
            Stack::JavaGradle,
        ] {
            let cmds = stack.default_commands();
            assert!(
                cmds.contains_key("verify"),
                "stack {} default TOML must define verify",
                stack.as_str()
            );
            assert!(
                !cmds.is_empty(),
                "stack {} default TOML must define at least one command",
                stack.as_str()
            );
        }
    }

    #[test]
    fn rust_clippy_aliased_to_lint() {
        let cmds = Stack::Rust.default_commands();
        let clippy = cmds.get("clippy").expect("clippy must exist");
        assert!(
            clippy.aliases().iter().any(|a| a == "lint"),
            "rust clippy must alias `lint`"
        );
        assert!(
            !cmds.contains_key("lint"),
            "rust must not define a duplicate top-level `lint` command"
        );
    }

    #[test]
    fn go_vet_aliased_to_lint() {
        let cmds = Stack::Go.default_commands();
        let vet = cmds.get("vet").expect("vet must exist");
        assert!(
            vet.aliases().iter().any(|a| a == "lint"),
            "go vet must alias `lint`"
        );
        assert!(
            !cmds.contains_key("lint"),
            "go must not define a duplicate top-level `lint` command"
        );
    }

    #[test]
    fn python_defines_lint_composite() {
        let cmds = Stack::Python.default_commands();
        assert!(
            cmds.contains_key("lint"),
            "python must define `lint` composite"
        );
    }

    /// PERF-3 (TASK-1409): two back-to-back calls must return equal-content
    /// maps, exercising the OnceLock-cached parse path on the second call.
    /// PERF-3 (TASK-1410): repeat `detect()` calls with the same start
    /// path must not re-issue `std::fs::canonicalize`. Asserted via the
    /// `canonicalize_cache_contains` test seam: the cache is empty for the
    /// (unique tempdir) start before the first call, populated after.
    #[test]
    fn detect_canonicalize_memoized_per_start_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("Cargo.toml"), "").expect("write");
        assert!(
            !super::detect::canonicalize_cache_contains(dir.path()),
            "fresh tempdir must not be cached yet"
        );
        let first = Stack::detect(dir.path());
        assert!(
            super::detect::canonicalize_cache_contains(dir.path()),
            "first detect must populate the canonicalize cache for this start"
        );
        let second = Stack::detect(dir.path());
        assert_eq!(first, Some(Stack::Rust));
        assert_eq!(second, Some(Stack::Rust));
    }

    #[test]
    fn default_commands_memoized_returns_stable_content() {
        let first = Stack::Rust.default_commands();
        let second = Stack::Rust.default_commands();
        assert_eq!(first.len(), second.len());
        for (k, v) in &first {
            assert_eq!(
                second.get(k).map(|s| s.display_cmd_fallback()),
                Some(v.display_cmd_fallback()),
                "memoized default_commands diverged at key {k}"
            );
        }
    }

    /// ERR-1 (TASK-1413): when the embedded TOML fails to parse, the
    /// helper must emit a `ui::warn` message that names the stack and the
    /// parser error. Test exercises the `ui_writer` seam with a synthetic
    /// broken TOML payload.
    #[test]
    fn parse_default_commands_emits_ui_warn_on_parse_failure() {
        let mut buf: Vec<u8> = Vec::new();
        let map = super::parse_default_commands(Stack::Rust, "not [valid toml", &mut buf);
        assert!(map.is_empty(), "parse failure must degrade to empty map");
        let out = String::from_utf8(buf).expect("utf8");
        assert!(
            out.starts_with("ops: warning:"),
            "expected ui::warn prefix, got: {out}"
        );
        assert!(out.contains("rust"), "warn must name the stack, got: {out}");
    }

    #[test]
    fn every_stack_defines_qa() {
        for stack in [
            Stack::Rust,
            Stack::Node,
            Stack::Go,
            Stack::Python,
            Stack::Terraform,
            Stack::Ansible,
            Stack::JavaMaven,
            Stack::JavaGradle,
        ] {
            let cmds = stack.default_commands();
            assert!(
                cmds.contains_key("qa"),
                "stack {} default TOML must define qa",
                stack.as_str()
            );
        }
    }
}
