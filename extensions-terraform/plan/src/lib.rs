pub mod model;
pub mod render;

pub use model::{Action, ClassifiedChange, Plan};
pub use render::{render_outputs_table, render_resource_table, render_summary_table};

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Context};

const DEFAULT_BINARY_PLAN: &str = ".ops/tfplan.binary";
const DEFAULT_JSON_PLAN: &str = ".ops/tfplan.json";

/// FN-3 / TASK-1281: a single clap-derived struct is the canonical
/// definition of every `ops plans` flag. The CLI variant carries one
/// `PlanOptions` and the dispatch arm forwards it directly to
/// `run_plan_pipeline`, so adding a new flag only edits this struct.
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
    #[arg(last = true)]
    pub passthrough: Vec<String>,
}

pub fn parse_and_classify(json: &str) -> anyhow::Result<(Plan, Vec<ClassifiedChange>)> {
    let plan: Plan = serde_json::from_str(json).context("failed to parse terraform plan JSON")?;
    let changes = classify_plan(&plan);
    Ok((plan, changes))
}

pub fn has_changes(classified: &[ClassifiedChange]) -> bool {
    classified.iter().any(|c| c.action.is_change())
}

/// FN-9 / TASK-0850: thin wrapper that locks `io::stdout()` and delegates
/// to [`run_plan_pipeline_to_with_tty`]. Preserves the previous public
/// signature so the binary entry point and downstream callers stay
/// unchanged. PATTERN-1 / TASK-1017: real TTY-ness is detected on
/// `stdout` here (via `IsTerminal`) and passed through explicitly,
/// rather than being derived from `--no-color`.
pub fn run_plan_pipeline(opts: PlanOptions) -> anyhow::Result<ExitCode> {
    use std::io::IsTerminal;
    let is_tty = std::io::stdout().is_terminal();
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    run_plan_pipeline_to_with_tty(opts, &mut handle, is_tty)
}

/// FN-9 / TASK-0850: orchestration entry point that writes rendered
/// summary / resource / outputs tables to `out` instead of global stdout.
/// Library callers (LSP plugin, web UI, dry-run) and tests can supply
/// their own `Vec<u8>` / file / pipe sink without spawning a subprocess.
///
/// PATTERN-1 / TASK-1017: defaults `is_tty=false` because an arbitrary
/// `&mut dyn Write` (a `Vec<u8>`, file, pipe) is not a terminal. Width
/// probing must therefore stay disabled or snapshot output becomes
/// environment-sensitive. Callers that *do* hand in a real TTY-backed
/// writer should call [`run_plan_pipeline_to_with_tty`] explicitly.
pub fn run_plan_pipeline_to(
    opts: PlanOptions,
    out: &mut dyn std::io::Write,
) -> anyhow::Result<ExitCode> {
    run_plan_pipeline_to_with_tty(opts, out, false)
}

/// PATTERN-1 / TASK-1017: explicit form that accepts the writer's
/// TTY-ness as a separate argument from the user's colour preference
/// (`opts.no_color`). `is_tty` drives terminal-width probing in
/// `render_resource_table`; `!opts.no_color` drives whether
/// `Action::color()` is applied to cells.
pub fn run_plan_pipeline_to_with_tty(
    opts: PlanOptions,
    out: &mut dyn std::io::Write,
    is_tty: bool,
) -> anyhow::Result<ExitCode> {
    let use_color = !opts.no_color;

    let json_str = match opts.json_file.as_deref() {
        Some("-") => read_stdin()?,
        Some(path) => read_json_file(path)?,
        None => run_terraform_pipeline(&opts)?,
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

    if !opts.keep_plan && opts.json_file.is_none() {
        cleanup_artifacts(&opts);
    }

    let code = if opts.detailed_exitcode && changes_present {
        2u8
    } else {
        0u8
    };
    Ok(ExitCode::from(code))
}

fn classify_plan(plan: &Plan) -> Vec<ClassifiedChange> {
    plan.resource_changes
        .as_ref()
        .map(|rcs| {
            rcs.iter()
                .filter_map(|rc| {
                    Action::classify(&rc.change.actions).map(|action| ClassifiedChange {
                        action,
                        address: rc.address.clone(),
                        resource_type: rc.r#type.clone().unwrap_or_default(),
                        name: rc.name.clone().unwrap_or_default(),
                        module: rc.module.clone(),
                        mode: rc.mode.clone().unwrap_or_else(|| "managed".into()),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn read_stdin() -> anyhow::Result<String> {
    read_stdin_capped(&mut std::io::stdin().lock())
}

/// SEC-33 (TASK-0924): byte-cap stdin reads at the same `plan_json_max_bytes`
/// limit as `read_json_file`. The threat model of TASK-0915 (a process
/// piping unbounded bytes — e.g. `cat /dev/zero | ops terraform plan
/// --json-file=-`) applies equally to the stdin branch; both feed the same
/// `parse_and_classify` pipeline so the cap should be uniform. Extracted as
/// a `Read`-generic helper so the cap can be exercised by unit tests
/// without a real stdin handle.
fn read_stdin_capped<R: std::io::Read>(reader: &mut R) -> anyhow::Result<String> {
    use std::io::Read;
    let cap = plan_json_max_bytes();
    let limit = cap.saturating_add(1);
    let mut buf = String::new();
    reader
        .take(limit)
        .read_to_string(&mut buf)
        .context("failed to read from stdin")?;
    if buf.len() as u64 > cap {
        anyhow::bail!(
            "plan JSON on stdin exceeds {cap} bytes (override via {PLAN_JSON_MAX_BYTES_ENV})"
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

fn plan_json_max_bytes() -> u64 {
    std::env::var(PLAN_JSON_MAX_BYTES_ENV)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_PLAN_JSON_MAX_BYTES)
}

fn read_json_file(path: &str) -> anyhow::Result<String> {
    use std::io::Read;
    let expanded = shellexpand::full(path).with_context(|| format!("invalid path: {path}"))?;
    let mut file = std::fs::File::open(expanded.as_ref())
        .with_context(|| format!("failed to open plan JSON {path}"))?;
    // SEC-33 / TASK-0915: cap the read so a symlink to /dev/zero or an
    // adversarially-large JSON cannot exhaust memory. Read up to cap+1
    // so we can detect overage and bail with a clear error.
    let cap = plan_json_max_bytes();
    let limit = cap.saturating_add(1);
    let mut content = String::new();
    (&mut file)
        .take(limit)
        .read_to_string(&mut content)
        .with_context(|| format!("failed to read plan JSON from {path}"))?;
    if content.len() as u64 > cap {
        anyhow::bail!(
            "plan JSON at {path} exceeds {cap} bytes (override via {PLAN_JSON_MAX_BYTES_ENV})"
        );
    }
    Ok(content)
}

fn run_terraform_pipeline(opts: &PlanOptions) -> anyhow::Result<String> {
    let binary_path = expand_path(opts.out.as_deref().unwrap_or(DEFAULT_BINARY_PLAN));
    let json_path = expand_path(opts.json_out.as_deref().unwrap_or(DEFAULT_JSON_PLAN));

    if let Some(parent) = binary_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Some(parent) = json_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

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

    plan_cmd
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit());

    let status = plan_cmd.status().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            anyhow::anyhow!(
                "`terraform` binary not found on PATH.\n\
                 install it from https://developer.hashicorp.com/terraform/install"
            )
        } else {
            anyhow::anyhow!("failed to run terraform plan: {e}")
        }
    })?;

    if opts.detailed_exitcode {
        match status.code() {
            Some(0) | Some(2) => {}
            _ => {
                bail!(
                    "terraform plan failed with exit code {}",
                    status.code().unwrap_or(1)
                );
            }
        }
    } else if !status.success() {
        bail!(
            "terraform plan failed with exit code {}",
            status.code().unwrap_or(1)
        );
    }

    let show_output = std::process::Command::new("terraform")
        .args(["show", "-json"])
        .arg(&binary_path)
        .output()
        .context("failed to run `terraform show -json`")?;

    if !show_output.status.success() {
        bail!(
            "terraform show -json failed: {}",
            String::from_utf8_lossy(&show_output.stderr)
        );
    }

    let json_str = String::from_utf8(show_output.stdout)
        .context("terraform show output is not valid UTF-8")?;

    if opts.keep_plan {
        std::fs::write(&json_path, &json_str)?;
    }

    Ok(json_str)
}

fn expand_path(path: &str) -> PathBuf {
    match shellexpand::full(path) {
        Ok(expanded) => PathBuf::from(expanded.as_ref()),
        Err(_) => PathBuf::from(path),
    }
}

fn cleanup_artifacts(opts: &PlanOptions) {
    for path_str in [
        opts.out.as_deref().unwrap_or(DEFAULT_BINARY_PLAN),
        opts.json_out.as_deref().unwrap_or(DEFAULT_JSON_PLAN),
    ] {
        let path = expand_path(path_str);
        if path.exists() {
            if let Err(e) = std::fs::remove_file(&path) {
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

    /// SEC-33 / TASK-0915: a plan JSON larger than the cap must be
    /// rejected without being slurped into memory. Override the cap to
    /// 64 bytes via OPS_PLAN_JSON_MAX_BYTES so the test stays fast.
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
        let result = read_stdin_capped(&mut reader);
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

    /// SEC-33 (TASK-0924): a stdin payload at or below the cap must read
    /// through unchanged.
    #[test]
    #[serial_test::serial(plan_json_max_bytes_env)]
    fn read_stdin_at_cap_returns_payload() {
        let saved = std::env::var(PLAN_JSON_MAX_BYTES_ENV).ok();
        unsafe { std::env::set_var(PLAN_JSON_MAX_BYTES_ENV, "8") };
        let mut reader = std::io::Cursor::new(b"12345678".to_vec());
        let result = read_stdin_capped(&mut reader);
        unsafe {
            match saved {
                Some(v) => std::env::set_var(PLAN_JSON_MAX_BYTES_ENV, v),
                None => std::env::remove_var(PLAN_JSON_MAX_BYTES_ENV),
            }
        }
        assert_eq!(result.expect("at-cap stdin payload reads ok"), "12345678");
    }

    /// FN-9 / TASK-0850: run_plan_pipeline_to writes its rendered tables
    /// to the provided sink instead of global stdout, and the pipeline
    /// returns ExitCode based on detailed_exitcode + changes_present.
    #[test]
    #[serial_test::serial(plan_json_max_bytes_env)]
    fn run_plan_pipeline_to_writes_to_supplied_buffer() {
        // Stage the minimal fixture as a file and feed it via opts.json_file
        // so we don't depend on a `terraform` binary on PATH.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plan.json");
        std::fs::write(&path, include_str!("../tests/fixtures/minimal.json")).unwrap();

        let opts = PlanOptions {
            json_file: Some(path.to_string_lossy().into_owned()),
            out: None,
            json_out: None,
            keep_plan: false,
            no_color: true,
            detailed_exitcode: false,
            show_outputs: false,
            passthrough: vec![],
        };

        let mut buf: Vec<u8> = Vec::new();
        let _code = run_plan_pipeline_to(opts, &mut buf).expect("pipeline ok");

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
        let path = dir.path().join("plan.json");
        std::fs::write(&path, include_str!("../tests/fixtures/minimal.json")).unwrap();

        let make_opts = || PlanOptions {
            json_file: Some(path.to_string_lossy().into_owned()),
            out: None,
            json_out: None,
            keep_plan: false,
            // Crucially: colour is *enabled* (no_color=false). Under the
            // old conflated `is_tty=!opts.no_color`, this would trigger
            // terminal_size probing and make the output width-dependent.
            no_color: false,
            detailed_exitcode: false,
            show_outputs: false,
            passthrough: vec![],
        };

        let mut buf_a: Vec<u8> = Vec::new();
        run_plan_pipeline_to(make_opts(), &mut buf_a).expect("pipeline ok");
        let mut buf_b: Vec<u8> = Vec::new();
        run_plan_pipeline_to(make_opts(), &mut buf_b).expect("pipeline ok");

        assert_eq!(
            buf_a, buf_b,
            "byte-identical output is required across runs for a buffered sink"
        );

        // And explicitly: passing is_tty=false through the
        // `_with_tty` form must match the default `_to` behaviour, so
        // there is one canonical "buffered sink" rendering.
        let mut buf_c: Vec<u8> = Vec::new();
        run_plan_pipeline_to_with_tty(make_opts(), &mut buf_c, false).expect("pipeline ok");
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
}
