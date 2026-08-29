#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]

pub mod model;
pub mod render;

pub use model::{Action, ClassifiedChange, Plan};
pub use render::{render_outputs_table, render_resource_table, render_summary_table};

use std::path::{Path, PathBuf};
use std::process::{ExitCode, Stdio};

use anyhow::{bail, Context};

use crate::render::sanitize_terminal_text;

const DEFAULT_BINARY_PLAN: &str = ".ops/tfplan.binary";
const DEFAULT_JSON_PLAN: &str = ".ops/tfplan.json";

/// FN-3 / TASK-1281: a single clap-derived struct is the canonical
/// definition of every `ops plans` flag.
///
/// The CLI variant carries one `PlanOptions` and the dispatch arm
/// forwards it directly to `run_plan_pipeline`, so adding a new flag
/// only edits this struct.
#[derive(clap::Args, Debug, Clone)]
pub struct PlanOptions {
    /// Read plan JSON from a file instead of running terraform. Use `-` for stdin.
    #[arg(long, value_name = "PATH")]
    pub json_file: Option<String>,
    /// Binary plan output path (default: .ops/tfplan.binary).
    #[arg(long, value_name = "PATH")]
    pub out: Option<String>,
    /// JSON plan output path (default: .ops/tfplan.json).
    #[arg(long, value_name = "PATH")]
    pub json_out: Option<String>,
    /// Keep plan artifacts after summary.
    #[arg(long)]
    pub keep_plan: bool,
    /// Force non-TTY table styling.
    #[arg(long)]
    pub no_color: bool,
    /// Forward -detailed-exitcode to terraform plan and map exit codes.
    #[arg(long)]
    pub detailed_exitcode: bool,
    /// Show planned output value changes.
    #[arg(long)]
    pub show_outputs: bool,
    /// Arguments passed through to `terraform plan` (default mode only).
    ///
    /// `-out`, `-input`, `-detailed-exitcode` and `-json` are reserved by
    /// this pipeline and are rejected here; use `--out`,
    /// `--detailed-exitcode` or `--json-file` instead.
    #[arg(last = true)]
    pub passthrough: Vec<String>,
}

/// # Errors
///
/// If `json` does not deserialize as a terraform plan document.
pub fn parse_and_classify(json: &str) -> anyhow::Result<(Plan, Vec<ClassifiedChange>)> {
    let plan: Plan = serde_json::from_str(json).context("failed to parse terraform plan JSON")?;
    let changes = classify_plan(&plan);
    Ok((plan, changes))
}

#[must_use]
pub fn has_changes(classified: &[ClassifiedChange]) -> bool {
    classified.iter().any(|c| c.action.is_change())
}

/// FN-9 / TASK-0850: thin wrapper that locks `io::stdout()` and delegates
/// to [`run_plan_pipeline_to_with_tty`].
///
/// Preserves the previous public signature so the binary entry point and
/// downstream callers stay unchanged. PATTERN-1 / TASK-1017: real
/// TTY-ness is detected on `stdout` here (via `IsTerminal`) and passed
/// through explicitly, rather than being derived from `--no-color`.
///
/// # Errors
///
/// If the plan JSON cannot be read (stdin, file, or a `terraform` invocation
/// that fails or is not on `PATH`), is empty, or does not parse as a
/// terraform plan; or if writing to the output sink fails.
pub fn run_plan_pipeline(opts: &PlanOptions) -> anyhow::Result<ExitCode> {
    use std::io::IsTerminal;
    let is_tty = std::io::stdout().is_terminal();
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    run_plan_pipeline_to_with_tty(opts, &mut handle, is_tty)
}

/// FN-9 / TASK-0850: orchestration entry point that writes rendered
/// summary / resource / outputs tables to `out` instead of global stdout.
///
/// Library callers (LSP plugin, web UI, dry-run) and tests can supply
/// their own `Vec<u8>` / file / pipe sink without spawning a subprocess.
///
/// PATTERN-1 / TASK-1017: defaults `is_tty=false` because an arbitrary
/// `&mut dyn Write` (a `Vec<u8>`, file, pipe) is not a terminal. Width
/// probing must therefore stay disabled or snapshot output becomes
/// environment-sensitive. Callers that *do* hand in a real TTY-backed
/// writer should call [`run_plan_pipeline_to_with_tty`] explicitly.
///
/// # Errors
///
/// If the plan JSON cannot be read (stdin, file, or a `terraform` invocation
/// that fails or is not on `PATH`), is empty, or does not parse as a
/// terraform plan; or if writing to the output sink fails.
pub fn run_plan_pipeline_to(
    opts: &PlanOptions,
    out: &mut dyn std::io::Write,
) -> anyhow::Result<ExitCode> {
    run_plan_pipeline_to_with_tty(opts, out, false)
}

/// PATTERN-1 / TASK-1017: explicit form that accepts the writer's
/// TTY-ness as a separate argument from the user's colour preference
/// (`opts.no_color`).
///
/// `is_tty` drives terminal-width probing in
/// `render_resource_table`; `!opts.no_color` drives whether
/// `Action::color()` is applied to cells.
///
/// # Errors
///
/// If the plan JSON cannot be read (stdin, file, or a `terraform` invocation
/// that fails or is not on `PATH`), is empty, or does not parse as a
/// terraform plan; or if writing to the output sink fails.
pub fn run_plan_pipeline_to_with_tty(
    opts: &PlanOptions,
    out: &mut dyn std::io::Write,
    is_tty: bool,
) -> anyhow::Result<ExitCode> {
    run_plan_pipeline_code(opts, out, is_tty).map(ExitCode::from)
}

/// TEST-31 / TASK-1952: `ExitCode` is opaque and does not implement
/// `PartialEq`, which is why the pipeline's most load-bearing contract —
/// the process exit status CI gates branch on — had no test. The code is
/// computed and returned as a plain `u8` here and only widened to
/// `ExitCode` at the public boundary, so it can be asserted directly.
fn run_plan_pipeline_code(
    opts: &PlanOptions,
    out: &mut dyn std::io::Write,
    is_tty: bool,
) -> anyhow::Result<u8> {
    // SEC-32 / TASK-1927: artifacts are recorded as this run creates them
    // so cleanup can run on *every* exit path below, not only on success.
    let mut created: Vec<PathBuf> = Vec::new();
    let result = plan_pipeline_body(opts, out, is_tty, &mut created);
    with_artifact_cleanup(opts, &created, result)
}

fn plan_pipeline_body(
    opts: &PlanOptions,
    out: &mut dyn std::io::Write,
    is_tty: bool,
    created: &mut Vec<PathBuf>,
) -> anyhow::Result<u8> {
    let use_color = !opts.no_color;

    let json_str = match opts.json_file.as_deref() {
        Some("-") => read_stdin()?,
        Some(path) => read_json_file(path)?,
        None => run_terraform_pipeline(opts, created)?,
    };

    if json_str.trim().is_empty() {
        bail!("plan JSON is empty");
    }

    let (plan, classified) = parse_and_classify(&json_str)?;

    let summary = render_summary_table(&classified, use_color);
    write!(out, "{summary}").context("write summary table")?;

    let changes_present = classified.iter().any(|c| c.action.is_change());
    if changes_present {
        let resources = render_resource_table(&classified, is_tty, use_color);
        write!(out, "{resources}").context("write resource table")?;
    }

    if opts.show_outputs {
        if let Some(ref outputs) = plan.output_changes {
            if !outputs.is_empty() {
                let out_tbl = render_outputs_table(outputs, use_color);
                write!(out, "{out_tbl}").context("write outputs table")?;
            }
        }
    }

    let code = if opts.detailed_exitcode && changes_present {
        2u8
    } else {
        0u8
    };
    Ok(code)
}

/// SEC-32 / TASK-1927: run artifact cleanup on both the `Ok` and the
/// `Err` arm, under the same `!keep_plan && json_file.is_none()`
/// condition the success path used to apply on its own.
///
/// Every early exit after `terraform plan` has written
/// `.ops/tfplan.binary` — an empty-JSON bail, a parse failure, a closed
/// stdout pipe (`ops plans | head`) — used to leave the binary plan on
/// disk. That file is the full planned state: provider blocks with
/// embedded credentials, generated passwords and sensitive outputs.
fn with_artifact_cleanup<T>(
    opts: &PlanOptions,
    created: &[PathBuf],
    result: anyhow::Result<T>,
) -> anyhow::Result<T> {
    if !opts.keep_plan && opts.json_file.is_none() {
        cleanup_artifacts(created);
    }
    result
}

fn classify_plan(plan: &Plan) -> Vec<ClassifiedChange> {
    plan.resource_changes
        .as_ref()
        .map(|rcs| {
            rcs.iter()
                .filter_map(|rc| {
                    // SEC-11 / TASK-1939: sanitize at ingress as well as at
                    // the render sink, so the control characters never enter
                    // the public `ClassifiedChange` a library caller may print
                    // itself. `mode` is carried but not rendered by this crate.
                    Action::classify(&rc.change.actions).map(|action| ClassifiedChange {
                        action,
                        address: sanitize_terminal_text(&rc.address),
                        resource_type: sanitize_terminal_text(
                            rc.r#type.as_deref().unwrap_or_default(),
                        ),
                        name: sanitize_terminal_text(rc.name.as_deref().unwrap_or_default()),
                        module: rc.module.as_deref().map(sanitize_terminal_text),
                        mode: rc
                            .mode
                            .as_deref()
                            .map_or_else(|| "managed".to_string(), sanitize_terminal_text),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn read_stdin() -> anyhow::Result<String> {
    read_capped(&mut std::io::stdin().lock(), "on stdin")
}

/// SEC-33 (TASK-0915 / TASK-0924 / TASK-1933) + DUP-1 (TASK-1950): the
/// single capped reader behind every plan-JSON ingress point — the
/// `--json-file` path, the `-` stdin form, and the `terraform show -json`
/// stdout of the default path. The threat model (a process piping
/// unbounded bytes, a symlink to `/dev/zero`, an adversarially large
/// plan, or a wrapped `terraform` on `PATH`) is the same for all three,
/// and they feed the same `parse_and_classify` pipeline, so the cap must
/// be uniform. Keeping one copy also keeps the three from drifting —
/// drift in a security control is a security bug.
///
/// `source` supplies the "on stdin" / "at {path}" fragment of the
/// messages. Reads up to `cap + 1` bytes so overage is detectable.
fn read_capped<R: std::io::Read>(reader: &mut R, source: &str) -> anyhow::Result<String> {
    use std::io::Read as _;
    let cap = plan_json_max_bytes();
    let limit = cap.saturating_add(1);
    let mut buf = String::new();
    reader
        .take(limit)
        .read_to_string(&mut buf)
        .with_context(|| format!("failed to read plan JSON {source}"))?;
    // `usize` is at most 64 bits on every supported target, so this widening
    // never actually saturates; the `u64::MAX` fallback would compare as
    // over-cap, which is the safe direction if that ever changed.
    if u64::try_from(buf.len()).unwrap_or(u64::MAX) > cap {
        anyhow::bail!(
            "plan JSON {source} exceeds {cap} bytes (override via {PLAN_JSON_MAX_BYTES_ENV})"
        );
    }
    Ok(buf)
}

/// SEC-33 / TASK-0915: default cap on `--json-file` reads. Real-world
/// terraform plans for large stacks routinely exceed 100 MB, so the
/// default sits well above that. Operators expecting larger plans can
/// raise the cap via `OPS_PLAN_JSON_MAX_BYTES`.
const DEFAULT_PLAN_JSON_MAX_BYTES: u64 = 256 * 1024 * 1024;
const PLAN_JSON_MAX_BYTES_ENV: &str = "OPS_PLAN_JSON_MAX_BYTES";

/// SEC-21 / SEC-33 (TASK-1936 / TASK-1933): fixed budget for captured
/// `terraform show -json` stderr. Anything beyond it is drained and
/// dropped rather than buffered, so a chatty or hostile `terraform` on
/// `PATH` cannot grow an unbounded `Vec<u8>` inside this process.
const TERRAFORM_STDERR_MAX_BYTES: u64 = 8 * 1024;

fn plan_json_max_bytes() -> u64 {
    std::env::var(PLAN_JSON_MAX_BYTES_ENV)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_PLAN_JSON_MAX_BYTES)
}

fn read_json_file(path: &str) -> anyhow::Result<String> {
    let expanded = shellexpand::full(path).with_context(|| format!("invalid path: {path}"))?;
    let mut file = std::fs::File::open(expanded.as_ref())
        .with_context(|| format!("failed to open plan JSON {path}"))?;
    read_capped(&mut file, &format!("at {path}"))
}

/// SEC-25 / TASK-1942: the artifact paths for one run, expanded exactly
/// once. Cleanup used to re-derive them from the flags a second time, so
/// any environment change between the write and the delete pointed the
/// two at different files.
#[derive(Debug)]
struct ArtifactPaths {
    binary: PathBuf,
    json: PathBuf,
}

/// FN-1 / TASK-1958: path preparation, split out of the former 80-line
/// `run_terraform_pipeline`.
///
/// ERR-13 / TASK-1945: each filesystem failure names the path involved,
/// and the two directory creations carry distinct wording so
/// "Permission denied" is attributable to one of them.
fn prepare_artifact_paths(opts: &PlanOptions) -> anyhow::Result<ArtifactPaths> {
    let binary = expand_path(opts.out.as_deref().unwrap_or(DEFAULT_BINARY_PLAN))?;
    let json = expand_path(opts.json_out.as_deref().unwrap_or(DEFAULT_JSON_PLAN))?;

    for (path, label) in [(&binary, "binary plan"), (&json, "JSON plan")] {
        let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) else {
            continue;
        };
        create_artifact_dir(parent)
            .with_context(|| format!("creating {label} artifact directory {}", parent.display()))?;
    }

    Ok(ArtifactPaths { binary, json })
}

/// SEC-29 / TASK-1930: 0700 on the artifact directory. Terraform plan
/// artifacts are among the most secret-dense files a stack produces —
/// provider configuration, generated passwords and keys, sensitive
/// output values — so no other local account on a shared build host or
/// multi-tenant runner may list or read them.
fn create_artifact_dir(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt as _;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(dir)
    }
}

/// SEC-29 / TASK-1930: tighten an artifact terraform wrote for us.
///
/// The 0700 directory already keeps other local accounts out of the
/// default `.ops/`, but `--out` may point into a directory that already
/// existed (whose mode is deliberately *not* changed — it could be
/// `$HOME`), and terraform writes the binary plan with its own umask.
/// The binary plan is the full planned state, so narrow it to 0600.
/// Best-effort: a plan we cannot chmod is not worth failing the run for.
fn harden_artifact_permissions(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        if let Err(e) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)) {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "could not restrict terraform plan artifact permissions"
                );
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

/// SEC-29 / TASK-1930: 0600 on the plan JSON. It carries the `after`
/// values of generated passwords and keys plus full provider
/// configuration, so it must not be world-readable. The mode is also
/// applied after opening, because `OpenOptions::mode` only takes effect
/// when the file is created and this path may already exist.
fn write_plan_json(path: &Path, contents: &str) -> std::io::Result<()> {
    use std::io::Write as _;
    let mut open_opts = std::fs::OpenOptions::new();
    open_opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        open_opts.mode(0o600);
    }
    let mut file = open_opts.open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    file.write_all(contents.as_bytes())
}

/// SEC-13 / TASK-1960: flags this pipeline sets itself and whose values
/// its surrounding logic depends on. Paired with the hint naming the
/// `ops plans` flag to use instead.
const RESERVED_PASSTHROUGH_FLAGS: [(&str, &str); 4] = [
    (
        "out",
        "use the `--out` flag so the pipeline reads back the plan it just wrote",
    ),
    (
        "input",
        "the pipeline always runs terraform with -input=false and a null stdout",
    ),
    ("detailed-exitcode", "use the `--detailed-exitcode` flag"),
    ("json", "use `--json-file` to supply plan JSON directly"),
];

/// SEC-13 / TASK-1960: terraform honours the *last* occurrence of a
/// repeated flag, and passthrough arguments are appended after this
/// pipeline's own. A passthrough `-out=elsewhere.tfplan` therefore
/// redirects the binary plan while `terraform show -json` still reads
/// the pipeline's path — showing the operator a *stale* plan rendered as
/// the current one, on the screen they approve an apply from. A
/// passthrough `-detailed-exitcode` makes a successful plan with changes
/// report "terraform plan failed with exit code 2".
///
/// This is not shell injection — `Command::args` is used correctly and
/// there is no shell. The defect is the absence of any check that the
/// caller is not overriding flags the surrounding logic assumes.
fn reject_reserved_passthrough(passthrough: &[String]) -> anyhow::Result<()> {
    for arg in passthrough {
        let Some(rest) = arg.strip_prefix('-') else {
            continue;
        };
        let rest = rest.strip_prefix('-').unwrap_or(rest);
        let name = rest.split('=').next().unwrap_or(rest);
        if let Some((flag, hint)) = RESERVED_PASSTHROUGH_FLAGS
            .iter()
            .find(|(f, _)| f.eq_ignore_ascii_case(name))
        {
            bail!(
                "`-{flag}` is reserved by `ops plans` and cannot be passed through to `terraform plan`: {hint}"
            );
        }
    }
    Ok(())
}

/// FN-1 / TASK-1958: orchestration only. Path preparation, the
/// `terraform plan` invocation and the JSON capture each live in their
/// own named helper.
fn run_terraform_pipeline(
    opts: &PlanOptions,
    created: &mut Vec<PathBuf>,
) -> anyhow::Result<String> {
    reject_reserved_passthrough(&opts.passthrough)?;
    let paths = prepare_artifact_paths(opts)?;
    // `run_terraform_plan` hardens the binary plan itself, immediately after
    // recording the path and before it interprets terraform's exit status, so
    // a partial artifact from a failed run is covered too.
    run_terraform_plan(opts, &paths.binary, created)?;
    let json_str = capture_plan_json(&paths.binary)?;

    if opts.keep_plan {
        write_plan_json(&paths.json, &json_str)
            .with_context(|| format!("writing plan JSON to {}", paths.json.display()))?;
        created.push(paths.json);
    }

    Ok(json_str)
}

fn run_terraform_plan(
    opts: &PlanOptions,
    binary_path: &Path,
    created: &mut Vec<PathBuf>,
) -> anyhow::Result<()> {
    let mut plan_cmd = std::process::Command::new("terraform");
    plan_cmd
        .arg("plan")
        .arg(format!("-out={}", binary_path.display()))
        .arg("-input=false")
        .arg("-no-color");

    if opts.detailed_exitcode {
        plan_cmd.arg("-detailed-exitcode");
    }

    plan_cmd.args(&opts.passthrough);

    plan_cmd.stdout(Stdio::null()).stderr(Stdio::inherit());

    let status = plan_cmd.status().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            anyhow::anyhow!(
                "`terraform` binary not found on PATH.\n\
                 install it from https://developer.hashicorp.com/terraform/install"
            )
        } else {
            // ERR-4: keep the io::Error as source() instead of flattening
            // it into the message string.
            anyhow::Error::new(e).context("failed to run terraform plan")
        }
    })?;

    // SEC-25 / SEC-32: terraform owns `-out` from the moment it starts and
    // may have written a partial artifact even on a failing exit, so record
    // the path once the process actually ran. Nothing is recorded when the
    // spawn itself failed, which is what keeps a run that never invoked
    // terraform from deleting a pre-existing file at `--out`.
    created.push(binary_path.to_path_buf());

    // SEC-29: harden the artifact *before* the exit status is interpreted.
    // A failing `terraform plan` can still have written a partial `-out`
    // file, and that partial file is just as secret-dense as a complete one.
    // Hardening only on the success path left it at terraform's umask for as
    // long as it survived on disk (and `--keep-plan` keeps it forever).
    harden_artifact_permissions(binary_path);

    plan_status_result(opts.detailed_exitcode, status.code(), status.success())
}

/// FN-1 / TASK-1958: one failure constructor shared by both exit-status
/// interpretations. The two branches differ only in which codes count as
/// success, which is why the `bail!` body was previously copy-pasted.
///
/// With `-detailed-exitcode`, terraform uses 2 for "succeeded, changes
/// present"; without it, only 0 is success.
fn plan_status_result(
    detailed_exitcode: bool,
    code: Option<i32>,
    success: bool,
) -> anyhow::Result<()> {
    let ok = if detailed_exitcode {
        matches!(code, Some(0 | 2))
    } else {
        success
    };
    if ok {
        Ok(())
    } else {
        bail!("terraform plan failed with exit code {}", code.unwrap_or(1));
    }
}

/// FN-1 / TASK-1958 + SEC-33 / TASK-1933: capture the plan document.
///
/// stdout is streamed through [`read_capped`], the same helper the file
/// and stdin branches use, instead of `Command::output()`'s unbounded
/// buffer — the default invocation was the one ingress point the
/// documented `OPS_PLAN_JSON_MAX_BYTES` control did nothing for. stderr
/// is read on its own thread so a full stderr pipe can never wedge the
/// child mid-stdout, and is bounded by [`TERRAFORM_STDERR_MAX_BYTES`].
fn capture_plan_json(binary_path: &Path) -> anyhow::Result<String> {
    let mut child = std::process::Command::new("terraform")
        .args(["show", "-json"])
        .arg(binary_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to run `terraform show -json`")?;

    let mut stderr = child
        .stderr
        .take()
        .context("`terraform show -json` stderr was not captured")?;
    let stderr_reader = std::thread::spawn(move || {
        use std::io::Read as _;
        let mut buf = Vec::new();
        let _ = (&mut stderr)
            .take(TERRAFORM_STDERR_MAX_BYTES)
            .read_to_end(&mut buf);
        // Drain the remainder so the child never blocks on a full pipe.
        let _ = std::io::copy(&mut stderr, &mut std::io::sink());
        buf
    });

    let read_result = {
        let mut stdout = child
            .stdout
            .take()
            .context("`terraform show -json` stdout was not captured")?;
        let result = read_capped(&mut stdout, "from `terraform show -json`");
        if result.is_err() {
            // Over-cap: stop the child rather than leaving it blocked on a
            // stdout pipe nobody is draining any more.
            let _ = child.kill();
        }
        result
    };

    let status = child
        .wait()
        .context("waiting for `terraform show -json` to exit")?;
    let stderr_bytes = stderr_reader.join().unwrap_or_default();

    let json_str = read_result?;

    if status.success() {
        Ok(json_str)
    } else {
        Err(show_failure_error(status.code(), &stderr_bytes))
    }
}

/// SEC-21 / TASK-1936: keep raw provider stderr out of the user-facing
/// error.
///
/// Terraform diagnostics routinely echo the offending value back to the
/// operator — invalid provider credentials, `-var` values, backend
/// connection strings, sensitive attributes named in a validation
/// failure. Splicing that into an `anyhow::Error` sends it wherever
/// errors flow, including any caller's log. The message keeps the exit
/// status; the diagnostics go to `tracing::debug!`, the same routing
/// `cleanup_artifacts` uses for operator-only detail.
fn show_failure_error(code: Option<i32>, stderr: &[u8]) -> anyhow::Error {
    if !stderr.is_empty() {
        tracing::debug!(
            stderr = %String::from_utf8_lossy(stderr),
            "`terraform show -json` diagnostics (truncated to {TERRAFORM_STDERR_MAX_BYTES} bytes)"
        );
    }
    code.map_or_else(
        || {
            anyhow::anyhow!(
                "`terraform show -json` was terminated by a signal; \
                 its diagnostics were logged at debug level"
            )
        },
        |c| {
            anyhow::anyhow!(
                "`terraform show -json` failed with exit code {c}; \
                 its diagnostics were logged at debug level"
            )
        },
    )
}

/// ERR-1 / TASK-1948: an expansion failure is propagated, not swallowed.
///
/// The previous `map_or_else` fallback to the literal string made
/// `--out '$UNSET/plan.binary'` create a directory literally named
/// `$UNSET` and write a secret-bearing artifact into it, while
/// `--json-file '$UNSET/plan.json'` reported "invalid path" for the
/// identical input. Same context wording as `read_json_file` so the two
/// flag families now behave alike.
fn expand_path(path: &str) -> anyhow::Result<PathBuf> {
    let expanded = shellexpand::full(path).with_context(|| format!("invalid path: {path}"))?;
    Ok(PathBuf::from(expanded.as_ref()))
}

/// SEC-25 / TASK-1942: delete only the artifacts *this* invocation
/// created.
///
/// The previous version re-derived both paths from the flags and
/// unlinked whatever sat there, so a default run — which never writes
/// the JSON at all — would silently delete a pre-existing
/// `ops plans --json-out ~/notes.json`.
fn cleanup_artifacts(created: &[PathBuf]) {
    for path in created {
        // SEC-25: no `exists()` probe first. That is the check-then-act
        // pattern the rule names, it races anything touching the path
        // between the two syscalls, and it adds nothing — `remove_file`
        // already reports `NotFound`.
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                // ERR-7 / TASK-0921: route best-effort cleanup failures through
                // `tracing::warn!` (mirroring `MetadataIngestor::load`) instead
                // of the user-facing `ui::note`. Cleanup is not actionable for
                // the user; the operator wants this in their log capture.
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "could not remove terraform plan artifact"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Baseline options: every flag off, no artifact paths. Tests that
    /// care about a flag set it explicitly.
    fn test_opts() -> PlanOptions {
        PlanOptions {
            json_file: None,
            out: None,
            json_out: None,
            keep_plan: false,
            no_color: true,
            detailed_exitcode: false,
            show_outputs: false,
            passthrough: vec![],
        }
    }

    fn opts_for_fixture(path: &Path) -> PlanOptions {
        PlanOptions {
            json_file: Some(path.to_string_lossy().into_owned()),
            ..test_opts()
        }
    }

    fn stage_fixture(dir: &Path, contents: &str) -> PathBuf {
        let path = dir.join("plan.json");
        std::fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn parse_minimal_fixture() {
        let json = include_str!("../tests/fixtures/minimal.json");
        let (plan, changes) = parse_and_classify(json).expect("parse should succeed");
        assert_eq!(changes.len(), 3);

        let actions: Vec<Action> = changes.iter().map(|c| c.action).collect();
        assert!(actions.contains(&Action::Create));
        assert!(actions.contains(&Action::Delete));
        assert!(actions.contains(&Action::NoOp));
        assert!(
            plan.format_version.as_deref() == Some("1.2"),
            "format_version should be 1.2"
        );
    }

    #[test]
    fn parse_replace_fixture() {
        let json = include_str!("../tests/fixtures/replace.json");
        let (_plan, changes) = parse_and_classify(json).expect("parse should succeed");
        assert_eq!(changes.len(), 2);

        let actions: Vec<Action> = changes.iter().map(|c| c.action).collect();
        assert!(actions.contains(&Action::Replace));
        assert!(actions.contains(&Action::Read));
    }

    #[test]
    fn parse_unknown_fixture_surfaces_unknown_action() {
        // SEC-31 (TASK-0833): unrecognized actions (`forget`,
        // `["import", "update"]`) must surface as `Action::Unknown`,
        // not be silently filtered out of the resource table.
        let json = include_str!("../tests/fixtures/unknown.json");
        let (_plan, changes) = parse_and_classify(json).expect("parse should succeed");
        let actions: Vec<Action> = changes.iter().map(|c| c.action).collect();
        assert_eq!(
            actions.iter().filter(|a| **a == Action::Unknown).count(),
            2,
            "both forget and import+update should surface as Unknown: {actions:?}"
        );
    }

    #[test]
    fn parse_empty_fixture() {
        let json = include_str!("../tests/fixtures/empty.json");
        let (_plan, changes) = parse_and_classify(json).expect("parse should succeed");
        assert!(changes.is_empty());
    }

    #[test]
    fn has_changes_true_when_create() {
        let changes = vec![ClassifiedChange {
            action: Action::Create,
            address: "test".into(),
            resource_type: "test".into(),
            name: "test".into(),
            module: None,
            mode: "managed".into(),
        }];
        assert!(has_changes(&changes));
    }

    /// SEC-11 / TASK-1939: control characters in the plan document must
    /// not survive into `ClassifiedChange`, which library callers may
    /// print without going through this crate's renderer.
    #[test]
    fn classify_plan_strips_control_characters_from_change_fields() {
        let json = r#"{
            "format_version": "1.2",
            "resource_changes": [
                {
                    "address": "aws_instance.web\u001b[2K",
                    "mode": "man\raged",
                    "type": "aws_\u001b[1minstance",
                    "name": "web\rfake",
                    "module": "module.a\u0007b",
                    "change": { "actions": ["create"] }
                }
            ]
        }"#;
        let (_plan, changes) = parse_and_classify(json).expect("parse should succeed");
        let change = &changes[0];
        for field in [
            &change.address,
            &change.resource_type,
            &change.name,
            &change.mode,
            change.module.as_ref().unwrap(),
        ] {
            assert!(
                !field.chars().any(char::is_control),
                "control characters must be stripped: {field:?}"
            );
        }
        assert_eq!(change.resource_type, "aws_[1minstance");
        assert_eq!(change.name, "webfake");
        assert_eq!(change.mode, "managed");
    }

    /// SEC-33 / TASK-0915: a plan JSON larger than the cap must be
    /// rejected without being slurped into memory. Override the cap to
    /// 64 bytes via `OPS_PLAN_JSON_MAX_BYTES` so the test stays fast.
    #[test]
    #[serial_test::serial(plan_json_max_bytes_env)]
    fn read_json_file_rejects_oversized_payload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.json");
        // Payload well over the 64-byte cap below.
        std::fs::write(&path, "x".repeat(1024)).unwrap();

        // SAFETY: serial-style local override; restored at end.
        let saved = std::env::var(PLAN_JSON_MAX_BYTES_ENV).ok();
        unsafe { std::env::set_var(PLAN_JSON_MAX_BYTES_ENV, "64") };
        let result = read_json_file(path.to_string_lossy().as_ref());
        unsafe {
            match saved {
                Some(v) => std::env::set_var(PLAN_JSON_MAX_BYTES_ENV, v),
                None => std::env::remove_var(PLAN_JSON_MAX_BYTES_ENV),
            }
        }
        let err = result.expect_err("oversized plan JSON must error");
        let msg = format!("{err}");
        assert!(
            msg.contains("exceeds 64 bytes"),
            "error must name the cap, got: {msg}"
        );
    }

    /// SEC-33 (TASK-0924): the stdin branch must apply the same cap as
    /// the file branch. Without this the `--json-file=-` path
    /// (`cat /dev/zero | ops terraform plan --json-file=-`) would OOM the
    /// renderer despite the file branch being capped in TASK-0915.
    #[test]
    #[serial_test::serial(plan_json_max_bytes_env)]
    fn read_stdin_rejects_oversized_payload() {
        let saved = std::env::var(PLAN_JSON_MAX_BYTES_ENV).ok();
        unsafe { std::env::set_var(PLAN_JSON_MAX_BYTES_ENV, "64") };
        let mut reader = std::io::Cursor::new(vec![b'x'; 1024]);
        let result = read_capped(&mut reader, "on stdin");
        unsafe {
            match saved {
                Some(v) => std::env::set_var(PLAN_JSON_MAX_BYTES_ENV, v),
                None => std::env::remove_var(PLAN_JSON_MAX_BYTES_ENV),
            }
        }
        let err = result.expect_err("oversized stdin plan JSON must error");
        let msg = format!("{err}");
        assert!(
            msg.contains("exceeds 64 bytes"),
            "error must name the cap, got: {msg}"
        );
        assert!(
            msg.contains(PLAN_JSON_MAX_BYTES_ENV),
            "error must name the override env var, got: {msg}"
        );
    }

    /// SEC-33 / TASK-1933: the `terraform show -json` branch now shares
    /// the capped reader, so the cap and the documented override are
    /// named for that source too — the default invocation used to be the
    /// one ingress point `OPS_PLAN_JSON_MAX_BYTES` did nothing for.
    #[test]
    #[serial_test::serial(plan_json_max_bytes_env)]
    fn read_capped_rejects_oversized_terraform_show_output() {
        let saved = std::env::var(PLAN_JSON_MAX_BYTES_ENV).ok();
        unsafe { std::env::set_var(PLAN_JSON_MAX_BYTES_ENV, "32") };
        let mut reader = std::io::Cursor::new(vec![b'x'; 4096]);
        let result = read_capped(&mut reader, "from `terraform show -json`");
        unsafe {
            match saved {
                Some(v) => std::env::set_var(PLAN_JSON_MAX_BYTES_ENV, v),
                None => std::env::remove_var(PLAN_JSON_MAX_BYTES_ENV),
            }
        }
        let msg = format!("{}", result.expect_err("oversized show output must error"));
        assert!(
            msg.contains("from `terraform show -json`"),
            "error must name the source, got: {msg}"
        );
        assert!(
            msg.contains("exceeds 32 bytes"),
            "error must name the cap, got: {msg}"
        );
        assert!(
            msg.contains(PLAN_JSON_MAX_BYTES_ENV),
            "error must name the override env var, got: {msg}"
        );
    }

    /// SEC-33 (TASK-0924): a stdin payload at or below the cap must read
    /// through unchanged.
    #[test]
    #[serial_test::serial(plan_json_max_bytes_env)]
    fn read_stdin_at_cap_returns_payload() {
        let saved = std::env::var(PLAN_JSON_MAX_BYTES_ENV).ok();
        unsafe { std::env::set_var(PLAN_JSON_MAX_BYTES_ENV, "8") };
        let mut reader = std::io::Cursor::new(b"12345678".to_vec());
        let result = read_capped(&mut reader, "on stdin");
        unsafe {
            match saved {
                Some(v) => std::env::set_var(PLAN_JSON_MAX_BYTES_ENV, v),
                None => std::env::remove_var(PLAN_JSON_MAX_BYTES_ENV),
            }
        }
        assert_eq!(result.expect("at-cap stdin payload reads ok"), "12345678");
    }

    /// FN-9 / TASK-0850: `run_plan_pipeline_to` writes its rendered tables
    /// to the provided sink instead of global stdout, and the pipeline
    /// returns `ExitCode` based on `detailed_exitcode` + `changes_present`.
    #[test]
    #[serial_test::serial(plan_json_max_bytes_env)]
    fn run_plan_pipeline_to_writes_to_supplied_buffer() {
        // Stage the minimal fixture as a file and feed it via opts.json_file
        // so we don't depend on a `terraform` binary on PATH.
        let dir = tempfile::tempdir().unwrap();
        let path = stage_fixture(dir.path(), include_str!("../tests/fixtures/minimal.json"));
        let opts = opts_for_fixture(&path);

        let mut buf: Vec<u8> = Vec::new();
        let _code = run_plan_pipeline_to(&opts, &mut buf).expect("pipeline ok");

        let out = String::from_utf8(buf).expect("utf-8");
        // Summary table (always emitted) + resource table (changes
        // present in fixture).
        assert!(out.contains("Plan:"), "must contain summary line: {out}");
        assert!(out.contains("create"), "must contain create row: {out}");
        assert!(out.contains("delete"), "must contain delete row: {out}");
        // No-op rows are filtered from the resource table.
        assert!(
            !out.contains("aws_s3_bucket"),
            "no-op must be filtered: {out}"
        );
    }

    /// TEST-31 / TASK-1952: `--detailed-exitcode` with changes present is
    /// the exit code CI gates branch on. A silent flip of 2 to 0 would
    /// let a gate report "no changes" for a plan that has them.
    #[test]
    #[serial_test::serial(plan_json_max_bytes_env)]
    fn detailed_exitcode_yields_two_when_changes_present() {
        let dir = tempfile::tempdir().unwrap();
        let path = stage_fixture(dir.path(), include_str!("../tests/fixtures/minimal.json"));
        let opts = PlanOptions {
            detailed_exitcode: true,
            ..opts_for_fixture(&path)
        };

        let mut buf: Vec<u8> = Vec::new();
        let code = run_plan_pipeline_code(&opts, &mut buf, false).expect("pipeline ok");
        assert_eq!(code, 2, "changes present under --detailed-exitcode is 2");
    }

    /// TEST-31 / TASK-1952: the other two corners of the same contract.
    #[test]
    #[serial_test::serial(plan_json_max_bytes_env)]
    fn exit_code_is_zero_without_changes_or_without_detailed_exitcode() {
        let dir = tempfile::tempdir().unwrap();

        let no_changes = dir.path().join("empty-plan.json");
        std::fs::write(&no_changes, include_str!("../tests/fixtures/empty.json")).unwrap();
        let opts = PlanOptions {
            detailed_exitcode: true,
            ..opts_for_fixture(&no_changes)
        };
        let mut buf: Vec<u8> = Vec::new();
        assert_eq!(
            run_plan_pipeline_code(&opts, &mut buf, false).expect("pipeline ok"),
            0,
            "a no-op-only plan is 0 even under --detailed-exitcode"
        );

        let with_changes =
            stage_fixture(dir.path(), include_str!("../tests/fixtures/minimal.json"));
        let opts = opts_for_fixture(&with_changes);
        let mut buf: Vec<u8> = Vec::new();
        assert_eq!(
            run_plan_pipeline_code(&opts, &mut buf, false).expect("pipeline ok"),
            0,
            "changes without --detailed-exitcode is 0"
        );
    }

    /// TEST-31 / TASK-1952: `--show-outputs` and `render_outputs_table`
    /// were never reached from a pipeline test.
    #[test]
    #[serial_test::serial(plan_json_max_bytes_env)]
    fn show_outputs_renders_the_outputs_table() {
        let dir = tempfile::tempdir().unwrap();
        let path = stage_fixture(dir.path(), include_str!("../tests/fixtures/outputs.json"));

        let opts = PlanOptions {
            show_outputs: true,
            ..opts_for_fixture(&path)
        };
        let mut buf: Vec<u8> = Vec::new();
        run_plan_pipeline_code(&opts, &mut buf, false).expect("pipeline ok");
        let out = String::from_utf8(buf).expect("utf-8");
        assert!(
            out.contains("Output"),
            "outputs table header must appear: {out}"
        );
        assert!(
            out.contains("db_password"),
            "output name must appear: {out}"
        );

        // Without the flag the same fixture must not render it.
        let opts = opts_for_fixture(&path);
        let mut buf: Vec<u8> = Vec::new();
        run_plan_pipeline_code(&opts, &mut buf, false).expect("pipeline ok");
        let out = String::from_utf8(buf).expect("utf-8");
        assert!(
            !out.contains("db_password"),
            "outputs table must be gated behind --show-outputs: {out}"
        );
    }

    /// TEST-31 / TASK-1952: the empty-plan-JSON guard.
    #[test]
    #[serial_test::serial(plan_json_max_bytes_env)]
    fn empty_plan_json_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = stage_fixture(dir.path(), "   \n\t  ");
        let opts = opts_for_fixture(&path);
        let mut buf: Vec<u8> = Vec::new();
        let err = run_plan_pipeline_code(&opts, &mut buf, false)
            .expect_err("whitespace-only plan JSON must error");
        assert!(
            format!("{err}").contains("plan JSON is empty"),
            "error must name the empty input, got: {err}"
        );
    }

    /// PATTERN-1 / TASK-1017: piped output (a `Vec<u8>` sink) must
    /// produce byte-identical bytes regardless of the host terminal
    /// width, even when the caller has *not* requested `--no-color`.
    /// Previously `is_tty` was derived from `!no_color`, so a
    /// coloured-but-piped invocation would still probe
    /// `terminal_size::terminal_size()` and width-truncate the module
    /// column based on the parent process's TTY. With colour and TTY
    /// detection decoupled, `run_plan_pipeline_to` now defaults
    /// `is_tty=false` for buffered sinks and the output is stable.
    #[test]
    #[serial_test::serial(plan_json_max_bytes_env)]
    fn run_plan_pipeline_to_buffered_sink_is_terminal_width_independent() {
        let dir = tempfile::tempdir().unwrap();
        let path = stage_fixture(dir.path(), include_str!("../tests/fixtures/minimal.json"));

        let make_opts = || PlanOptions {
            // Crucially: colour is *enabled* (no_color=false). Under the
            // old conflated `is_tty=!opts.no_color`, this would trigger
            // terminal_size probing and make the output width-dependent.
            no_color: false,
            ..opts_for_fixture(&path)
        };

        let mut buf_a: Vec<u8> = Vec::new();
        run_plan_pipeline_to(&make_opts(), &mut buf_a).expect("pipeline ok");
        let mut buf_b: Vec<u8> = Vec::new();
        run_plan_pipeline_to(&make_opts(), &mut buf_b).expect("pipeline ok");

        assert_eq!(
            buf_a, buf_b,
            "byte-identical output is required across runs for a buffered sink"
        );

        // And explicitly: passing is_tty=false through the
        // `_with_tty` form must match the default `_to` behaviour, so
        // there is one canonical "buffered sink" rendering.
        let mut buf_c: Vec<u8> = Vec::new();
        run_plan_pipeline_to_with_tty(&make_opts(), &mut buf_c, false).expect("pipeline ok");
        assert_eq!(
            buf_a, buf_c,
            "run_plan_pipeline_to must default is_tty=false"
        );
    }

    #[test]
    fn has_changes_false_when_only_noop() {
        let changes = vec![ClassifiedChange {
            action: Action::NoOp,
            address: "test".into(),
            resource_type: "test".into(),
            name: "test".into(),
            module: None,
            mode: "managed".into(),
        }];
        assert!(!has_changes(&changes));
    }

    #[test]
    fn has_changes_false_when_empty() {
        assert!(!has_changes(&[]));
    }

    /// SEC-32 / TASK-1927: an artifact produced before a later failure
    /// must not outlive the run. The binary plan is the full planned
    /// state — provider credentials, generated passwords, sensitive
    /// outputs — and the tool deletes it on the happy path, so nobody
    /// looks for the residue an error path leaves behind.
    #[test]
    fn artifacts_are_cleaned_up_on_the_error_path() {
        let dir = tempfile::tempdir().unwrap();
        let artifact = dir.path().join("tfplan.binary");
        std::fs::write(&artifact, b"binary plan").unwrap();

        let opts = test_opts();
        let created = vec![artifact.clone()];
        let result: anyhow::Result<()> = Err(anyhow::anyhow!("plan JSON is empty"));

        let out = with_artifact_cleanup(&opts, &created, result);
        assert!(out.is_err(), "the original error must be preserved");
        assert!(
            !artifact.exists(),
            "artifact must be removed on the error path"
        );
    }

    /// SEC-32 / TASK-1927: `--keep-plan` still wins on the error path.
    #[test]
    fn artifacts_survive_the_error_path_under_keep_plan() {
        let dir = tempfile::tempdir().unwrap();
        let artifact = dir.path().join("tfplan.binary");
        std::fs::write(&artifact, b"binary plan").unwrap();

        let opts = PlanOptions {
            keep_plan: true,
            ..test_opts()
        };
        let created = vec![artifact.clone()];
        let result: anyhow::Result<()> = Err(anyhow::anyhow!("plan JSON is empty"));

        let out = with_artifact_cleanup(&opts, &created, result);
        assert!(out.is_err());
        assert!(
            artifact.exists(),
            "--keep-plan must retain the artifact even when the run failed"
        );
    }

    /// SEC-25 / TASK-1942: a default run never writes the `--json-out`
    /// path, so cleanup must not unlink whatever happens to sit there.
    /// `ops plans --json-out ~/notes.json` used to delete `~/notes.json`.
    #[test]
    fn pre_existing_json_out_survives_a_run_that_never_wrote_it() {
        let dir = tempfile::tempdir().unwrap();
        let notes = dir.path().join("notes.json");
        std::fs::write(&notes, b"the user's own file").unwrap();
        let artifact = dir.path().join("tfplan.binary");
        std::fs::write(&artifact, b"binary plan").unwrap();

        let opts = PlanOptions {
            json_out: Some(notes.to_string_lossy().into_owned()),
            ..test_opts()
        };
        // Only the binary plan was actually created by this run.
        let created = vec![artifact.clone()];
        with_artifact_cleanup(&opts, &created, Ok(())).expect("cleanup path returns Ok");

        assert!(!artifact.exists(), "our own artifact is still removed");
        assert!(
            notes.exists(),
            "a file this run never wrote must not be deleted"
        );
        assert_eq!(std::fs::read(&notes).unwrap(), b"the user's own file");
    }

    /// SEC-25 / TASK-1942: `remove_file`'s `NotFound` is success, so the
    /// racy `exists()` probe is gone and a missing artifact is silent.
    #[test]
    fn cleanup_of_a_missing_artifact_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("never-created.binary");
        cleanup_artifacts(std::slice::from_ref(&missing));
        assert!(!missing.exists());
    }

    /// SEC-29 / TASK-1930: the artifact directory is 0700 and the plan
    /// JSON is 0600, so no other local account on a shared build host
    /// can read the plan's credentials and generated secrets.
    #[cfg(unix)]
    #[test]
    fn unix_artifacts_are_created_with_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = tempfile::tempdir().unwrap();
        let artifact_dir = dir.path().join("nested/.ops");
        create_artifact_dir(&artifact_dir).expect("dir creation ok");
        let dir_mode = std::fs::metadata(&artifact_dir)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(dir_mode, 0o700, "artifact directory must be 0700");

        let json_path = artifact_dir.join("tfplan.json");
        write_plan_json(&json_path, "{}").expect("json write ok");
        let file_mode = std::fs::metadata(&json_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(file_mode, 0o600, "plan JSON must be 0600");
        assert_eq!(std::fs::read_to_string(&json_path).unwrap(), "{}");

        // The binary plan is written by terraform under its own umask, so
        // it is narrowed afterwards rather than at creation.
        let binary_path = artifact_dir.join("tfplan.binary");
        std::fs::write(&binary_path, b"binary plan").unwrap();
        std::fs::set_permissions(&binary_path, std::fs::Permissions::from_mode(0o644)).unwrap();
        harden_artifact_permissions(&binary_path);
        let binary_mode = std::fs::metadata(&binary_path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(binary_mode, 0o600, "binary plan must be narrowed to 0600");

        // A missing artifact is not an error worth failing the run for.
        harden_artifact_permissions(&artifact_dir.join("absent.binary"));
    }

    /// ERR-13 / TASK-1945: an unwritable artifact path must name itself,
    /// and the two directory creations must be distinguishable from each
    /// other — "Permission denied (os error 13)" twice over is not
    /// actionable.
    #[test]
    fn artifact_directory_errors_name_the_path_and_the_role() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, b"not a directory").unwrap();
        let blocked = blocker.join("sub");
        let usable = dir.path().join("ok");

        let opts = PlanOptions {
            out: Some(blocked.join("tfplan.binary").to_string_lossy().into_owned()),
            json_out: Some(usable.join("tfplan.json").to_string_lossy().into_owned()),
            ..test_opts()
        };
        let err = prepare_artifact_paths(&opts).expect_err("unwritable parent must error");
        let msg = format!("{err}");
        assert!(
            msg.contains(&blocked.display().to_string()),
            "error must name the path, got: {msg}"
        );
        assert!(
            msg.contains("binary plan artifact directory"),
            "error must name which artifact it was for, got: {msg}"
        );

        let opts = PlanOptions {
            out: Some(usable.join("tfplan.binary").to_string_lossy().into_owned()),
            json_out: Some(blocked.join("tfplan.json").to_string_lossy().into_owned()),
            ..test_opts()
        };
        let msg = format!(
            "{}",
            prepare_artifact_paths(&opts).expect_err("unwritable parent must error")
        );
        assert!(
            msg.contains("JSON plan artifact directory"),
            "the two directory errors must be distinguishable, got: {msg}"
        );
    }

    /// ERR-1 / TASK-1948: an unexpandable `--out` errors instead of
    /// silently creating a directory literally named `$UNSET`, and
    /// reports the same "invalid path" wording `--json-file` already used
    /// for the identical input.
    #[test]
    fn unexpandable_paths_error_identically_for_out_and_json_file() {
        let unexpandable = "$OPS_TFPLAN_DEFINITELY_UNSET_1948/plan.binary";

        let opts = PlanOptions {
            out: Some(unexpandable.to_string()),
            ..test_opts()
        };
        let out_msg = format!(
            "{}",
            prepare_artifact_paths(&opts).expect_err("unexpandable --out must error")
        );
        assert!(
            out_msg.contains("invalid path"),
            "--out must report an invalid path, got: {out_msg}"
        );
        assert!(
            out_msg.contains(unexpandable),
            "the message must quote the offending value, got: {out_msg}"
        );

        let json_msg = format!(
            "{}",
            read_json_file(unexpandable).expect_err("unexpandable --json-file must error")
        );
        assert!(
            json_msg.contains("invalid path"),
            "--json-file must report an invalid path, got: {json_msg}"
        );

        // And nothing was created under the literal variable reference.
        assert!(!Path::new("$OPS_TFPLAN_DEFINITELY_UNSET_1948").exists());
    }

    /// SEC-21 / TASK-1936: raw provider stderr never reaches the
    /// user-facing error string.
    #[test]
    fn show_failure_error_omits_the_captured_stderr_body() {
        let stderr = b"Error: invalid provider credentials: AKIAIOSFODNN7EXAMPLE / wJalrXUtnFEMI";
        let msg = format!("{}", show_failure_error(Some(1), stderr));
        assert!(
            !msg.contains("AKIAIOSFODNN7EXAMPLE"),
            "the secret-bearing stderr body must not appear: {msg}"
        );
        assert!(
            !msg.contains("invalid provider credentials"),
            "no part of the stderr body may appear: {msg}"
        );
        assert!(
            msg.contains("exit code 1"),
            "the exit status must be reported: {msg}"
        );

        let signalled = format!("{}", show_failure_error(None, b""));
        assert!(
            signalled.contains("signal"),
            "a signalled child must still be described: {signalled}"
        );
    }

    /// SEC-13 / TASK-1960: every reserved flag is rejected before
    /// terraform is invoked, in both the `-flag=value` and bare forms,
    /// and the error names the `ops plans` flag to use instead.
    #[test]
    fn reserved_passthrough_flags_are_rejected() {
        for (arg, expected) in [
            ("-out=elsewhere.tfplan", "--out"),
            ("--out", "--out"),
            ("-input=true", "-input=false"),
            ("-detailed-exitcode", "--detailed-exitcode"),
            ("-json", "--json-file"),
        ] {
            let err = reject_reserved_passthrough(&[arg.to_string()])
                .expect_err("reserved flag must be rejected");
            let msg = format!("{err}");
            assert!(
                msg.contains("reserved by `ops plans`"),
                "{arg} must be reported as reserved, got: {msg}"
            );
            assert!(
                msg.contains(expected),
                "{arg} must point at {expected}, got: {msg}"
            );
        }
    }

    /// SEC-13 / TASK-1960: ordinary passthrough arguments still pass.
    #[test]
    fn ordinary_passthrough_flags_are_allowed() {
        let args = [
            "-var".to_string(),
            "env=prod".to_string(),
            "-target=aws_instance.web".to_string(),
            "-refresh=false".to_string(),
            "-parallelism=4".to_string(),
        ];
        reject_reserved_passthrough(&args).expect("non-reserved passthrough must be allowed");
    }

    /// SEC-13 / TASK-1960: the rejection happens before any subprocess
    /// runs, so it holds on a host with no `terraform` on `PATH`.
    #[test]
    fn reserved_passthrough_is_rejected_before_terraform_runs() {
        let dir = tempfile::tempdir().unwrap();
        let opts = PlanOptions {
            out: Some(dir.path().join("tfplan.binary").display().to_string()),
            json_out: Some(dir.path().join("tfplan.json").display().to_string()),
            passthrough: vec!["-out=elsewhere.tfplan".to_string()],
            ..test_opts()
        };
        let mut created = Vec::new();
        let err =
            run_terraform_pipeline(&opts, &mut created).expect_err("reserved flag must error");
        assert!(format!("{err}").contains("reserved by `ops plans`"));
        assert!(
            created.is_empty(),
            "nothing may be recorded as created before terraform runs"
        );
    }

    /// FN-1 / TASK-1958: the two exit-status interpretations now share
    /// one failure constructor; their behaviour is unchanged.
    #[test]
    fn plan_status_interpretation_is_unchanged() {
        plan_status_result(true, Some(0), true).expect("detailed 0 is success");
        plan_status_result(true, Some(2), false).expect("detailed 2 is success");
        plan_status_result(false, Some(0), true).expect("plain 0 is success");

        let err = plan_status_result(true, Some(1), false).expect_err("detailed 1 is failure");
        assert!(format!("{err}").contains("exit code 1"));

        // Without -detailed-exitcode, 2 is a plain failure.
        let err = plan_status_result(false, Some(2), false).expect_err("plain 2 is failure");
        assert!(format!("{err}").contains("exit code 2"));

        // A signalled child reports no code; the message falls back to 1.
        let err = plan_status_result(false, None, false).expect_err("signalled child is failure");
        assert!(format!("{err}").contains("exit code 1"));
    }
}
