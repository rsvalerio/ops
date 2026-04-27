//! Run-before-push hook extension: install and manage git pre-push hooks.

use ops_extension::ExtensionType;

pub const NAME: &str = "run-before-push";
pub const DESCRIPTION: &str = "Setup git pre-push hook to run an ops command of your choice";
pub const SHORTNAME: &str = "run-before-push";

pub struct RunBeforePushExtension;

ops_extension::impl_extension! {
    RunBeforePushExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::COMMAND,
    data_provider_name: None,
    register_data_providers: |_self, _registry| {},
    factory: RUN_BEFORE_PUSH_FACTORY = |_, _| {
        Some((NAME, Box::new(RunBeforePushExtension)))
    },
}

/// The shell script installed as `.git/hooks/pre-push`.
const HOOK_SCRIPT: &str = "#!/usr/bin/env bash\nexec ops run-before-push\n";

/// Environment variable that skips the run-before-push check when set to "1".
pub const SKIP_ENV_VAR: &str = "SKIP_OPS_RUN_BEFORE_PUSH";

ops_hook_common::impl_hook_wrappers! {
    name: NAME,
    hook_filename: "pre-push",
    hook_script: HOOK_SCRIPT,
    skip_env_var: SKIP_ENV_VAR,
    legacy_markers: &["ops run-before-push", "ops before-push"],
    command_help: "Run run-before-push checks before pushing",
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_hook_common::test_helpers::EnvGuard;

    // -- HOOK_SCRIPT --

    #[test]
    fn hook_script_contains_ops_run_before_push() {
        assert!(HOOK_SCRIPT.contains("ops run-before-push"));
    }

    #[test]
    fn hook_script_starts_with_shebang() {
        assert!(HOOK_SCRIPT.starts_with("#!/usr/bin/env bash"));
    }

    // -- should_skip --

    #[test]
    #[serial_test::serial]
    fn should_skip_returns_false_by_default() {
        let _guard = EnvGuard::remove(SKIP_ENV_VAR);
        assert!(!should_skip());
    }

    // -- install_hook: wrapper-specific legacy markers --

    #[test]
    fn install_hook_updates_legacy_before_push_hook() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(
            git_dir.join("hooks/pre-push"),
            "#!/bin/sh\nexec ops before-push\n",
        )
        .unwrap();

        let mut buf = Vec::new();
        let path = install_hook(&git_dir, &mut buf).expect("install_hook");

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, HOOK_SCRIPT);

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Updating outdated"));
    }

    // -- Extension metadata --

    #[test]
    fn extension_constants() {
        assert_eq!(NAME, "run-before-push");
        assert_eq!(SHORTNAME, "run-before-push");
        assert!(!DESCRIPTION.is_empty());
    }
}
