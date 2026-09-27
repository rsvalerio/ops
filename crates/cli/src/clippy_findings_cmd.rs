//! `ops clippy-findings`: survey-mode Clippy, one normalized row per
//! diagnostic.
//!
//! This is not a gate. It runs `cargo clippy --message-format=json` with the
//! caller's lint flags — never adding `-D warnings` — and prints a versioned
//! JSON report whose rows are stable finding identities:
//!
//! - the lint name without its `clippy::` prefix;
//! - the package as `name@version` plus its repo-relative manifest directory,
//!   never the raw `package_id` (which embeds the absolute checkout path);
//! - the target name and kind;
//! - the primary span's repo-relative file, line and column — a diagnostic
//!   with no span is attributed to the crate's `Cargo.toml` at line 0;
//! - the message, verbatim.
//!
//! Spans outside the repository (registry sources, the toolchain, `OUT_DIR`
//! and anything else under the target directory) are dropped and counted;
//! plain rustc lint warnings are counted separately. Rows are sorted and
//! de-duplicated, so the report of one commit is byte-identical from any
//! checkout path — Cargo compiles in parallel and emits in completion order,
//! and a library's lib and unit-test builds report the same diagnostic twice.

use std::collections::{BTreeSet, HashMap};
use std::io::{BufRead, BufReader, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// Version of the JSON report; bump on any incompatible row change.
pub const SCHEMA_VERSION: u32 = 1;

/// The `kind` discriminator of the JSON report.
const REPORT_KIND: &str = "clippy-findings";

/// How the survey selects the build: lockfile policy and feature set.
///
/// The default is the `clippy` gate's build (`--all-features`) under
/// `--locked`, so a survey never rewrites `Cargo.lock`: a stale or missing
/// lock makes Cargo refuse and the survey fail instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurveyOptions {
    /// Pass `--locked`.
    pub locked: bool,
    /// Pass `--all-features`. Off, Cargo builds the default features plus
    /// `features`, minus the defaults when `no_default_features` is set.
    pub all_features: bool,
    /// Pass `--no-default-features`.
    pub no_default_features: bool,
    /// Passed as `--features <list>` when non-empty.
    pub features: Vec<String>,
}

impl Default for SurveyOptions {
    fn default() -> Self {
        Self {
            locked: true,
            all_features: true,
            no_default_features: false,
            features: Vec::new(),
        }
    }
}

/// Default wait for `cargo metadata --no-deps` (no network, no build).
const METADATA_TIMEOUT: Duration = Duration::from_secs(60);

/// One Clippy diagnostic. Field order is the report's sort order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Finding {
    /// Repo-relative file of the primary span, `/`-separated; the crate's
    /// `Cargo.toml` for a spanless diagnostic.
    pub file: String,
    /// 1-based line, or 0 for a spanless diagnostic.
    pub line: u64,
    /// 1-based column, or 0 for a spanless diagnostic.
    pub column: u64,
    /// Lint name without the `clippy::` prefix.
    pub lint: String,
    /// `name@version`.
    pub package: String,
    /// Repo-relative manifest directory; `.` for a root package.
    pub manifest_dir: String,
    /// Target name (e.g. `ops`, `ops_core`).
    pub target: String,
    /// Target kinds, comma-joined (e.g. `lib`, `bin`, `test`).
    pub target_kind: String,
    /// The diagnostic message, verbatim.
    pub message: String,
}

/// The versioned JSON report printed on stdout.
#[derive(Debug, Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub kind: &'static str,
    pub findings: Vec<Finding>,
    /// Distinct Clippy diagnostics dropped because their span (or package)
    /// lies outside the repository.
    pub dropped_out_of_tree: usize,
    /// Distinct plain rustc lint warnings (not Clippy findings).
    pub rustc_warnings: usize,
}

/// A workspace member as the report names it.
#[derive(Debug, Clone)]
struct Package {
    id: String,
    dir: PathBuf,
}

/// The workspace layout needed to normalize Cargo's paths.
#[derive(Debug)]
pub struct Workspace {
    /// `workspace_root` exactly as Cargo reports it; relative span paths are
    /// relative to it.
    root: PathBuf,
    /// Target directory relative to `root`, when it lives inside it.
    target_rel: Option<PathBuf>,
    /// `root` relative to the repository toplevel (empty when they match).
    prefix: PathBuf,
    /// Members inside the repository, keyed by absolute manifest path.
    packages: HashMap<PathBuf, Package>,
}

#[derive(Deserialize)]
struct Metadata {
    workspace_root: PathBuf,
    target_directory: PathBuf,
    packages: Vec<MetadataPackage>,
}

#[derive(Deserialize)]
struct MetadataPackage {
    name: String,
    version: String,
    manifest_path: PathBuf,
}

impl Workspace {
    /// Build the layout from `cargo metadata --no-deps` JSON; `repo_prefix`
    /// maps the workspace root to its offset under the repository toplevel.
    fn from_metadata(json: &[u8], repo_prefix: impl FnOnce(&Path) -> PathBuf) -> Result<Self> {
        let meta: Metadata =
            serde_json::from_slice(json).context("parsing `cargo metadata` output")?;
        let prefix = repo_prefix(&meta.workspace_root);
        let target_rel = meta
            .target_directory
            .strip_prefix(&meta.workspace_root)
            .ok()
            .map(Path::to_path_buf);
        let mut ws = Self {
            root: meta.workspace_root,
            target_rel,
            prefix,
            packages: HashMap::new(),
        };
        for pkg in meta.packages {
            // A member outside the checkout (`path = "../shared"`) has no
            // repo-relative directory; its diagnostics count as out of tree.
            let Some(dir) = pkg.manifest_path.parent().and_then(|d| ws.in_tree(d)) else {
                continue;
            };
            ws.packages.insert(
                pkg.manifest_path,
                Package {
                    id: format!("{}@{}", pkg.name, pkg.version),
                    dir,
                },
            );
        }
        Ok(ws)
    }

    /// `path` relative to the repository, or `None` when it lies outside the
    /// workspace or inside its target directory (generated sources).
    fn in_tree(&self, path: &Path) -> Option<PathBuf> {
        let rel = if path.is_absolute() {
            path.strip_prefix(&self.root).ok()?
        } else {
            path
        };
        let rel = normalize(rel)?;
        if self.target_rel.as_ref().is_some_and(|t| rel.starts_with(t)) {
            return None;
        }
        Some(self.prefix.join(rel))
    }
}

/// Lexically normalize a relative path: drop `.`, reject `..` and any root
/// or prefix component, so the result can never escape the base it is
/// joined to.
fn normalize(rel: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for component in rel.components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(out)
}

/// `/`-joined rendering, identical on every platform.
fn slash(path: &Path) -> String {
    let parts: Vec<_> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect();
    parts.join("/")
}

#[derive(Deserialize)]
struct CargoMessage {
    reason: String,
    #[serde(default)]
    manifest_path: Option<PathBuf>,
    #[serde(default)]
    target: Option<Target>,
    #[serde(default)]
    message: Option<Diagnostic>,
}

#[derive(Deserialize)]
struct Target {
    name: String,
    #[serde(default)]
    kind: Vec<String>,
}

#[derive(Deserialize)]
struct Diagnostic {
    message: String,
    level: String,
    #[serde(default)]
    code: Option<Code>,
    #[serde(default)]
    spans: Vec<Span>,
}

#[derive(Deserialize)]
struct Code {
    code: String,
}

#[derive(Deserialize)]
struct Span {
    file_name: String,
    line_start: u64,
    column_start: u64,
    is_primary: bool,
}

/// Identity of a diagnostic that is counted rather than reported, so the
/// lib and unit-test builds of one crate count it once.
type CountKey = (Option<PathBuf>, String, String, u64, u64, String);

/// Accumulates Cargo's JSON message stream into a [`Report`].
#[derive(Default)]
pub struct Collector {
    findings: BTreeSet<Finding>,
    dropped: BTreeSet<CountKey>,
    rustc_warnings: BTreeSet<CountKey>,
}

impl Collector {
    /// Consume one line of `cargo --message-format=json` output.
    ///
    /// # Errors
    ///
    /// Returns an error when the line is not a JSON object — Cargo's JSON
    /// mode writes nothing else to stdout, so anything else means the
    /// stream is not what this parser was built for.
    pub fn push_line(&mut self, ws: &Workspace, line: &str) -> Result<()> {
        if line.trim().is_empty() {
            return Ok(());
        }
        let msg: CargoMessage =
            serde_json::from_str(line).context("parsing a cargo JSON message")?;
        self.push(ws, msg);
        Ok(())
    }

    fn push(&mut self, ws: &Workspace, msg: CargoMessage) {
        if msg.reason != "compiler-message" {
            return;
        }
        let Some(diag) = msg.message else { return };
        if !matches!(diag.level.as_str(), "warning" | "error") {
            return;
        }
        let primary = diag.spans.iter().find(|s| s.is_primary);
        let code = diag.code.map(|c| c.code);
        let key = |code: &str| -> CountKey {
            (
                msg.manifest_path.clone(),
                code.to_string(),
                primary.map_or_else(String::new, |s| s.file_name.clone()),
                primary.map_or(0, |s| s.line_start),
                primary.map_or(0, |s| s.column_start),
                diag.message.clone(),
            )
        };
        let Some(code) = code else {
            // Codeless warnings are Cargo's per-crate summaries ("`x` (lib)
            // generated 3 warnings") and similar notices, not lints.
            return;
        };
        let Some(lint) = code.strip_prefix("clippy::") else {
            if diag.level == "warning" {
                self.rustc_warnings.insert(key(&code));
            }
            return;
        };
        let Some(pkg) = msg.manifest_path.as_ref().and_then(|m| ws.packages.get(m)) else {
            self.dropped.insert(key(&code));
            return;
        };
        let (file, line, column) = if let Some(span) = primary {
            let Some(file) = ws.in_tree(Path::new(&span.file_name)) else {
                self.dropped.insert(key(&code));
                return;
            };
            (slash(&file), span.line_start, span.column_start)
        } else {
            (slash(&pkg.dir.join("Cargo.toml")), 0, 0)
        };
        let (target, target_kind) = msg
            .target
            .map_or_else(Default::default, |t| (t.name, t.kind.join(",")));
        let manifest_dir = if pkg.dir.as_os_str().is_empty() {
            ".".to_string()
        } else {
            slash(&pkg.dir)
        };
        self.findings.insert(Finding {
            file,
            line,
            column,
            lint: lint.to_string(),
            package: pkg.id.clone(),
            manifest_dir,
            target,
            target_kind,
            message: diag.message,
        });
    }

    /// The sorted, de-duplicated report.
    #[must_use]
    pub fn into_report(self) -> Report {
        Report {
            schema_version: SCHEMA_VERSION,
            kind: REPORT_KIND,
            findings: self.findings.into_iter().collect(),
            dropped_out_of_tree: self.dropped.len(),
            rustc_warnings: self.rustc_warnings.len(),
        }
    }
}

impl SurveyOptions {
    /// Build from the CLI's opt-out flags. Selecting features explicitly
    /// (`--features`, `--no-default-features`) drops `--all-features`, which
    /// would otherwise override them.
    #[must_use]
    pub const fn from_flags(
        no_locked: bool,
        no_all_features: bool,
        no_default_features: bool,
        features: Vec<String>,
    ) -> Self {
        Self {
            locked: !no_locked,
            all_features: !(no_all_features || no_default_features || !features.is_empty()),
            no_default_features,
            features,
        }
    }
}

/// The full `cargo` argument list for the survey.
fn clippy_args(opts: &SurveyOptions, lint_flags: &[String]) -> Vec<String> {
    let mut args: Vec<String> = vec!["clippy".into(), "--workspace".into()];
    if opts.locked {
        args.push("--locked".into());
    }
    if opts.all_features {
        args.push("--all-features".into());
    }
    if opts.no_default_features {
        args.push("--no-default-features".into());
    }
    if !opts.features.is_empty() {
        args.push("--features".into());
        args.push(opts.features.join(","));
    }
    args.push("--all-targets".into());
    args.push("--message-format=json".into());
    if !lint_flags.is_empty() {
        args.push("--".to_string());
        args.extend(lint_flags.iter().cloned());
    }
    args
}

/// The workspace root's offset under the git toplevel, or empty when there
/// is no repository (or the workspace is not inside it).
fn repo_prefix(ws_root: &Path) -> PathBuf {
    let toplevel = ops_core::subprocess::run_with_timeout(
        Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(ws_root),
        METADATA_TIMEOUT,
        "git rev-parse",
    )
    .ok()
    .filter(|out| out.status.success())
    .map(|out| PathBuf::from(String::from_utf8_lossy(&out.stdout).trim_end()));
    let (Some(toplevel), Ok(root)) = (toplevel, ws_root.canonicalize()) else {
        return PathBuf::new();
    };
    toplevel
        .canonicalize()
        .ok()
        .and_then(|top| root.strip_prefix(top).ok().and_then(normalize))
        .unwrap_or_default()
}

fn load_workspace(cwd: &Path) -> Result<Workspace> {
    let out = ops_core::subprocess::run_cargo(
        &["metadata", "--no-deps", "--format-version", "1"],
        cwd,
        METADATA_TIMEOUT,
        "cargo metadata",
    )?;
    if !out.status.success() {
        bail!(
            "cargo metadata failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Workspace::from_metadata(&out.stdout, repo_prefix)
}

/// Run the survey from `cwd` and print the JSON report on stdout.
///
/// Cargo's progress and any compile errors stream to stderr untouched.
///
/// # Errors
///
/// Fails when `cargo metadata` or `cargo clippy` cannot run, when clippy
/// exits non-zero (a compile error, or a `-D` lint flag the caller passed —
/// the finding set would be incomplete, so no report is printed; under
/// `--locked` this includes a stale or missing `Cargo.lock`), or when the
/// JSON stream does not parse.
pub fn run_clippy_findings(
    cwd: &Path,
    opts: &SurveyOptions,
    lint_flags: &[String],
    dry_run: bool,
) -> Result<ExitCode> {
    let args = clippy_args(opts, lint_flags);
    let cargo = ops_core::subprocess::resolve_cargo_bin();
    if dry_run {
        println!(
            "{} {}",
            cargo.to_string_lossy(),
            shlex::try_join(args.iter().map(String::as_str)).unwrap_or_else(|_| args.join(" "))
        );
        return Ok(ExitCode::SUCCESS);
    }

    let ws = load_workspace(cwd)?;
    let mut child = Command::new(&cargo)
        .args(&args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to spawn {}", cargo.to_string_lossy()))?;
    let stdout = child
        .stdout
        .take()
        .context("cargo clippy stdout was not captured")?;

    let mut collector = Collector::default();
    let mut parse_result = Ok(());
    for (index, line) in BufReader::new(stdout).lines().enumerate() {
        let line = line.context("reading cargo clippy output")?;
        if let Err(err) = collector.push_line(&ws, &line) {
            // Keep draining so the child never blocks on a full pipe.
            if parse_result.is_ok() {
                parse_result =
                    Err(err.context(format!("cargo output line {}", index.saturating_add(1))));
            }
        }
    }
    let status = child.wait().context("waiting for cargo clippy")?;
    parse_result?;
    if !status.success() {
        bail!("cargo clippy failed ({status}); no findings report emitted");
    }

    let report = collector.into_report();
    let mut out = std::io::stdout().lock();
    serde_json::to_writer_pretty(&mut out, &report).context("writing findings report")?;
    writeln!(out).context("writing findings report")?;
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(root: &str) -> String {
        serde_json::json!({
            "workspace_root": root,
            "target_directory": format!("{root}/target"),
            "packages": [
                {"name": "ops", "version": "0.65.0",
                 "manifest_path": format!("{root}/crates/cli/Cargo.toml")},
                {"name": "top", "version": "1.0.0",
                 "manifest_path": format!("{root}/Cargo.toml")},
                {"name": "shared", "version": "0.1.0",
                 "manifest_path": "/elsewhere/shared/Cargo.toml"}
            ]
        })
        .to_string()
    }

    fn workspace(root: &str) -> Workspace {
        Workspace::from_metadata(metadata(root).as_bytes(), |_| PathBuf::new()).unwrap()
    }

    fn message(
        root: &str,
        manifest: &str,
        code: Option<&str>,
        level: &str,
        spans: &serde_json::Value,
    ) -> String {
        serde_json::json!({
            "reason": "compiler-message",
            "package_id": format!("path+file://{root}/crates/cli#ops@0.65.0"),
            "manifest_path": format!("{root}/{manifest}"),
            "target": {"name": "ops", "kind": ["bin"]},
            "message": {
                "message": "this could be rewritten\nas a `match`",
                "level": level,
                "code": code.map(|c| serde_json::json!({"code": c})),
                "spans": spans,
            }
        })
        .to_string()
    }

    fn span(file: &str, line: u64, col: u64) -> serde_json::Value {
        serde_json::json!([
            {"file_name": "ignored.rs", "line_start": 1, "column_start": 1, "is_primary": false},
            {"file_name": file, "line_start": line, "column_start": col, "is_primary": true}
        ])
    }

    fn stream(root: &str) -> Vec<String> {
        let lint = Some("clippy::single_match_else");
        vec![
            message(
                root,
                "crates/cli/Cargo.toml",
                lint,
                "warning",
                &span("crates/cli/src/main.rs", 10, 5),
            ),
            // Same diagnostic from the unit-test build of the same target.
            message(
                root,
                "crates/cli/Cargo.toml",
                lint,
                "warning",
                &span("crates/cli/src/main.rs", 10, 5),
            ),
            // Same line, different column: a distinct finding.
            message(
                root,
                "crates/cli/Cargo.toml",
                lint,
                "warning",
                &span("crates/cli/src/main.rs", 10, 9),
            ),
            // Spanless: attributed to the crate manifest.
            message(
                root,
                "crates/cli/Cargo.toml",
                Some("clippy::cargo_common_metadata"),
                "warning",
                &serde_json::json!([]),
            ),
            message(
                root,
                "Cargo.toml",
                Some("clippy::cargo_common_metadata"),
                "warning",
                &serde_json::json!([]),
            ),
            // Out of tree: registry source, OUT_DIR, member outside checkout.
            message(
                root,
                "crates/cli/Cargo.toml",
                lint,
                "warning",
                &span("/home/u/.cargo/registry/src/x/lib.rs", 1, 1),
            ),
            message(
                root,
                "crates/cli/Cargo.toml",
                lint,
                "warning",
                &span(&format!("{root}/target/debug/build/x/out/gen.rs"), 3, 1),
            ),
            message(
                root,
                "crates/cli/Cargo.toml",
                lint,
                "warning",
                &span("../outside/src/lib.rs", 3, 1),
            ),
            message(
                "/elsewhere",
                "shared/Cargo.toml",
                lint,
                "warning",
                &span("src/lib.rs", 3, 1),
            ),
            // Plain rustc warning, counted once across two builds.
            message(
                root,
                "crates/cli/Cargo.toml",
                Some("dead_code"),
                "warning",
                &span("crates/cli/src/main.rs", 2, 1),
            ),
            message(
                root,
                "crates/cli/Cargo.toml",
                Some("dead_code"),
                "warning",
                &span("crates/cli/src/main.rs", 2, 1),
            ),
            // Codeless summary and a non-compiler record are ignored.
            message(
                root,
                "crates/cli/Cargo.toml",
                None,
                "warning",
                &serde_json::json!([]),
            ),
            r#"{"reason":"build-finished","success":true}"#.to_string(),
            String::new(),
        ]
    }

    fn report_json(root: &str) -> String {
        let ws = workspace(root);
        let mut collector = Collector::default();
        for line in stream(root) {
            collector.push_line(&ws, &line).unwrap();
        }
        serde_json::to_string_pretty(&collector.into_report()).unwrap()
    }

    fn report(root: &str) -> serde_json::Value {
        serde_json::from_str(&report_json(root)).unwrap()
    }

    #[test]
    fn rows_carry_the_normalized_identity() {
        let report = report("/home/a/ops");
        assert_eq!(report["schema_version"], 1);
        assert_eq!(report["kind"], "clippy-findings");
        let first = report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["column"] == 5)
            .unwrap();
        assert_eq!(first["lint"], "single_match_else");
        assert_eq!(first["package"], "ops@0.65.0");
        assert_eq!(first["manifest_dir"], "crates/cli");
        assert_eq!(first["target"], "ops");
        assert_eq!(first["target_kind"], "bin");
        assert_eq!(first["file"], "crates/cli/src/main.rs");
        assert_eq!(first["line"], 10);
        assert_eq!(first["column"], 5);
        assert_eq!(first["message"], "this could be rewritten\nas a `match`");
    }

    #[test]
    fn duplicates_collapse_and_distinct_columns_stay_distinct() {
        let report = report("/home/a/ops");
        let columns: Vec<_> = report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["file"] == "crates/cli/src/main.rs")
            .map(|f| f["column"].as_u64().unwrap())
            .collect();
        assert_eq!(columns, vec![5, 9]);
    }

    #[test]
    fn spanless_diagnostics_attribute_to_the_manifest() {
        let report = report("/home/a/ops");
        let spanless: Vec<_> = report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["line"] == 0)
            .map(|f| {
                (
                    f["file"].as_str().unwrap(),
                    f["manifest_dir"].as_str().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            spanless,
            vec![("Cargo.toml", "."), ("crates/cli/Cargo.toml", "crates/cli")]
        );
    }

    #[test]
    fn out_of_tree_and_rustc_warnings_are_counted_not_reported() {
        let report = report("/home/a/ops");
        assert_eq!(report["findings"].as_array().unwrap().len(), 4);
        assert_eq!(report["dropped_out_of_tree"], 4);
        assert_eq!(report["rustc_warnings"], 1);
    }

    #[test]
    fn output_is_identical_across_checkout_paths() {
        assert_eq!(
            report_json("/home/a/ops"),
            report_json("/tmp/wt/other-name")
        );
    }

    #[test]
    fn repo_prefix_is_prepended_to_every_path() {
        let root = "/repo/rust";
        let ws = Workspace::from_metadata(metadata(root).as_bytes(), |r| {
            assert_eq!(r, Path::new(root));
            PathBuf::from("rust")
        })
        .unwrap();
        let mut collector = Collector::default();
        for line in stream(root) {
            collector.push_line(&ws, &line).unwrap();
        }
        let report = collector.into_report();
        assert!(report.findings.iter().all(|f| f.file.starts_with("rust/")));
        assert!(report.findings.iter().any(|f| f.manifest_dir == "rust"));
    }

    #[test]
    fn non_json_line_is_an_error() {
        let ws = workspace("/home/a/ops");
        assert!(Collector::default()
            .push_line(&ws, "Compiling ops")
            .is_err());
    }

    #[test]
    fn args_never_deny_warnings_and_append_lint_flags() {
        let opts = SurveyOptions::default();
        let args = clippy_args(&opts, &[]);
        assert!(!args.iter().any(|a| a == "-D" || a == "--"));
        assert!(args.contains(&"--message-format=json".to_string()));
        let args = clippy_args(&opts, &["-W".into(), "clippy::pedantic".into()]);
        assert_eq!(&args[args.len() - 3..], ["--", "-W", "clippy::pedantic"]);
    }

    #[test]
    fn default_args_are_the_gate_build_under_locked() {
        assert_eq!(
            clippy_args(&SurveyOptions::default(), &[]),
            [
                "clippy",
                "--workspace",
                "--locked",
                "--all-features",
                "--all-targets",
                "--message-format=json",
            ]
        );
    }

    #[test]
    fn explicit_feature_selection_drops_all_features() {
        assert!(SurveyOptions::from_flags(false, false, false, vec![]).all_features);
        assert!(!SurveyOptions::from_flags(false, true, false, vec![]).all_features);
        assert!(!SurveyOptions::from_flags(false, false, true, vec![]).all_features);
        assert!(!SurveyOptions::from_flags(false, false, false, vec!["a".into()]).all_features);
        assert!(!SurveyOptions::from_flags(true, false, false, vec![]).locked);
        assert_eq!(
            SurveyOptions::from_flags(false, false, false, vec![]),
            SurveyOptions::default()
        );
    }

    #[test]
    fn unlocked_default_features_survey() {
        let opts = SurveyOptions {
            locked: false,
            all_features: false,
            ..SurveyOptions::default()
        };
        assert_eq!(
            clippy_args(&opts, &[]),
            [
                "clippy",
                "--workspace",
                "--all-targets",
                "--message-format=json",
            ]
        );
    }

    #[test]
    fn explicit_feature_list_without_defaults() {
        let opts = SurveyOptions {
            all_features: false,
            no_default_features: true,
            features: vec!["a".into(), "b/c".into()],
            ..SurveyOptions::default()
        };
        assert_eq!(
            clippy_args(&opts, &["-W".into(), "clippy::pedantic".into()]),
            [
                "clippy",
                "--workspace",
                "--locked",
                "--no-default-features",
                "--features",
                "a,b/c",
                "--all-targets",
                "--message-format=json",
                "--",
                "-W",
                "clippy::pedantic",
            ]
        );
    }

    #[test]
    fn normalize_rejects_escapes() {
        assert_eq!(normalize(Path::new("./a/b")), Some(PathBuf::from("a/b")));
        assert_eq!(normalize(Path::new("a/../b")), None);
        assert_eq!(normalize(Path::new("/abs")), None);
    }
}
