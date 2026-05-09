//! Filesystem walk that resolves a [`Stack`] from manifest presence.
//!
//! ARCH-1 / TASK-1185: extracted from the monolithic `stack.rs` so the
//! ancestor-walk and per-extension probe code lives separately from the
//! enum + embedded TOML metadata table.

use std::path::Path;

use super::Stack;

/// SEC-25: probe a manifest path with `try_exists` so transient errors are
/// logged rather than silently swallowed by `Path::exists`.
pub(super) fn manifest_present(path: &Path) -> bool {
    match path.try_exists() {
        Ok(present) => present,
        Err(err) => {
            // ERR-7 (TASK-0945): Debug-format path/error so a CWD-relative
            // ancestor probe path containing newlines / ANSI escapes cannot
            // forge log records.
            tracing::debug!(
                path = ?path.display(),
                error = ?err,
                "stack manifest probe failed; treating as not present",
            );
            false
        }
    }
}

/// File extensions used for extension-based detection (in addition to exact manifest files).
fn manifest_extensions(stack: Stack) -> &'static [&'static str] {
    match stack {
        Stack::Terraform => &["tf"],
        _ => &[],
    }
}

/// Whether `stack` has a manifest (exact filename or extension match) in `dir`.
pub(super) fn has_manifest_in_dir(stack: Stack, dir: &Path) -> bool {
    if stack
        .manifest_files()
        .iter()
        .any(|f| manifest_present(&dir.join(f)))
    {
        return true;
    }
    let extensions = manifest_extensions(stack);
    if !extensions.is_empty() {
        if let Ok(entries) = dir.read_dir() {
            // ERR-1 (TASK-0935): explicit match so a per-entry IO error
            // leaves a `tracing::debug` breadcrumb instead of silently
            // making the manifest "not found".
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::debug!(
                            parent = ?dir.display(),
                            error = ?e,
                            "stack manifest extension probe: read_dir entry failed; skipping",
                        );
                        continue;
                    }
                };
                if let Some(ext) = entry.path().extension() {
                    if extensions.iter().any(|e| ext == *e) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Walk ancestors of `start` looking for a manifest match.
///
/// SEC-25 / TASK-0902: canonicalize once so the `pop()` walk operates on
/// the resolved chain. Reaching the cwd through a symlink would otherwise
/// let lexical `..` traversal yield ancestors outside the canonical
/// workspace, picking up a sibling project's manifests.
pub(super) fn detect(start: &Path) -> Option<Stack> {
    // Priority order for detection (Generic is excluded — no manifest files).
    const DETECT_ORDER: &[Stack] = &[
        Stack::Rust,
        Stack::Node,
        Stack::Go,
        Stack::Python,
        Stack::Terraform,
        Stack::Ansible,
        Stack::JavaGradle,
        Stack::JavaMaven,
    ];

    let mut current = match std::fs::canonicalize(start) {
        Ok(p) => p,
        Err(e) => {
            tracing::debug!(
                path = ?start.display(),
                error = ?e,
                "Stack::detect could not canonicalize start; falling back to lexical walk"
            );
            start.to_path_buf()
        }
    };
    for _ in 0..Stack::MAX_DETECT_DEPTH {
        if let Some(&stack) = DETECT_ORDER
            .iter()
            .find(|s| has_manifest_in_dir(**s, &current))
        {
            return Some(stack);
        }
        if !current.pop() {
            return None;
        }
    }
    None
}
