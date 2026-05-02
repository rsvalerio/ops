//! Stack detection and stack-specific defaults.
//!
//! A "stack" represents a language, framework, or toolchain (Rust, Node, Go, etc.).
//! Each stack has:
//! - Manifest files used for detection (Cargo.toml, package.json, go.mod)
//! - Default commands (build, test, lint) loaded from embedded `.default.<stack>.ops.toml`
//! - Default data directory location

use crate::config::{CommandSpec, Config};
use indexmap::IndexMap;
use std::path::Path;

/// SEC-25: probe a manifest path with `try_exists` so transient errors are
/// logged rather than silently swallowed by `Path::exists`. Permission errors
/// or other I/O failures are treated as "not found" for detection purposes
/// (a wrong stack default for one CLI invocation is acceptable), but they are
/// surfaced via `tracing::debug` so a user investigating mis-detection has a
/// breadcrumb to follow.
fn manifest_present(path: &Path) -> bool {
    match path.try_exists() {
        Ok(present) => present,
        Err(err) => {
            tracing::debug!(
                path = %path.display(),
                error = %err,
                "stack manifest probe failed; treating as not present",
            );
            false
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "lowercase")]
pub enum Stack {
    Rust,
    Node,
    Go,
    Python,
    Terraform,
    Ansible,
    #[strum(serialize = "java-maven")]
    JavaMaven,
    #[strum(serialize = "java-gradle")]
    JavaGradle,
    Generic,
}

impl Stack {
    pub fn as_str(&self) -> &'static str {
        (*self).into()
    }

    pub fn manifest_files(&self) -> &[&str] {
        self.metadata().0
    }

    /// File extensions used for extension-based detection (in addition to exact manifest files).
    fn manifest_extensions(&self) -> &[&str] {
        match self {
            Stack::Terraform => &["tf"],
            _ => &[],
        }
    }

    /// Whether this stack has a manifest (exact filename or extension match) in `dir`.
    fn has_manifest_in_dir(&self, dir: &Path) -> bool {
        if self
            .manifest_files()
            .iter()
            .any(|f| manifest_present(&dir.join(f)))
        {
            return true;
        }
        let extensions = self.manifest_extensions();
        if !extensions.is_empty() {
            if let Ok(entries) = dir.read_dir() {
                for entry in entries.flatten() {
                    if let Some(ext) = entry.path().extension() {
                        if extensions.iter().any(|e| ext == *e) {
                            return true;
                        }
                    }
                }
            }
        }
        false
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
                    let accepted = Self::ACCEPTED_NAMES.join(", ");
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

    /// Stack names accepted in `config.stack` overrides, used in diagnostics
    /// when an unrecognised value is rejected.
    const ACCEPTED_NAMES: &'static [&'static str] = &[
        "rust",
        "node",
        "go",
        "python",
        "terraform",
        "ansible",
        "java-maven",
        "java-gradle",
        "generic",
    ];

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
        // Priority order for detection (Generic is excluded — no manifest files).
        const DETECT_ORDER: &[Stack] = &[
            Stack::Rust,
            Stack::Node,
            Stack::Go,
            Stack::Python,
            Stack::Terraform,
            Stack::Ansible,
            Stack::JavaGradle,
            Stack::JavaMaven,
        ];

        // SEC-25 / TASK-0902: canonicalize once so the `pop()` walk operates
        // on the resolved chain. Reaching the cwd through a symlink would
        // otherwise let lexical `..` traversal yield ancestors outside the
        // canonical workspace, picking up a sibling project's manifests.
        // If canonicalization fails (missing dir, EACCES on a parent), fall
        // back to the lexical walk and leave a debug breadcrumb so an
        // operator chasing odd detection can correlate.
        let mut current = match std::fs::canonicalize(start) {
            Ok(p) => p,
            Err(e) => {
                tracing::debug!(
                    path = %start.display(),
                    error = %e,
                    "Stack::detect could not canonicalize start; falling back to lexical walk"
                );
                start.to_path_buf()
            }
        };
        for _ in 0..Self::MAX_DETECT_DEPTH {
            if let Some(&stack) = DETECT_ORDER
                .iter()
                .find(|s| s.has_manifest_in_dir(&current))
            {
                return Some(stack);
            }
            if !current.pop() {
                return None;
            }
        }
        None
    }

    /// Embedded TOML content for this stack's default commands, or None for Generic.
    fn default_commands_toml(&self) -> Option<&'static str> {
        self.metadata().1
    }

    /// Single source of truth for per-stack metadata: (manifest_files, default_commands_toml).
    ///
    /// Consolidates two parallel match blocks (CD-11) so that adding a new stack
    /// requires updating exactly one match arm.
    fn metadata(&self) -> (&[&str], Option<&'static str>) {
        // Reduces the include_str!(concat!(env!(...), "/src/", file)) boilerplate to one line per arm.
        macro_rules! meta {
            ($files:expr, $toml:literal) => {
                (
                    $files as &[&str],
                    Some(include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/src/",
                        $toml
                    ))),
                )
            };
        }
        match self {
            Stack::Rust => meta!(&["Cargo.toml"], ".default.rust.ops.toml"),
            Stack::Node => meta!(&["package.json"], ".default.node.ops.toml"),
            Stack::Go => meta!(&["go.mod"], ".default.go.ops.toml"),
            Stack::Python => meta!(
                &["pyproject.toml", "setup.py", "requirements.txt"],
                ".default.python.ops.toml"
            ),
            Stack::Terraform => meta!(&["main.tf", "terraform.tf"], ".default.terraform.ops.toml"),
            Stack::Ansible => meta!(
                &["site.yml", "playbook.yml", "ansible.cfg"],
                ".default.ansible.ops.toml"
            ),
            Stack::JavaMaven => meta!(&["pom.xml"], ".default.java-maven.ops.toml"),
            Stack::JavaGradle => meta!(
                &["build.gradle", "build.gradle.kts"],
                ".default.java-gradle.ops.toml"
            ),
            Stack::Generic => (&[], None),
        }
    }

    pub fn default_commands(&self) -> IndexMap<String, CommandSpec> {
        let toml = match self.default_commands_toml() {
            Some(t) => t,
            None => return IndexMap::new(),
        };
        // `.default.<stack>.ops.toml` is `include_str!`-embedded at compile
        // time and validated by [`tests::all_embedded_default_tomls_parse`].
        // A parse failure here means the CI gate was skipped. Log at warn
        // instead of panicking so a bad default TOML degrades gracefully
        // (empty command map) rather than aborting the process.
        let config: Config = match toml::from_str(toml) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    stack = ?self,
                    error = %e,
                    "embedded default commands TOML failed to parse; returning empty command map"
                );
                return IndexMap::new();
            }
        };
        config.commands
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
        use std::sync::{Arc, Mutex};
        use tracing_subscriber::fmt::MakeWriter;

        #[derive(Clone, Default)]
        struct BufWriter(Arc<Mutex<Vec<u8>>>);
        impl std::io::Write for BufWriter {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0.lock().expect("lock").extend_from_slice(b);
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl<'a> MakeWriter<'a> for BufWriter {
            type Writer = BufWriter;
            fn make_writer(&'a self) -> Self::Writer {
                self.clone()
            }
        }

        let buf = BufWriter::default();
        let captured = buf.0.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf)
            .with_max_level(tracing::Level::WARN)
            .with_ansi(false)
            .finish();

        let dir = tempfile::tempdir().expect("tempdir");
        tracing::subscriber::with_default(subscriber, || {
            let _ = Stack::resolve(Some("not-a-stack"), dir.path());
        });

        let captured = String::from_utf8(captured.lock().expect("lock").clone()).expect("utf8");
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
        use std::sync::{Arc, Mutex};
        use tracing_subscriber::fmt::MakeWriter;

        #[derive(Clone, Default)]
        struct BufWriter(Arc<Mutex<Vec<u8>>>);
        impl std::io::Write for BufWriter {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0.lock().expect("lock").extend_from_slice(b);
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl<'a> MakeWriter<'a> for BufWriter {
            type Writer = BufWriter;
            fn make_writer(&'a self) -> Self::Writer {
                self.clone()
            }
        }

        let buf = BufWriter::default();
        let captured = buf.0.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf)
            .with_max_level(tracing::Level::WARN)
            .with_ansi(false)
            .finish();

        let dir = tempfile::tempdir().expect("tempdir");
        tracing::subscriber::with_default(subscriber, || {
            let _ = Stack::resolve(Some("rust"), dir.path());
        });
        assert!(captured.lock().expect("lock").is_empty());
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
