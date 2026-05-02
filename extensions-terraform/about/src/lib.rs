//! Terraform stack `project_identity` provider.
//!
//! Parses `.tf` files for `required_version` constraints and counts local
//! modules under `modules/*/`. No terraform subprocess — purely filesystem.
//!
//! # Manifest IO policy
//!
//! ERR-1 / TASK-0851: every `.tf` read goes through
//! [`ops_about::manifest_io::read_optional_text`] so the project-wide rule
//! "missing manifest is silent, real IO error is `tracing::warn!`-and-fall-
//! back" applies here the same way it does in the Python / Go siblings.
//! A permission-denied / EIO / "is a directory" failure on `versions.tf`
//! is therefore distinguishable from "no version declared" in the logs.
//! The directory enumeration in [`find_required_version`] mirrors the
//! same policy — non-NotFound `read_dir` failures are logged at `warn`.

use std::path::Path;

use ops_about::identity::{provide_identity_from_manifest, ParsedManifest};
use ops_core::project_identity::{base_about_fields, AboutFieldDef};
use ops_extension::{Context, DataProvider, DataProviderError, ExtensionType};

const NAME: &str = "about-terraform";
const DESCRIPTION: &str = "Terraform project identity";
const SHORTNAME: &str = "about-terraform";
const DATA_PROVIDER_NAME: &str = "project_identity";

#[non_exhaustive]
pub struct AboutTerraformExtension;

ops_extension::impl_extension! {
    AboutTerraformExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::Terraform),
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        registry.register(DATA_PROVIDER_NAME, Box::new(TerraformIdentityProvider));
    },
    factory: TERRAFORM_ABOUT_FACTORY = |_, _| {
        Some((NAME, Box::new(AboutTerraformExtension)))
    },
}

struct TerraformIdentityProvider;

impl DataProvider for TerraformIdentityProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn about_fields(&self) -> Vec<AboutFieldDef> {
        base_about_fields()
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        provide_identity_from_manifest(ctx.working_directory.as_path(), |root| {
            let required_version = find_required_version(root);
            let module_count = count_local_modules(root);

            let stack_detail = required_version.map(|v| format!("Terraform {v}"));

            ParsedManifest::build(|m| {
                m.stack_label = "Terraform";
                m.stack_detail = stack_detail;
                m.module_label = "modules";
                m.module_count = module_count;
            })
        })
    }
}

/// Scan `.tf` files for `required_version` in a `terraform` block.
///
/// Looks for patterns like `required_version = ">= 1.5"` or
/// `required_version = "~> 1.0"`. Only the first match is used.
fn find_required_version(root: &Path) -> Option<String> {
    let candidates = ["versions.tf", "main.tf", "terraform.tf", "version.tf"];
    for candidate in candidates {
        let path = root.join(candidate);
        // ERR-1 / TASK-0851: route through the shared helper so a
        // permission-denied / EIO / "is a directory" failure surfaces as
        // tracing::warn! instead of silently degrading to "no version".
        if let Some(content) = ops_about::manifest_io::read_optional_text(&path, candidate) {
            if let Some(v) = extract_required_version(&content) {
                return Some(v);
            }
        }
    }
    // Scan all .tf files as fallback. A non-NotFound read_dir failure here
    // also deserves a warn — same rationale as above for the per-candidate
    // reads. NotFound on the workspace root is silent (caller falls back).
    let entries = match std::fs::read_dir(root) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            tracing::warn!(
                root = ?root.display(),
                error = %e,
                "failed to enumerate workspace root for .tf files"
            );
            return None;
        }
    };
    // CL-3 / TASK-0852: read_dir ordering is platform-dependent (ext4
    // hash order, APFS insertion-ish order, Windows alphabetical) so the
    // first-match-wins fallback used to produce non-deterministic results
    // across operators when multiple .tf files declared different
    // `required_version` strings. Sort by filename so the chosen winner
    // is the alphabetically-first .tf file containing a constraint —
    // documented and reproducible.
    let mut tf_paths: Vec<std::path::PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "tf"))
        .collect();
    tf_paths.sort();
    for path in tf_paths {
        let kind = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "<unnamed>.tf".to_string());
        if let Some(content) = ops_about::manifest_io::read_optional_text(&path, &kind) {
            if let Some(v) = extract_required_version(&content) {
                return Some(v);
            }
        }
    }
    None
}

/// SEC-11 / TASK-0853: cap on the rendered `required_version` string.
/// HCL constraints in real configs are short (`~> 1.5`, `>= 1.0, < 2.0`,
/// etc.) — well under 64 chars. An adversarial `.tf` could otherwise
/// embed a long string that ends up rendered into the About card; we
/// truncate at this cap and log so the truncation is observable.
const REQUIRED_VERSION_MAX_LEN: usize = 64;

/// Extract `required_version` value from a single `.tf` file's content.
///
/// SEC-11 / TASK-0853: HCL standardises the value as a double-quoted
/// string. We strip a trailing `# ...` or `// ...` comment from the
/// remainder *before* the quote check (mirroring `go_mod::strip_line_comment`)
/// and require the value to be wrapped in matching `"`. A bare or
/// single-quoted value is rejected — surfacing it as the rendered version
/// would mislead the operator about what the manifest actually says.
fn extract_required_version(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }
        let Some(rest) = trimmed.strip_prefix("required_version") else {
            continue;
        };
        let rest = rest.trim();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim();
        // HCL: the value must be a double-quoted string. Reject anything
        // else so a bare `>= 1.5` or single-quoted value surfaces as
        // "no version" instead of as a misleading literal.
        let Some(after_open) = rest.strip_prefix('"') else {
            continue;
        };
        let Some(end) = after_open.find('"') else {
            continue;
        };
        // Whatever follows the closing quote must be empty after stripping
        // a trailing `#` or `//` comment — anything else (junk, a second
        // value) is malformed HCL and we'd rather surface "no version"
        // than a misleading prefix.
        let after_close = after_open[end + 1..].trim();
        let after_close_trimmed = strip_inline_comment(after_close).trim();
        if !after_close_trimmed.is_empty() {
            continue;
        }
        let v = after_open[..end].trim();
        if v.is_empty() {
            continue;
        }
        if v.len() > REQUIRED_VERSION_MAX_LEN {
            // SEC-11 / SEC-33: cap the rendered length and log.
            let truncated: String = v.chars().take(REQUIRED_VERSION_MAX_LEN).collect();
            tracing::warn!(
                original_len = v.len(),
                cap = REQUIRED_VERSION_MAX_LEN,
                "required_version value exceeds cap; truncating before rendering"
            );
            return Some(truncated);
        }
        return Some(v.to_string());
    }
    None
}

/// SEC-11 / TASK-0853: strip a trailing `# ...` or `// ...` HCL comment
/// from a line fragment that is *already known* not to contain a quoted
/// value (i.e. the post-closing-quote tail). Mirrors the equivalent
/// helper in `extensions-go::go_mod::strip_line_comment`.
fn strip_inline_comment(s: &str) -> &str {
    let mut end = s.len();
    if let Some(i) = s.find('#') {
        end = end.min(i);
    }
    if let Some(i) = s.find("//") {
        end = end.min(i);
    }
    &s[..end]
}

/// Count local modules under `modules/*/main.tf`.
fn count_local_modules(root: &Path) -> Option<usize> {
    let modules_dir = root.join("modules");
    let Ok(entries) = std::fs::read_dir(&modules_dir) else {
        return None;
    };
    let count = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter(|e| e.path().join("main.tf").exists())
        .count();
    if count > 0 {
        Some(count)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::project_identity::ProjectIdentity;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn provider_name() {
        let provider = TerraformIdentityProvider;
        assert_eq!(provider.name(), "project_identity");
    }

    #[test]
    fn about_fields_match_base() {
        let provider = TerraformIdentityProvider;
        let fields = provider.about_fields();
        let base = base_about_fields();
        assert_eq!(fields.len(), base.len());
        for (a, b) in fields.iter().zip(base.iter()) {
            assert_eq!(a.id, b.id);
        }
    }

    #[test]
    fn provide_simple_terraform_project() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("main.tf"),
            "resource \"null_resource\" \"test\" {}\n",
        );

        let provider = TerraformIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        let expected = dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert_eq!(id.name, expected);
        assert_eq!(id.stack_label, "Terraform");
        assert_eq!(id.module_label, "modules");
        assert!(id.stack_detail.is_none());
    }

    #[test]
    fn provide_with_required_version() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("versions.tf"),
            r#"terraform {
  required_version = ">= 1.5"
}
"#,
        );
        write(&dir.path().join("main.tf"), "");

        let provider = TerraformIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        assert_eq!(id.stack_detail.as_deref(), Some("Terraform >= 1.5"));
    }

    #[test]
    fn provide_with_modules() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("main.tf"), "");
        write(&dir.path().join("modules").join("api").join("main.tf"), "");
        write(
            &dir.path().join("modules").join("network").join("main.tf"),
            "",
        );
        // Not a module (no main.tf)
        std::fs::create_dir_all(dir.path().join("modules").join("empty")).unwrap();

        let provider = TerraformIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        assert_eq!(id.module_count, Some(2));
    }

    #[test]
    fn provide_no_manifest_falls_back_to_dir_name() {
        let dir = tempfile::tempdir().unwrap();

        let provider = TerraformIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        let expected = dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert_eq!(id.name, expected);
        assert_eq!(id.stack_label, "Terraform");
        assert!(id.module_count.is_none());
        assert!(id.stack_detail.is_none());
    }

    #[test]
    fn provide_populates_repository_from_git_remote() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("main.tf"), "");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(
            git_dir.join("config"),
            "[remote \"origin\"]\n\turl = https://github.com/o/r.git\n",
        )
        .unwrap();

        let provider = TerraformIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        assert_eq!(id.repository.as_deref(), Some("https://github.com/o/r"));
    }

    /// CL-3 / TASK-0852: when none of the well-known candidates matches
    /// and the fallback walks every `*.tf` in the workspace root, the
    /// chosen winner must be deterministic across platforms — we sort the
    /// directory listing by filename so the alphabetically-first .tf
    /// file that carries a `required_version` is always the one returned.
    #[test]
    fn find_required_version_fallback_is_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        // Pick filenames that fall outside the named-candidate list
        // (versions.tf, main.tf, terraform.tf, version.tf) so we exercise
        // the read_dir fallback. The alphabetically-first should win.
        write(
            &dir.path().join("a-providers.tf"),
            "terraform {\n  required_version = \"~> 1.5\"\n}\n",
        );
        write(
            &dir.path().join("z-extras.tf"),
            "terraform {\n  required_version = \">= 99.0\"\n}\n",
        );
        let v = find_required_version(dir.path());
        assert_eq!(
            v,
            Some("~> 1.5".to_string()),
            "alphabetically-first .tf file with a constraint must win"
        );
    }

    /// ERR-1 / TASK-0851: a non-NotFound IO failure on `versions.tf`
    /// (e.g. the path is a directory) must surface a `tracing::warn!`
    /// instead of silently degrading to "no version declared".
    #[test]
    fn find_required_version_warns_when_versions_tf_is_a_directory() {
        use std::sync::{Arc, Mutex};
        use tracing_subscriber::fmt::MakeWriter;

        let dir = tempfile::tempdir().unwrap();
        // Create `versions.tf` as a directory, so read_to_string fails
        // with a non-NotFound error (IsADirectory / Other on most OSes).
        std::fs::create_dir(dir.path().join("versions.tf")).unwrap();

        #[derive(Clone, Default)]
        struct BufWriter(Arc<Mutex<Vec<u8>>>);
        impl std::io::Write for BufWriter {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(b);
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

        let v = tracing::subscriber::with_default(subscriber, || find_required_version(dir.path()));
        assert!(v.is_none(), "no required_version should be returned");

        let logs = String::from_utf8(captured.lock().unwrap().clone()).unwrap();
        assert!(
            logs.contains("failed to read manifest") && logs.contains("versions.tf"),
            "warn should name versions.tf and the read failure, got: {logs}"
        );
    }

    #[test]
    fn extract_required_version_from_content() {
        let content = r#"terraform {
  required_version = "~> 1.0"
}
"#;
        assert_eq!(
            extract_required_version(content),
            Some("~> 1.0".to_string())
        );
    }

    #[test]
    fn extract_required_version_skips_comments() {
        let content = r#"# required_version = "skip"
// required_version = "also skip"
required_version = ">= 1.5"
"#;
        assert_eq!(
            extract_required_version(content),
            Some(">= 1.5".to_string())
        );
    }

    #[test]
    fn extract_required_version_none_when_absent() {
        assert_eq!(
            extract_required_version("resource \"test\" \"x\" {}\n"),
            None
        );
    }

    /// SEC-11 / TASK-0853: a trailing `# ...` comment after the quoted
    /// value must be stripped before rendering — previously the entire
    /// remainder including the comment was returned verbatim.
    #[test]
    fn extract_required_version_strips_trailing_hash_comment() {
        assert_eq!(
            extract_required_version("required_version = \">= 1.5\" # patch needed\n"),
            Some(">= 1.5".to_string())
        );
    }

    /// SEC-11 / TASK-0853: same for `// …`.
    #[test]
    fn extract_required_version_strips_trailing_slash_comment() {
        assert_eq!(
            extract_required_version("required_version = \">= 1.5\" // note\n"),
            Some(">= 1.5".to_string())
        );
    }

    /// SEC-11 / TASK-0853: a `#` inside the quoted value is part of the
    /// value, not a comment introducer.
    #[test]
    fn extract_required_version_keeps_hash_inside_quotes() {
        assert_eq!(
            extract_required_version("required_version = \">= 1.5 # marker\"\n"),
            Some(">= 1.5 # marker".to_string())
        );
    }

    /// SEC-11 / TASK-0853: HCL standardises double-quoted; a bare value
    /// (`required_version = >= 1.5 # comment`) must NOT be returned —
    /// surfacing it would mislead the operator about what the manifest
    /// actually says.
    #[test]
    fn extract_required_version_rejects_bare_value() {
        assert_eq!(
            extract_required_version("required_version = >= 1.5 # comment\n"),
            None
        );
    }

    /// SEC-11 / TASK-0853: single-quoted values are not standard HCL and
    /// are rejected.
    #[test]
    fn extract_required_version_rejects_single_quoted() {
        assert_eq!(
            extract_required_version("required_version = '>= 1.5'\n"),
            None
        );
    }

    /// SEC-11 / TASK-0853: an excessively long value is truncated to
    /// REQUIRED_VERSION_MAX_LEN before being rendered into the About card.
    #[test]
    fn extract_required_version_caps_overlong_value() {
        let long = "v".repeat(200);
        let content = format!("required_version = \"{long}\"\n");
        let v = extract_required_version(&content).expect("Some");
        assert_eq!(v.len(), REQUIRED_VERSION_MAX_LEN);
        assert!(v.chars().all(|c| c == 'v'));
    }
}
