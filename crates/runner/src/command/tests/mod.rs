//! Tests for command execution, split per concern (TASK-0423).

pub(crate) use super::*;
pub(crate) use crate::command::build::build_command;
pub(crate) use crate::command::events::RunnerEvent;
pub(crate) use crate::command::exec::{emit_output_events, exec_standalone};
pub(crate) use crate::command::results::StepResult;
pub(crate) use crate::test_support::{test_runner, EventAssertions};
pub(crate) use ops_core::config::CommandSpec;
pub(crate) use ops_core::expand::Variables;
pub(crate) use ops_core::test_utils::{
    composite_cmd, echo_cmd, exec_spec, exec_spec_with_cwd, false_cmd, parallel_cmd, sleep_cmd,
    true_cmd,
};
pub(crate) use std::collections::HashMap;
pub(crate) use std::path::PathBuf;
pub(crate) use std::sync::atomic::AtomicBool;
pub(crate) use std::sync::Arc;
pub(crate) use std::time::Duration;
pub(crate) use tokio::sync::mpsc;

pub(crate) fn test_vars() -> Variables {
    Variables::from_env(std::path::Path::new("."))
}

pub(crate) fn runner_with_test_commands() -> CommandRunner {
    let mut commands = HashMap::new();
    commands.insert(
        "build".to_string(),
        CommandSpec::Exec(exec_spec("cargo", &["build"])),
    );
    commands.insert(
        "clippy".to_string(),
        CommandSpec::Exec(exec_spec("cargo", &["clippy"])),
    );
    commands.insert(
        "verify".to_string(),
        CommandSpec::Composite(composite_cmd(&["build", "clippy"])),
    );
    test_runner(commands)
}

mod build_cmd;
mod data;
mod events;
mod exec;
mod expand;
mod parallel;
mod parallel_infra;
mod raw_mode;
mod secrets;
mod sequential;
