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
//! - **Misconfiguration / `IaC`** — run when a Dockerfile or other
//!   infrastructure-as-code marker is present (`*.tf`, `compose.yaml`,
//!   `Chart.yaml`, `kustomization.yaml`, …).
//!
//! `--dry-run` (the global flag) prints the resolved plan — both the scans that
//! *will* run and the ones that *won't*, each with the reason — without
//! requiring `trivy` to be installed. That is the "check what to run" preview.
//!
//! Every scan skips each stack's default build/dependency directories at any
//! depth plus `.git` (see [`shared_skip_dirs`], TASK-2264): build output is
//! generated artefact, not source, and Trivy aborts a scan when a concurrent
//! build deletes a file mid-walk. Generic names — `build`, `dist` — are
//! plausible checked-in source paths too, so they are skipped only where a
//! stack manifest beside them says they are generated output (see
//! [`WalkOutcome::generated`]). `--no-default-skips` opts out.
//!
//! # Exit code
//!
//! `ops sec` is the terminal step of `ops qa`, so its exit code is what a CI
//! gate or pre-push hook keys off. It **fails closed**:
//!
//! - non-zero when any scan reports findings, errors, or times out;
//! - non-zero when *zero* scans ran (SEC-31 / TASK-1754). An all-skipped run
//!   used to print nothing and exit 0, which is indistinguishable — to every
//!   automated consumer — from "all scans ran and found nothing". A silently
//!   inert security gate reporting healthy is the fail-open shape, so the
//!   empty selection is reported explicitly and treated as a failure.
//!   `--dry-run` remains the way to preview a plan without running anything.
//!
//! Each `trivy` invocation is bounded by [`DEFAULT_SCAN_TIMEOUT`], overridable
//! via the [`SCAN_TIMEOUT_ENV`] environment variable.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output};
use std::time::Duration;

use anyhow::Context as _;
use ops_core::subprocess::{run_with_timeout, RunError};

/// Directories never worth walking, for detection *and* for Trivy — the
/// *unambiguous* half of the skip policy.
///
/// TASK-2264: the single shared list is `ops_core::stack::scan_skip_dirs` —
/// every stack's declared build/dependency directories plus `.git`, minus
/// the generic names — so the detection walk and the Trivy invocations
/// cannot drift apart. Build output is generated artefact, not source:
/// walking it is slow (~16s on a Rust project, mostly `target/`), scans
/// generated files rather than the code the user wrote, and races the
/// builds producing it (Trivy aborts when a file vanishes mid-walk).
///
/// The generic names (`build`, `dist`) are *not* in this list: they are
/// plausible checked-in source paths (`services/build`), so the walk skips
/// them only where [`ops_core::stack::is_generated_build_dir`] says the
/// directory beside a stack manifest is generated output, and Trivy receives
/// those as discovered per-path `--skip-dirs` entries rather than a blanket
/// `**/build` (TASK-2271).
///
/// `--no-default-skips` (see [`run_sec_to`]) removes the list from the
/// Trivy invocations only; detection keeps skipping, because a marker file
/// inside build output (a `Cargo.lock` under `target/package/`) says nothing
/// about the project's own dependency manifests.
fn shared_skip_dirs() -> Vec<&'static str> {
    ops_core::stack::scan_skip_dirs()
}

/// One Trivy scan `ops sec` can run. Declaration order is the order scans run
/// in: the cheap, universally-relevant secret scan first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scan {
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
pub enum ScanArg {
    #[value(name = "secrets", alias = "secret")]
    Secret,
    #[value(name = "vuln", alias = "vulnerabilities", alias = "vulns")]
    Vuln,
    #[value(name = "misconfig", alias = "config", alias = "iac")]
    Misconfig,
}

impl ScanArg {
    const fn to_scan(self) -> Scan {
        match self {
            Self::Secret => Scan::Secret,
            Self::Vuln => Scan::Vuln,
            Self::Misconfig => Scan::Misconfig,
        }
    }
}

impl Scan {
    /// All scans in run order. Selection filters this down per project.
    const ALL: &'static [Self] = &[Self::Secret, Self::Vuln, Self::Misconfig];

    /// Short human label used in the plan and section headers.
    const fn label(self) -> &'static str {
        match self {
            Self::Secret => "secrets",
            Self::Vuln => "vulnerabilities",
            Self::Misconfig => "misconfiguration",
        }
    }

    /// Trivy argv preceding the target path. `--exit-code 1` makes Trivy report
    /// a non-zero status when it *finds* something (its default is to exit 0
    /// even on findings), so the aggregated `ops sec` exit code reflects
    /// findings as the user expects in CI. `--quiet` drops Trivy's progress/INFO
    /// logs so the only noise on a findings result is the report itself.
    const fn trivy_args(self) -> &'static [&'static str] {
        match self {
            Self::Secret => &["fs", "--quiet", "--scanners", "secret", "--exit-code", "1"],
            Self::Vuln => &["fs", "--quiet", "--scanners", "vuln", "--exit-code", "1"],
            Self::Misconfig => &["config", "--quiet", "--exit-code", "1"],
        }
    }
}

/// Whether a scan is selected for this project, and why. The `reason` is shown
/// for both selected and skipped scans so `--dry-run` documents the full
/// decision, not just the affirmative half.
#[derive(Debug, Clone)]
pub struct PlanEntry {
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

/// Markers that justify a misconfiguration / `IaC` scan — Dockerfiles and common
/// infrastructure-as-code files. Trivy's `config` scan then walks the tree for
/// every `IaC` file it understands; we only need one marker to switch it on.
// The comparisons below run against `name.to_ascii_lowercase()`, so the
// case-sensitivity the lint warns about is already handled.
#[allow(clippy::case_sensitive_file_extension_comparisons)]
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
// As in `is_misconfig_marker`: `name` is lowercased before comparison.
#[allow(clippy::case_sensitive_file_extension_comparisons)]
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

/// What a detection walk found. Tracked as two bools so file inspection can
/// stop once both categories are present.
#[derive(Debug, Default, Clone, Copy)]
struct Detected {
    vuln: bool,
    misconfig: bool,
}

/// A detection walk's full result: the scan categories with a marker file,
/// plus every *generated* build directory discovered on the way (paths
/// relative to `root`, e.g. `build` or `app/dist`).
///
/// The inventory is what Trivy skips for the generic names: a blanket
/// `**/build` would also exclude checked-in source directories
/// (`services/build`), so those names reach Trivy only as the exact
/// discovered paths (TASK-2271).
#[derive(Debug)]
struct WalkOutcome {
    found: Detected,
    generated: Vec<PathBuf>,
}

/// Walk `root` (skipping VCS/build/vendor directories via
/// [`shared_skip_dirs`]; generic names via
/// [`ops_core::stack::is_generated_build_dir`]) and report which scan
/// categories have a marker file.
///
/// The walk no longer early-exits once both categories are found: it must also
/// produce the complete generated-directory inventory for Trivy's
/// `--skip-dirs`, and a partial inventory would let Trivy walk — and race
/// the builds deleting files in — the very directories the skip list exists
/// to avoid. Only the *file* work stays bounded: marker checks are gated on
/// the category still being unfound, so the (content-reading) Kubernetes
/// manifest probe stops as early as it ever did; directory enumeration is
/// getdents-cheap next to the Trivy scans this walk feeds.
fn detect(root: &Path) -> WalkOutcome {
    let skip = shared_skip_dirs();
    let generic = ops_core::stack::generic_build_dirs();
    let mut found = Detected::default();
    let mut generated: Vec<PathBuf> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if file_type.is_dir() {
                let skip_dir = if skip.iter().any(|d| *d == name) {
                    true
                } else if generic.iter().any(|d| *d == name)
                    && ops_core::stack::is_generated_build_dir(&dir, &name)
                {
                    // Record the path relative to the scan root: that is
                    // the exact `--skip-dirs` entry Trivy gets.
                    if let Ok(rel) = entry.path().strip_prefix(root) {
                        generated.push(rel.to_path_buf());
                    }
                    true
                } else {
                    false
                };
                if !skip_dir {
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
    WalkOutcome { found, generated }
}

/// Resolve the full scan plan for `root`: every known scan with its selected
/// flag and the reason behind it. `--force`/`--skip` overrides win over the
/// auto-detection default; conflicting overrides are rejected upstream in
/// [`run_sec_to`], so a scan never appears in both lists here.
/// Test-facing convenience: walk + plan in one call. Production paths go
/// through [`build_plan_from`] on a walk whose generated-dir inventory they
/// also reuse for the Trivy invocations.
#[cfg(test)]
fn build_plan(root: &Path, skip: &[Scan], force: &[Scan]) -> Vec<PlanEntry> {
    build_plan_from(detect(root).found, skip, force)
}

/// The plan computation proper, over an already-run detection walk. Split
/// from the `build_plan` wrapper so [`run_sec_to`] can reuse one walk's outcome —
/// including its generated-directory inventory — for both the plan and the
/// Trivy invocations instead of walking twice.
fn build_plan_from(found: Detected, skip: &[Scan], force: &[Scan]) -> Vec<PlanEntry> {
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

/// Wall-clock budget for a single `trivy` invocation.
///
/// ASYNC-6 / SEC-33 (TASK-1748): `Command::output()` waits forever and buffers
/// the whole report in memory. `ops sec` is the terminal step of `ops qa`, so
/// an unbounded wait here is a hung CI job or a hung pre-push hook with no
/// diagnostic beyond a half-written `scanning vulnerabilities ` line.
///
/// Ten minutes is deliberately generous: a cold `trivy fs --scanners vuln`
/// downloads and unpacks the vulnerability DB from a remote registry, which
/// legitimately takes minutes on a slow link, while a warm scan is seconds.
/// The budget exists to bound a *hang* — an unreachable registry, a
/// captive-portal proxy, a stalled TLS handshake — not to police a slow but
/// progressing scan. Operators on a slower link raise it via
/// [`SCAN_TIMEOUT_ENV`].
const DEFAULT_SCAN_TIMEOUT: Duration = Duration::from_secs(600);

/// Environment variable overriding [`DEFAULT_SCAN_TIMEOUT`], in whole seconds.
/// A missing, unparsable, or zero value falls back to the default with a
/// warning rather than silently arming a zero-second deadline.
const SCAN_TIMEOUT_ENV: &str = "OPS_SEC_TIMEOUT_SECS";

/// Resolve the per-scan timeout from [`SCAN_TIMEOUT_ENV`], falling back to
/// [`DEFAULT_SCAN_TIMEOUT`].
fn scan_timeout() -> Duration {
    let Some(raw) = std::env::var_os(SCAN_TIMEOUT_ENV) else {
        return DEFAULT_SCAN_TIMEOUT;
    };
    match raw.to_string_lossy().trim().parse::<u64>() {
        Ok(secs) if secs > 0 => Duration::from_secs(secs),
        _ => {
            ops_core::ui::warn(format!(
                "ignoring {SCAN_TIMEOUT_ENV}={}: expected a positive whole number of seconds; \
                 using {}s",
                raw.to_string_lossy(),
                DEFAULT_SCAN_TIMEOUT.as_secs()
            ));
            DEFAULT_SCAN_TIMEOUT
        }
    }
}

/// Full Trivy argv for `scan` over `root`: the scan's own subcommand and
/// flags, then one `--skip-dirs` pair per skip pattern, then the target
/// path.
///
/// Patterns come in two shapes (TASK-2271): unambiguous skip dirs go as
/// `**/<name>`, which matches at any depth — verified against Trivy 0.74:
/// `--skip-dirs '**/target'` skips both a top-level `target/` and a nested
/// workspace's `nested/target/` — so a monorepo's inner Cargo workspaces
/// are covered without per-manifest discovery. The generic names arrive as
/// exact discovered paths (`build`, `app/dist`): a bare name matches only
/// at the scan root and a relative path only itself (both verified against
/// Trivy 0.74), which is what keeps a checked-in `services/build` scanned
/// while generated output is skipped.
/// An empty pattern list passes no `--skip-dirs` at all, leaving Trivy's
/// own built-in defaults in charge (`--no-default-skips`).
fn trivy_argv(scan: Scan, root: &Path, skip_patterns: &[String]) -> Vec<String> {
    let mut args: Vec<String> = scan.trivy_args().iter().map(|s| (*s).to_string()).collect();
    for pattern in skip_patterns {
        args.push("--skip-dirs".to_string());
        args.push(pattern.clone());
    }
    args.push(root.to_string_lossy().into_owned());
    args
}

/// The `--skip-dirs` patterns for every scan: `**/<name>` for each
/// unambiguous shared skip dir, plus the exact discovered paths of
/// generated `build`/`dist` directories.
fn trivy_skip_patterns(generated: &[PathBuf]) -> Vec<String> {
    shared_skip_dirs()
        .iter()
        .map(|d| format!("**/{d}"))
        .chain(generated.iter().map(|p| p.to_string_lossy().into_owned()))
        .collect()
}

/// Spawn `trivy <args> <root>`, capturing its output rather than inheriting
/// stdio. Capturing lets the caller stay silent on a clean scan and only print
/// Trivy's report when something is actually found.
///
/// Goes through `ops_core::subprocess::run_with_timeout` rather than
/// `Command::output()` so the wait has a deadline and each captured stream has
/// a byte cap — the workspace's established answer for spawning a callee whose
/// runtime and output volume we do not control.
fn run_trivy(
    root: &Path,
    scan: Scan,
    timeout: Duration,
    no_default_skips: bool,
    generated: &[PathBuf],
) -> Result<Output, RunError> {
    let patterns: Vec<String> = if no_default_skips {
        Vec::new()
    } else {
        trivy_skip_patterns(generated)
    };
    let mut cmd = Command::new("trivy");
    cmd.args(trivy_argv(scan, root, &patterns));
    run_with_timeout(&mut cmd, timeout, &format!("trivy {} scan", scan.label()))
}

/// Run one scan and report it on a single quiet line: `scanning <scan> ✓` when
/// clean. On a non-zero result (findings or a Trivy error) the line ends with
/// `✗` and Trivy's captured report is printed beneath it — stdout (the report)
/// to `w`, stderr (logs/errors) to the process stderr. Returns whether the scan
/// was clean.
fn run_scan(
    root: &Path,
    scan: Scan,
    w: &mut dyn std::io::Write,
    no_default_skips: bool,
    generated: &[PathBuf],
) -> anyhow::Result<bool> {
    let timeout = scan_timeout();
    report_scan(scan, timeout, w, || {
        run_trivy(root, scan, timeout, no_default_skips, generated)
    })
}

/// Render one scan's outcome. Split from [`run_scan`] so tests can drive the
/// timeout branch against a blocking stand-in program instead of Trivy.
fn report_scan(
    scan: Scan,
    timeout: Duration,
    w: &mut dyn std::io::Write,
    run: impl FnOnce() -> Result<Output, RunError>,
) -> anyhow::Result<bool> {
    use std::io::Write as _;
    write!(w, "scanning {} ", scan.label())?;
    w.flush()?;
    let output = match run() {
        Ok(output) => output,
        // A timeout is reported as a scan outcome, not as a command failure:
        // the line still closes with the failure marker, the operator is told
        // what timed out and how to raise the budget, and the run keeps going
        // so the remaining scans still report. Fails closed via `Ok(false)`.
        Err(RunError::Timeout(_)) => {
            writeln!(w, "✗")?;
            writeln!(
                w,
                "  the {} scan timed out after {}s \
                 (raise it with {SCAN_TIMEOUT_ENV}=<seconds>)",
                scan.label(),
                timeout.as_secs()
            )?;
            w.flush()?;
            return Ok(false);
        }
        Err(e) => {
            writeln!(w, "✗")?;
            w.flush()?;
            return Err(anyhow::Error::new(e))
                .with_context(|| format!("failed to run `trivy` for the {} scan", scan.label()));
        }
    };
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

/// The `ops sec` CLI overrides in one group, mirroring `run_cmd`'s
/// `RunOptions`: `--skip` / `--force` scan selection plus
/// `--no-default-skips`. Grouping them keeps [`run_sec_to`] under the
/// argument count the workspace clippy gate enforces, and removes the
/// adjacent-bool swap risk a bare `no_default_skips: bool` parameter adds.
#[derive(Debug, Default, Clone)]
pub struct SecOverrides {
    /// Scans to drop even if detection would select them (`--skip`).
    pub skip: Vec<Scan>,
    /// Scans to run even if detection would skip them (`--force`).
    pub force: Vec<Scan>,
    /// Do not pass the default build/dependency skip dirs to Trivy
    /// (`--no-default-skips`).
    pub no_default_skips: bool,
}

/// Entry point: build the plan, preview-or-run it, and return an aggregated
/// exit code. `skip`/`force` come straight from the `--skip`/`--force` CLI
/// flags, `no_default_skips` from `--no-default-skips`. Splitting the
/// testable core into [`run_sec_to`] keeps the plan output assertable
/// without spawning Trivy.
pub fn run_sec(
    root: &Path,
    dry_run: bool,
    skip: &[ScanArg],
    force: &[ScanArg],
    no_default_skips: bool,
) -> anyhow::Result<ExitCode> {
    let overrides = SecOverrides {
        skip: skip.iter().map(|s| s.to_scan()).collect(),
        force: force.iter().map(|s| s.to_scan()).collect(),
        no_default_skips,
    };
    run_sec_to(root, dry_run, &overrides, &mut std::io::stdout())
}

fn run_sec_to(
    root: &Path,
    dry_run: bool,
    overrides: &SecOverrides,
    w: &mut dyn std::io::Write,
) -> anyhow::Result<ExitCode> {
    let SecOverrides {
        skip,
        force,
        no_default_skips,
    } = overrides;
    // A scan named in both lists is contradictory intent — reject it loudly
    // rather than silently letting one side win.
    if let Some(conflict) = skip.iter().find(|s| force.contains(s)) {
        anyhow::bail!(
            "scan `{}` was passed to both --skip and --force",
            conflict.label()
        );
    }

    // One walk feeds everything: the scan plan and the generated-build-dir
    // inventory the Trivy invocations skip per-path.
    let walk = detect(root);
    let plan = build_plan_from(walk.found, skip, force);
    let selected: Vec<Scan> = plan.iter().filter(|e| e.selected).map(|e| e.scan).collect();

    if dry_run {
        // Preview only — print the full plan (run + skip, with reasons) and
        // never execute Trivy, so do not require it installed. A heads-up keeps
        // the preview honest when it would have failed live.
        write_plan(w, &plan).context("failed to write scan plan")?;
        write_skip_dirs(w, *no_default_skips, &walk.generated)
            .context("failed to write skip-dir plan")?;
        if !trivy_on_path() {
            ops_core::ui::warn(TRIVY_MISSING_HELP);
        }
        return Ok(ExitCode::SUCCESS);
    }

    // SEC-31 (TASK-1754): zero scans selected must not fall through the loop
    // into SUCCESS. `run_cmd/plan.rs::plans_for_names` refuses the same shape for
    // the same reason — "executed zero steps, reported success" masks an
    // upstream filtering bug — and here the blast radius is a security gate
    // that reports healthy without scanning anything. Checked before the
    // `trivy` probe so an all-skipped run says what is actually wrong rather
    // than complaining about a tool it would never have spawned.
    if selected.is_empty() {
        write_no_scans_ran(w, &plan).context("failed to write empty-plan report")?;
        return Ok(ExitCode::FAILURE);
    }

    if !trivy_on_path() {
        anyhow::bail!(TRIVY_MISSING_HELP);
    }

    // Stay quiet by default: one `scanning <scan> ✓` line per scan. Run every
    // applicable scan even after one reports findings, then aggregate: success
    // only when every scan was clean.
    let mut all_ok = true;
    for scan in selected {
        if !run_scan(root, scan, w, *no_default_skips, &walk.generated)? {
            all_ok = false;
        }
    }

    Ok(if all_ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// TASK-2264 AC #5: render the directories every Trivy scan will skip, so
/// `ops sec --dry-run` previews not just *which* scans run but *what* they
/// walk. Each entry is the pattern actually passed to Trivy: `**/<dir>` for
/// the unambiguous names (any depth), the exact discovered path for each
/// generated `build`/`dist` directory (TASK-2271).
fn write_skip_dirs(
    w: &mut dyn std::io::Write,
    no_default_skips: bool,
    generated: &[PathBuf],
) -> std::io::Result<()> {
    if no_default_skips {
        writeln!(
            w,
            "  Trivy skip dirs disabled (--no-default-skips); scan auto-detection unchanged"
        )?;
        return Ok(());
    }
    let patterns = trivy_skip_patterns(generated);
    writeln!(w, "  skipping dirs: {}", patterns.join(", "))
}

/// Report an all-skipped run on the same writer the scan lines use, naming
/// every scan and why it was dropped, so the operator can see the state that
/// produced the non-zero exit.
fn write_no_scans_ran(w: &mut dyn std::io::Write, plan: &[PlanEntry]) -> std::io::Result<()> {
    writeln!(w, "ops sec: no scans ran — every scan was skipped:")?;
    for entry in plan {
        writeln!(w, "  [skip] {:<16} ({})", entry.scan.label(), entry.reason)?;
    }
    writeln!(
        w,
        "Refusing to report a clean scan when nothing was scanned. \
         Use `--dry-run` to preview a plan without running Trivy."
    )?;
    w.flush()
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
        let err = run_sec_to(
            dir.path(),
            true,
            &SecOverrides {
                skip: vec![Scan::Vuln],
                force: vec![Scan::Vuln],
                ..Default::default()
            },
            &mut buf,
        )
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
        let code = run_sec_to(dir.path(), true, &SecOverrides::default(), &mut buf)
            .expect("dry-run must not error");
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

    /// ASYNC-6 (TASK-1748): a scan that outruns its deadline must be reported
    /// as a timed-out scan — failure marker, the elapsed budget, and the
    /// escape hatch — not as a bare `RunError`, and it must fail closed.
    #[cfg(unix)]
    #[test]
    fn timed_out_scan_reports_the_timeout_and_fails_closed() {
        let timeout = Duration::from_millis(150);
        let mut buf: Vec<u8> = Vec::new();
        let clean = report_scan(Scan::Vuln, timeout, &mut buf, || {
            // Stand-in for a `trivy` blocked on an unreachable DB registry.
            let mut cmd = Command::new("sleep");
            cmd.arg("30");
            run_with_timeout(&mut cmd, timeout, "blocking stand-in")
        })
        .expect("a timeout is a scan outcome, not a command error");

        assert!(!clean, "a timed-out scan must not count as clean");
        let out = String::from_utf8(buf).expect("utf-8");
        assert!(
            out.contains('✗'),
            "line must end with the failure marker: {out}"
        );
        assert!(
            out.contains("timed out after 0s") || out.contains("timed out after"),
            "message must say the scan timed out: {out}"
        );
        assert!(
            out.contains(SCAN_TIMEOUT_ENV),
            "message must name the escape hatch: {out}"
        );
    }

    /// AC#4: the non-zero outcome of a timed-out scan reaches the aggregate.
    /// `report_scan` returning `false` is exactly what `run_sec_to`'s loop
    /// turns into `ExitCode::FAILURE`, so pin the mapping directly.
    #[cfg(unix)]
    #[test]
    fn a_timed_out_scan_makes_the_aggregate_exit_code_non_zero() {
        let timeout = Duration::from_millis(150);
        let mut buf: Vec<u8> = Vec::new();
        let clean = report_scan(Scan::Vuln, timeout, &mut buf, || {
            let mut cmd = Command::new("sleep");
            cmd.arg("30");
            run_with_timeout(&mut cmd, timeout, "blocking stand-in")
        })
        .expect("timeout must not error");
        let aggregate = if clean {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
        assert_eq!(format!("{aggregate:?}"), format!("{:?}", ExitCode::FAILURE));
    }

    /// A clean scan still reports `✓` and counts as clean.
    #[test]
    fn clean_scan_reports_the_success_marker() {
        let mut buf: Vec<u8> = Vec::new();
        let clean = report_scan(Scan::Secret, Duration::from_secs(1), &mut buf, || {
            let mut cmd = Command::new(if cfg!(windows) { "cmd" } else { "true" });
            if cfg!(windows) {
                cmd.args(["/C", "exit", "0"]);
            }
            run_with_timeout(&mut cmd, Duration::from_secs(10), "clean stand-in")
        })
        .expect("a clean scan must not error");
        assert!(clean);
        assert!(String::from_utf8(buf).unwrap().contains('✓'));
    }

    /// A spawn failure is still a hard error naming the scan, not a silent
    /// "unclean scan".
    #[test]
    fn spawn_failure_propagates_naming_the_scan() {
        let mut buf: Vec<u8> = Vec::new();
        let err = report_scan(Scan::Misconfig, Duration::from_secs(1), &mut buf, || {
            let mut cmd = Command::new("ops-no-such-program-for-tests");
            run_with_timeout(&mut cmd, Duration::from_secs(1), "missing stand-in")
        })
        .expect_err("a spawn failure must propagate");
        assert!(
            format!("{err:#}").contains("misconfiguration"),
            "error must name the scan: {err:#}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn scan_timeout_defaults_and_honours_the_env_override() {
        let guard = crate::test_utils::EnvVarGuard::unset(SCAN_TIMEOUT_ENV);
        assert_eq!(scan_timeout(), DEFAULT_SCAN_TIMEOUT);
        guard.set_value("42");
        assert_eq!(scan_timeout(), Duration::from_secs(42));
        // Nonsense and zero fall back rather than arming a 0s deadline.
        for bad in ["0", "-1", "abc", ""] {
            guard.set_value(bad);
            assert_eq!(scan_timeout(), DEFAULT_SCAN_TIMEOUT, "input: {bad:?}");
        }
    }

    /// SEC-31 (TASK-1754): `--skip` on every scan must not exit 0 with no
    /// output — that is indistinguishable from "everything ran and was clean".
    #[test]
    fn all_scans_skipped_reports_zero_scans_and_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "Cargo.lock");
        touch(dir.path(), "Dockerfile");
        let mut buf: Vec<u8> = Vec::new();
        let code = run_sec_to(
            dir.path(),
            false,
            &SecOverrides {
                skip: vec![Scan::Secret, Scan::Vuln, Scan::Misconfig],
                ..Default::default()
            },
            &mut buf,
        )
        .expect("an all-skipped run must not error");

        assert_eq!(
            format!("{code:?}"),
            format!("{:?}", ExitCode::FAILURE),
            "an all-skipped run must fail closed"
        );
        let out = String::from_utf8(buf).unwrap();
        assert!(
            out.contains("no scans ran"),
            "must say zero scans ran: {out}"
        );
        for scan in Scan::ALL {
            assert!(
                out.contains(scan.label()),
                "must list {} and its skip reason: {out}",
                scan.label()
            );
        }
        assert!(
            out.contains("skipped (--skip)"),
            "must carry the skip reasons from the plan: {out}"
        );
    }

    /// The `--dry-run` preview is unaffected: it still exits 0 for an
    /// all-skipped plan, because previewing a plan is not a scan.
    #[test]
    fn dry_run_of_an_all_skipped_plan_still_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let mut buf: Vec<u8> = Vec::new();
        let code = run_sec_to(
            dir.path(),
            true,
            &SecOverrides {
                skip: vec![Scan::Secret, Scan::Vuln, Scan::Misconfig],
                ..Default::default()
            },
            &mut buf,
        )
        .expect("dry-run must not error");
        assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
    }

    /// TASK-2264 AC #2: every scan type (secret, vuln, config) receives the
    /// unambiguous shared skip dirs as `--skip-dirs **/<dir>` before the
    /// target path. The `**/` form is what makes a nested workspace's
    /// `target/` skip too (AC #3, verified against Trivy 0.74).
    #[test]
    fn trivy_argv_passes_shared_skip_dirs_to_every_scan() {
        let patterns = trivy_skip_patterns(&[]);
        for scan in Scan::ALL {
            let argv = trivy_argv(*scan, Path::new("/proj"), &patterns);
            for dir in shared_skip_dirs() {
                let pattern = format!("**/{dir}");
                assert!(
                    argv.iter().any(|a| a == &pattern),
                    "{scan:?} argv must skip {pattern}: {argv:?}"
                );
            }
            // The generic names never appear in their blanket form — they
            // reach Trivy only as discovered per-path entries (TASK-2271).
            for generic in ops_core::stack::generic_build_dirs() {
                let blanket = format!("**/{generic}");
                assert!(
                    !argv.iter().any(|a| a == &blanket),
                    "{scan:?} argv must not blanket-skip {blanket}: {argv:?}"
                );
            }
            // `--skip-dirs` precedes its value, and the root comes last.
            assert_eq!(argv.last().unwrap(), "/proj");
            for (i, a) in argv.iter().enumerate() {
                if a == "--skip-dirs" {
                    assert!(
                        argv.get(i + 1).is_some_and(|v| !v.is_empty()),
                        "--skip-dirs must be followed by its value: {argv:?}"
                    );
                }
            }
        }
    }

    /// TASK-2271: a generated `build/` (Gradle manifest beside it) reaches
    /// Trivy as its exact discovered path — never as `**/build`, which would
    /// also exclude a checked-in `services/build`.
    #[test]
    fn trivy_argv_skips_generated_build_dirs_by_exact_path() {
        let generated = vec![PathBuf::from("build"), PathBuf::from("app/dist")];
        let patterns = trivy_skip_patterns(&generated);
        for path in &generated {
            assert!(
                patterns.iter().any(|p| *p == path.to_string_lossy()),
                "generated dir {path:?} must be skipped by exact path: {patterns:?}"
            );
        }
        assert!(
            !patterns.iter().any(|p| p == "**/build" || p == "**/dist"),
            "blanket generic patterns must never be passed: {patterns:?}"
        );
        for scan in Scan::ALL {
            let argv = trivy_argv(*scan, Path::new("/proj"), &patterns);
            assert!(
                argv.iter().any(|a| a == "app/dist"),
                "{scan:?} argv must carry the nested generated path: {argv:?}"
            );
        }
    }

    /// TASK-2264 AC #4: `--no-default-skips` passes no `--skip-dirs` at all,
    /// leaving Trivy's own built-in defaults in charge.
    #[test]
    fn trivy_argv_with_no_default_skips_passes_no_skip_dirs() {
        for scan in Scan::ALL {
            let argv = trivy_argv(*scan, Path::new("/proj"), &[]);
            assert!(
                !argv.iter().any(|a| a == "--skip-dirs"),
                "{scan:?} argv must not carry --skip-dirs: {argv:?}"
            );
        }
    }

    /// TASK-2264 AC #5: the dry-run preview names the directories every scan
    /// will skip, in the `**/<dir>` form actually passed to Trivy.
    #[test]
    fn dry_run_lists_the_skipped_directories() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "Cargo.lock");
        let mut buf: Vec<u8> = Vec::new();
        run_sec_to(dir.path(), true, &SecOverrides::default(), &mut buf)
            .expect("dry-run must not error");
        let out = String::from_utf8(buf).unwrap();
        assert!(
            out.contains("skipping dirs:"),
            "preview must name the skip list: {out}"
        );
        assert!(
            out.contains("**/target") && out.contains("**/.git"),
            "preview must list the actual Trivy patterns: {out}"
        );
    }

    /// AC #5 counterpart: with `--no-default-skips` the preview says the
    /// default list is off instead of silently keeping it.
    #[test]
    fn dry_run_with_no_default_skips_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let mut buf: Vec<u8> = Vec::new();
        run_sec_to(
            dir.path(),
            true,
            &SecOverrides {
                no_default_skips: true,
                ..Default::default()
            },
            &mut buf,
        )
        .expect("dry-run must not error");
        let out = String::from_utf8(buf).unwrap();
        assert!(
            out.contains("--no-default-skips"),
            "preview must say the default skips are disabled: {out}"
        );
        assert!(
            !out.contains("**/target"),
            "no default patterns may be listed under --no-default-skips: {out}"
        );
    }

    /// TASK-2264 AC #6: detection and Trivy derive from the one shared
    /// policy, so a marker inside *generated* build output is invisible to
    /// detection — pinned for a Gradle `build/` declared by a
    /// `build.gradle` beside it.
    #[test]
    fn markers_inside_gradle_build_dirs_ignored_by_detection() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "build.gradle");
        std::fs::create_dir_all(dir.path().join("build/gen")).unwrap();
        touch(&dir.path().join("build/gen"), "Dockerfile");
        let plan = build_plan(dir.path(), &[], &[]);
        let misconfig = plan.iter().find(|e| e.scan == Scan::Misconfig).unwrap();
        assert!(
            !misconfig.selected,
            "markers under build/ must not trigger a scan"
        );
    }

    /// TASK-2271: a `build/` with no declaring manifest beside it is
    /// checked-in source, not generated output — its markers stay
    /// detectable and it never lands in the generated inventory Trivy
    /// skips.
    #[test]
    fn checked_in_build_dirs_stay_detectable() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("services/build")).unwrap();
        touch(&dir.path().join("services/build"), "Dockerfile");
        let walk = detect(dir.path());
        let plan = build_plan_from(walk.found, &[], &[]);
        let misconfig = plan.iter().find(|e| e.scan == Scan::Misconfig).unwrap();
        assert!(
            misconfig.selected,
            "markers under a checked-in services/build must trigger a scan"
        );
        assert!(
            !walk
                .generated
                .iter()
                .any(|p| p.to_string_lossy().contains("services/build")),
            "checked-in dirs must not be skipped by Trivy: {:?}",
            walk.generated
        );
    }

    /// TASK-2271: generated `build`/`dist` directories land in the walk's
    /// inventory as root-relative paths — top-level and nested alike — and
    /// that inventory is what `trivy_skip_patterns` passes per-path.
    #[test]
    fn walk_collects_generated_dirs_as_relative_paths() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "build.gradle");
        std::fs::create_dir_all(dir.path().join("build")).unwrap();
        std::fs::create_dir_all(dir.path().join("app")).unwrap();
        touch(&dir.path().join("app"), "package.json");
        std::fs::create_dir_all(dir.path().join("app/dist")).unwrap();

        let walk = detect(dir.path());
        let mut recorded = walk.generated.clone();
        recorded.sort();
        assert_eq!(
            recorded,
            vec![PathBuf::from("app/dist"), PathBuf::from("build")],
            "generated dirs are recorded relative to the scan root"
        );

        let patterns = trivy_skip_patterns(&walk.generated);
        assert!(patterns.contains(&"build".to_string()));
        assert!(patterns.contains(&"app/dist".to_string()));
        assert!(
            !patterns.contains(&"**/build".to_string()),
            "no blanket generic pattern: {patterns:?}"
        );
    }
}
