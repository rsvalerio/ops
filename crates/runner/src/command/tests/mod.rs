//! Tests for command execution, split per concern (TASK-0423).

pub use super::*;
pub use crate::command::abort::AbortSignal;
pub use crate::command::build::{build_command, WorkspaceCanonicalCache};
pub use crate::command::events::RunnerEvent;
pub use crate::command::exec::{emit_output_events, exec_standalone, ExecTaskCtx};
pub use crate::command::results::StepResult;
pub use crate::test_support::{test_runner, EventAssertions};
pub use ops_core::config::CommandSpec;
pub use ops_core::expand::Variables;
pub use ops_core::test_utils::{
    composite_cmd, echo_cmd, exec_spec, exec_spec_with_cwd, false_cmd, parallel_cmd, sleep_cmd,
    true_cmd,
};
pub use std::collections::HashMap;
pub use std::path::PathBuf;
pub use std::sync::Arc;
pub use std::time::Duration;
pub use tokio::sync::mpsc;

pub fn test_vars() -> Variables {
    Variables::from_env(std::path::Path::new(".")).expect("UTF-8 path")
}

pub fn runner_with_test_commands() -> CommandRunner {
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
