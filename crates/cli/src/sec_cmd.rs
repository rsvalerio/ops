//! `ops sec` — run [Trivy](https://trivy.dev) security scans, auto-selected by
//! the file types detected in the workspace.
//!
//! The command is a thin wrapper: it never bundles or reimplements a scanner.
//! It decides *which* scans make sense for the project and shells out to the
//! `trivy` CLI for each. Output is quiet by design — one `scanning <scan> ✓`
//! line per scan — and Trivy's full report is shown only when a scan actually
//! finds something (or errors).
//!
//! Scan selection:
//! - **Secret** — always run. Every project can leak credentials.
//! - **Vulnerability** — run when a dependency manifest/lockfile is present
//!   (`Cargo.lock`, `package-lock.json`, `go.sum`, `requirements.txt`, …).
//! - **Misconfiguration / IaC** — run when a Dockerfile or other
//!   infrastructure-as-code marker is present (`*.tf`, `compose.yaml`,
//!   `Chart.yaml`, `kustomization.yaml`, …).
//!
//! `--dry-run` (the global flag) prints the resolved plan — both the scans that
//! *will* run and the ones that *won't*, each with the reason — without
//! requiring `trivy` to be installed. That is the "check what to run" preview.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::Context as _;

/// Directories never worth walking for detection. Mirrors the text-fixer
/// discovery deny-list so detection stays fast on large trees and does not
/// trip over vendored dependencies or build output.
const SKIP_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    ".venv",
    "venv",
    "__pycache__",
    ".terraform",
];

/// One Trivy scan `ops sec` can run. Declaration order is the order scans run
/// in: the cheap, universally-relevant secret scan first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scan {
    /// Secret scan — always enabled.
    Secret,
    /// Dependency vulnerability scan.
    Vuln,
    /// Misconfiguration / infrastructure-as-code scan.
    Misconfig,
}

/// CLI-facing scan selector for `--skip` / `--force`. A separate `ValueEnum`
/// keeps the clap surface (stable flag spellings + aliases) decoupled from the
/// internal [`Scan`] variants.
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScanArg {
    #[value(name = "secrets", alias = "secret")]
    Secret,
    #[value(name = "vuln", alias = "vulnerabilities", alias = "vulns")]
    Vuln,
    #[value(name = "misconfig", alias = "config", alias = "iac")]
    Misconfig,
}

impl ScanArg {
    fn to_scan(self) -> Scan {
        match self {
            ScanArg::Secret => Scan::Secret,
            ScanArg::Vuln => Scan::Vuln,
            ScanArg::Misconfig => Scan::Misconfig,
        }
    }
}

impl Scan {
    /// All scans in run order. Selection filters this down per project.
    const ALL: &'static [Scan] = &[Scan::Secret, Scan::Vuln, Scan::Misconfig];

    /// Short human label used in the plan and section headers.
    fn label(self) -> &'static str {
        match self {
            Scan::Secret => "secrets",
            Scan::Vuln => "vulnerabilities",
            Scan::Misconfig => "misconfiguration",
        }
    }

    /// Trivy argv preceding the target path. `--exit-code 1` makes Trivy report
    /// a non-zero status when it *finds* something (its default is to exit 0
    /// even on findings), so the aggregated `ops sec` exit code reflects
    /// findings as the user expects in CI. `--quiet` drops Trivy's progress/INFO
    /// logs so the only noise on a findings result is the report itself.
    fn trivy_args(self) -> &'static [&'static str] {
        match self {
            Scan::Secret => &["fs", "--quiet", "--scanners", "secret", "--exit-code", "1"],
            Scan::Vuln => &["fs", "--quiet", "--scanners", "vuln", "--exit-code", "1"],
            Scan::Misconfig => &["config", "--quiet", "--exit-code", "1"],
        }
    }
}

/// Whether a scan is selected for this project, and why. The `reason` is shown
/// for both selected and skipped scans so `--dry-run` documents the full
/// decision, not just the affirmative half.
#[derive(Debug, Clone)]
pub(crate) struct PlanEntry {
    pub scan: Scan,
    pub selected: bool,
    pub reason: &'static str,
}

/// Markers that justify a vulnerability scan — dependency manifests and
/// lockfiles Trivy knows how to read.
fn is_vuln_marker(name: &str) -> bool {
    const EXACT: &[&str] = &[
        "Cargo.lock",
        "package-lock.json",
        "yarn.lock",
        "pnpm-lock.yaml",
        "go.mod",
        "go.sum",
        "requirements.txt",
        "Pipfile.lock",
        "poetry.lock",
        "Gemfile.lock",
        "pom.xml",
        "composer.lock",
        "gradle.lockfile",
    ];
    EXACT.iter().any(|m| m.eq_ignore_ascii_case(name))
}

/// Markers that justify a misconfiguration / IaC scan — Dockerfiles and common
/// infrastructure-as-code files. Trivy's `config` scan then walks the tree for
/// every IaC file it understands; we only need one marker to switch it on.
fn is_misconfig_marker(name: &str) -> bool {
    const EXACT: &[&str] = &[
        "Dockerfile",
        "Containerfile",
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
        "Chart.yaml",
        "kustomization.yml",
        "kustomization.yaml",
    ];
    if EXACT.iter().any(|m| m.eq_ignore_ascii_case(name)) {
        return true;
    }
    // `Dockerfile.dev`, `prod.Dockerfile`, `app.dockerfile`, `*.tf`, `*.tofu`.
    let lower = name.to_ascii_lowercase();
    lower.starts_with("dockerfile.")
        || lower.ends_with(".dockerfile")
        || lower.ends_with(".tf")
        || lower.ends_with(".tofu")
}

/// Whether `name` is a YAML file by extension. Used to decide which files are
/// worth sniffing for a Kubernetes manifest signature.
fn is_yaml_file(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".yaml") || lower.ends_with(".yml")
}

/// Heuristically decide whether `path` is a Kubernetes manifest. Generic k8s
/// YAML is indistinguishable from any other YAML by filename, so we sniff a
/// bounded prefix for the two top-level keys every manifest declares —
/// `apiVersion:` and `kind:`. Reading is capped so a giant YAML data file can't
/// turn detection into a full-file read.
fn is_k8s_manifest(path: &Path) -> bool {
    use std::io::Read as _;
    const SNIFF_BYTES: u64 = 4096;
    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };
    let mut buf = String::new();
    if file.take(SNIFF_BYTES).read_to_string(&mut buf).is_err() {
        // Non-UTF-8 (binary) content is not a manifest.
        return false;
    }
    let mut has_api_version = false;
    let mut has_kind = false;
    for line in buf.lines() {
        // Top-level mapping keys are unindented; `trim_start` also tolerates
        // a leading document marker without matching list items.
        let trimmed = line.trim_start();
        if trimmed.starts_with("apiVersion:") {
            has_api_version = true;
        } else if trimmed.starts_with("kind:") {
            has_kind = true;
        }
        if has_api_version && has_kind {
            return true;
        }
    }
    false
}

/// What a detection walk found. Tracked as two bools so the walk can stop
/// early once both categories are present.
#[derive(Debug, Default, Clone, Copy)]
struct Detected {
    vuln: bool,
    misconfig: bool,
}

impl Detected {
    fn complete(self) -> bool {
        self.vuln && self.misconfig
    }
}

/// Walk `root` (skipping VCS/build/vendor directories) and report which scan
/// categories have a marker file. Bounded by `complete()` early-exit so a large
/// monorepo stops scanning as soon as both categories are confirmed.
fn detect(root: &Path) -> Detected {
    let mut found = Detected::default();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if found.complete() {
            break;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let file_type = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if file_type.is_symlink() {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if file_type.is_dir() {
                if !SKIP_DIRS.iter().any(|d| *d == name) {
                    stack.push(entry.path());
                }
            } else if file_type.is_file() {
                if !found.vuln && is_vuln_marker(&name) {
                    found.vuln = true;
                }
                if !found.misconfig
                    && (is_misconfig_marker(&name)
                        || (is_yaml_file(&name) && is_k8s_manifest(&entry.path())))
                {
                    found.misconfig = true;
                }
            }
        }
    }
    found
}

/// Resolve the full scan plan for `root`: every known scan with its selected
/// flag and the reason behind it. `--force`/`--skip` overrides win over the
/// auto-detection default; conflicting overrides are rejected upstream in
/// [`run_sec_to`], so a scan never appears in both lists here.
fn build_plan(root: &Path, skip: &[Scan], force: &[Scan]) -> Vec<PlanEntry> {
    let found = detect(root);
    Scan::ALL
        .iter()
        .map(|&scan| {
            let (selected, reason) = if force.contains(&scan) {
                (true, "forced on (--force)")
            } else if skip.contains(&scan) {
                (false, "skipped (--skip)")
            } else {
                match scan {
                    Scan::Secret => (true, "always enabled"),
                    Scan::Vuln => {
                        if found.vuln {
                            (true, "dependency manifest or lockfile detected")
                        } else {
                            (false, "no dependency manifest or lockfile found")
                        }
                    }
                    Scan::Misconfig => {
                        if found.misconfig {
                            (true, "Dockerfile, Kubernetes, or IaC files detected")
                        } else {
                            (false, "no Dockerfile, Kubernetes, or IaC files found")
                        }
                    }
                }
            };
            PlanEntry {
                scan,
                selected,
                reason,
            }
        })
        .collect()
}

/// Render the resolved plan, marking each scan run/skip with its reason.
fn write_plan(w: &mut dyn std::io::Write, plan: &[PlanEntry]) -> std::io::Result<()> {
    writeln!(w, "ops sec — scan plan:")?;
    for entry in plan {
        let mark = if entry.selected { "run " } else { "skip" };
        writeln!(
            w,
            "  [{mark}] {:<16} ({})",
            entry.scan.label(),
            entry.reason
        )?;
    }
    Ok(())
}

/// Best-effort check that `trivy` is resolvable on `PATH`. Used to fail with a
/// clear, actionable message *before* any scan starts, rather than letting the
/// first spawn surface a bare `No such file or directory`.
fn trivy_on_path() -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&path) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        if dir.join("trivy").is_file() {
            return true;
        }
        #[cfg(windows)]
        if dir.join("trivy.exe").is_file() {
            return true;
        }
    }
    false
}

const TRIVY_MISSING_HELP: &str = "trivy not found on PATH. `ops sec` requires Trivy.\n\
     Install it: https://trivy.dev/latest/getting-started/installation/ \
     (e.g. `brew install trivy`).";

/// Spawn `trivy <args> <root>`, capturing its output rather than inheriting
/// stdio. Capturing lets the caller stay silent on a clean scan and only print
/// Trivy's report when something is actually found.
fn run_trivy(root: &Path, scan: Scan) -> anyhow::Result<std::process::Output> {
    Command::new("trivy")
        .args(scan.trivy_args())
        .arg(root)
        .output()
        .with_context(|| format!("failed to run `trivy` for the {} scan", scan.label()))
}

/// Run one scan and report it on a single quiet line: `scanning <scan> ✓` when
/// clean. On a non-zero result (findings or a Trivy error) the line ends with
/// `✗` and Trivy's captured report is printed beneath it — stdout (the report)
/// to `w`, stderr (logs/errors) to the process stderr. Returns whether the scan
/// was clean.
fn run_scan(root: &Path, scan: Scan, w: &mut dyn std::io::Write) -> anyhow::Result<bool> {
    use std::io::Write as _;
    write!(w, "scanning {} ", scan.label())?;
    w.flush()?;
    let output = run_trivy(root, scan)?;
    if output.status.success() {
        writeln!(w, "✓")?;
        return Ok(true);
    }
    writeln!(w, "✗")?;
    w.write_all(&output.stdout)?;
    w.flush()?;
    let mut stderr = std::io::stderr().lock();
    let _ = stderr.write_all(&output.stderr);
    Ok(false)
}

/// Entry point: build the plan, preview-or-run it, and return an aggregated
/// exit code. `skip`/`force` come straight from the `--skip`/`--force` CLI
/// flags. Splitting the testable core into [`run_sec_to`] keeps the plan
/// output assertable without spawning Trivy.
pub(crate) fn run_sec(
    root: &Path,
    dry_run: bool,
    skip: &[ScanArg],
    force: &[ScanArg],
) -> anyhow::Result<ExitCode> {
    let skip: Vec<Scan> = skip.iter().map(|s| s.to_scan()).collect();
    let force: Vec<Scan> = force.iter().map(|s| s.to_scan()).collect();
    run_sec_to(root, dry_run, &skip, &force, &mut std::io::stdout())
}

fn run_sec_to(
    root: &Path,
    dry_run: bool,
    skip: &[Scan],
    force: &[Scan],
    w: &mut dyn std::io::Write,
) -> anyhow::Result<ExitCode> {
    // A scan named in both lists is contradictory intent — reject it loudly
    // rather than silently letting one side win.
    if let Some(conflict) = skip.iter().find(|s| force.contains(s)) {
        anyhow::bail!(
            "scan `{}` was passed to both --skip and --force",
            conflict.label()
        );
    }

    let plan = build_plan(root, skip, force);
    let selected: Vec<Scan> = plan.iter().filter(|e| e.selected).map(|e| e.scan).collect();

    if dry_run {
        // Preview only — print the full plan (run + skip, with reasons) and
        // never execute Trivy, so do not require it installed. A heads-up keeps
        // the preview honest when it would have failed live.
        write_plan(w, &plan).context("failed to write scan plan")?;
        if !trivy_on_path() {
            ops_core::ui::warn(TRIVY_MISSING_HELP);
        }
        return Ok(ExitCode::SUCCESS);
    }

    if !trivy_on_path() {
        anyhow::bail!(TRIVY_MISSING_HELP);
    }

    // Stay quiet by default: one `scanning <scan> ✓` line per scan. Run every
    // applicable scan even after one reports findings, then aggregate: success
    // only when every scan was clean.
    let mut all_ok = true;
    for scan in selected {
        if !run_scan(root, scan, w)? {
            all_ok = false;
        }
    }

    Ok(if all_ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(dir: &Path, name: &str) {
        std::fs::write(dir.join(name), b"x").expect("write marker");
    }

    /// Selected scans for `root` under the given overrides, in run order.
    fn selected_scans(root: &Path, skip: &[Scan], force: &[Scan]) -> Vec<Scan> {
        build_plan(root, skip, force)
            .into_iter()
            .filter(|e| e.selected)
            .map(|e| e.scan)
            .collect()
    }

    #[test]
    fn secret_scan_always_selected() {
        let dir = tempfile::tempdir().unwrap();
        let plan = build_plan(dir.path(), &[], &[]);
        let secret = plan.iter().find(|e| e.scan == Scan::Secret).unwrap();
        assert!(secret.selected, "secret scan must always be selected");
    }

    #[test]
    fn empty_project_runs_only_secrets() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(selected_scans(dir.path(), &[], &[]), vec![Scan::Secret]);
    }

    #[test]
    fn cargo_lock_enables_vuln_scan() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "Cargo.lock");
        let plan = build_plan(dir.path(), &[], &[]);
        let vuln = plan.iter().find(|e| e.scan == Scan::Vuln).unwrap();
        assert!(vuln.selected, "Cargo.lock must enable the vuln scan");
    }

    #[test]
    fn dockerfile_enables_misconfig_scan() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "Dockerfile");
        let plan = build_plan(dir.path(), &[], &[]);
        let misconfig = plan.iter().find(|e| e.scan == Scan::Misconfig).unwrap();
        assert!(misconfig.selected, "Dockerfile must enable misconfig scan");
    }

    #[test]
    fn terraform_file_enables_misconfig_scan() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "main.tf");
        let plan = build_plan(dir.path(), &[], &[]);
        let misconfig = plan.iter().find(|e| e.scan == Scan::Misconfig).unwrap();
        assert!(misconfig.selected, "*.tf must enable misconfig scan");
    }

    #[test]
    fn kubernetes_yaml_enables_misconfig_scan() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("deployment.yaml"),
            "apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: web\n",
        )
        .unwrap();
        let plan = build_plan(dir.path(), &[], &[]);
        let misconfig = plan.iter().find(|e| e.scan == Scan::Misconfig).unwrap();
        assert!(
            misconfig.selected,
            "a k8s manifest YAML must enable the misconfig scan"
        );
    }

    #[test]
    fn plain_yaml_does_not_enable_misconfig_scan() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.yaml"),
            "name: my-app\nversion: 1\nsettings:\n  debug: true\n",
        )
        .unwrap();
        let plan = build_plan(dir.path(), &[], &[]);
        let misconfig = plan.iter().find(|e| e.scan == Scan::Misconfig).unwrap();
        assert!(
            !misconfig.selected,
            "non-manifest YAML must not enable the misconfig scan"
        );
    }

    #[test]
    fn skip_removes_an_auto_selected_scan() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "Cargo.lock");
        // Vuln would auto-select; --skip vuln must drop it. Secret stays.
        let selected = selected_scans(dir.path(), &[Scan::Vuln], &[]);
        assert_eq!(selected, vec![Scan::Secret]);
    }

    #[test]
    fn skip_can_disable_the_always_on_secret_scan() {
        let dir = tempfile::tempdir().unwrap();
        let selected = selected_scans(dir.path(), &[Scan::Secret], &[]);
        assert!(
            !selected.contains(&Scan::Secret),
            "--skip secrets must override the always-on default"
        );
    }

    #[test]
    fn force_enables_a_scan_detection_would_skip() {
        let dir = tempfile::tempdir().unwrap();
        // No IaC markers, so misconfig would auto-skip; --force misconfig runs it.
        let selected = selected_scans(dir.path(), &[], &[Scan::Misconfig]);
        assert!(selected.contains(&Scan::Misconfig));
    }

    #[test]
    fn conflicting_skip_and_force_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let mut buf: Vec<u8> = Vec::new();
        let err = run_sec_to(dir.path(), true, &[Scan::Vuln], &[Scan::Vuln], &mut buf)
            .expect_err("a scan in both --skip and --force must error");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("--skip") && msg.contains("--force"),
            "error must name both flags, got: {msg}"
        );
    }

    #[test]
    fn named_dockerfile_variants_detected() {
        assert!(is_misconfig_marker("Dockerfile"));
        assert!(is_misconfig_marker("Dockerfile.dev"));
        assert!(is_misconfig_marker("prod.Dockerfile"));
        assert!(is_misconfig_marker("app.dockerfile"));
        assert!(is_misconfig_marker("compose.yaml"));
        assert!(is_misconfig_marker("variables.tofu"));
        assert!(!is_misconfig_marker("README.md"));
        assert!(!is_misconfig_marker("dockerfile-notes.txt"));
    }

    #[test]
    fn markers_in_nested_dirs_detected() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("services/api")).unwrap();
        touch(&dir.path().join("services/api"), "Dockerfile");
        let plan = build_plan(dir.path(), &[], &[]);
        let misconfig = plan.iter().find(|e| e.scan == Scan::Misconfig).unwrap();
        assert!(misconfig.selected, "nested Dockerfile must be detected");
    }

    #[test]
    fn markers_inside_skip_dirs_ignored() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("node_modules/pkg")).unwrap();
        touch(&dir.path().join("node_modules/pkg"), "Dockerfile");
        let plan = build_plan(dir.path(), &[], &[]);
        let misconfig = plan.iter().find(|e| e.scan == Scan::Misconfig).unwrap();
        assert!(
            !misconfig.selected,
            "markers under node_modules must not trigger a scan"
        );
    }

    #[test]
    fn dry_run_writes_plan_and_does_not_require_trivy() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "Cargo.lock");
        let mut buf: Vec<u8> = Vec::new();
        let code =
            run_sec_to(dir.path(), true, &[], &[], &mut buf).expect("dry-run must not error");
        // ExitCode is opaque; compare Debug form against SUCCESS.
        assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("scan plan"));
        assert!(out.contains("secrets"));
        assert!(out.contains("[run ]"), "selected scans marked run: {out}");
        assert!(out.contains("[skip]"), "skipped scans marked skip: {out}");
    }

    #[test]
    fn plan_lists_every_scan() {
        let dir = tempfile::tempdir().unwrap();
        let plan = build_plan(dir.path(), &[], &[]);
        assert_eq!(plan.len(), Scan::ALL.len());
    }
}
