pub mod model;
pub mod render;

pub use model::{Action, ClassifiedChange, Plan};
pub use render::{render_outputs_table, render_resource_table, render_summary_table};

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Context};

const DEFAULT_BINARY_PLAN: &str = ".ops/tfplan.binary";
const DEFAULT_JSON_PLAN: &str = ".ops/tfplan.json";

pub struct PlanOptions {
    pub json_file: Option<String>,
    pub out: Option<String>,
    pub json_out: Option<String>,
    pub keep_plan: bool,
    pub no_color: bool,
    pub detailed_exitcode: bool,
    pub show_outputs: bool,
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

pub fn run_plan_pipeline(opts: PlanOptions) -> anyhow::Result<ExitCode> {
    let is_tty = !opts.no_color;

    let json_str = match opts.json_file.as_deref() {
        Some("-") => read_stdin()?,
        Some(path) => read_json_file(path)?,
        None => run_terraform_pipeline(&opts)?,
    };

    if json_str.trim().is_empty() {
        bail!("plan JSON is empty");
    }

    let (plan, classified) = parse_and_classify(&json_str)?;

    let summary = render_summary_table(&classified, is_tty);
    print!("{summary}");

    let changes_present = classified.iter().any(|c| c.action.is_change());
    if changes_present {
        let resources = render_resource_table(&classified, is_tty);
        print!("{resources}");
    }

    if opts.show_outputs {
        if let Some(ref outputs) = plan.output_changes {
            if !outputs.is_empty() {
                let out_tbl = render_outputs_table(outputs, is_tty);
                print!("{out_tbl}");
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
    use std::io::Read;
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .context("failed to read from stdin")?;
    Ok(buf)
}

fn read_json_file(path: &str) -> anyhow::Result<String> {
    let expanded = shellexpand::full(path).with_context(|| format!("invalid path: {path}"))?;
    std::fs::read_to_string(expanded.as_ref())
        .with_context(|| format!("failed to read plan JSON from {path}"))
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
