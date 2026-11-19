//! Plain-text rendering of the machine report.
//!
//! Most report strings are not ours: setting values and sources come from a
//! checkout's `.cargo/config.toml`, process names from `ps` (any local
//! process sets its own), paths and fs types from the environment and the
//! mount table. Every such string goes through `sanitise_to_string` before it reaches the
//! terminal, so an ESC / OSC sequence, a CR or LF, or a bidi override cannot
//! repaint the screen, forge a report line or hide the tmpfs warning.

use super::{FsReport, MachineReport, Setting};

// Escaped rather than dropped: this is a diagnostic page, and showing that a
// value carried control characters (as visible `\xNN` / `\u{NNNN}` text) is
// itself the finding.
use ops_core::ui::sanitise_to_string as safe;

fn setting_line(label: &str, setting: Option<&Setting>) -> String {
    setting.map_or_else(
        || format!("  {label:<15}unset"),
        |s| format!("  {label:<15}{}  [{}]", safe(&s.value), safe(&s.source)),
    )
}

fn fs_line(label: &str, fs: Option<&FsReport>) -> String {
    let Some(fs) = fs else {
        return format!("  {label:<15}unknown");
    };
    const MIB: u64 = 1024 * 1024;
    let size = match (fs.total_bytes, fs.available_bytes) {
        (Some(t), Some(a)) => format!(
            ", {} MiB ({} MiB free)",
            t.checked_div(MIB).unwrap_or(0),
            a.checked_div(MIB).unwrap_or(0)
        ),
        _ => String::new(),
    };
    let warn = if fs.tmpfs { "  WARNING: tmpfs" } else { "" };
    format!(
        "  {label:<15}{} ({}{size}){warn}",
        safe(&fs.path),
        fs.fs_type.as_deref().map_or_else(|| "unknown fs".to_string(), safe)
    )
}

/// Render the report as plain text lines.
#[must_use]
pub fn format_machine_report(report: &MachineReport) -> Vec<String> {
    let load = report
        .load_average
        .map_or_else(|| "unknown".to_string(), |[a, b, c]| format!("{a:.2} {b:.2} {c:.2}"));
    let procs = if report.build_processes.is_empty() {
        "none".to_string()
    } else {
        report
            .build_processes
            .iter()
            .map(|p| format!("{} ({})", safe(&p.name), p.pid))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut lines = vec![
        String::new(),
        "  MACHINE".to_string(),
        String::new(),
        format!("  {:<15}{}", "os", report.os),
        format!(
            "  {:<15}{}",
            "cores",
            report.cores.map_or_else(|| "unknown".to_string(), |c| c.to_string())
        ),
        format!("  {:<15}{load}", "load average"),
        format!("  {:<15}{procs}", "build procs"),
        format!(
            "  {:<15}{}",
            "host",
            report.cargo.host.as_deref().map_or_else(|| "unknown".to_string(), safe)
        ),
        setting_line("jobs", report.cargo.jobs.as_ref()),
        setting_line("rustc wrapper", report.cargo.rustc_wrapper.as_ref()),
        setting_line("target dir", Some(&report.cargo.target_dir)),
        setting_line("linker", report.cargo.linker.as_ref()),
        setting_line("rustflags", report.cargo.rustflags.as_ref()),
        setting_line("incremental", Some(&report.cargo.incremental)),
    ];
    if let Some(profiles) = &report.cargo.incremental_profiles {
        lines.push(setting_line("  dev", Some(&profiles.dev)));
        lines.push(setting_line("  release", Some(&profiles.release)));
    }
    lines.extend([
        fs_line("TMPDIR", report.tmpdir.as_ref()),
        fs_line("target fs", report.target_dir.as_ref()),
    ]);
    if report.sccache.is_some() {
        lines.push(format!("  {:<15}stats available via --json", "sccache"));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::super::{
        write_machine_report, BuildProcess, CargoSettings, IncrementalProfiles,
        MACHINE_JSON_SCHEMA_VERSION,
    };
    use super::*;

    /// The text report lists each profile under `incremental`
    /// when the profiles decide, and the JSON carries them.
    #[test]
    fn profile_incremental_is_rendered_when_no_override_applies() {
        let mut report = sample_report();
        report.cargo.incremental =
            Setting { value: "profile".to_string(), source: "default".to_string() };
        report.cargo.incremental_profiles = Some(IncrementalProfiles {
            dev: Setting { value: "true".to_string(), source: "default".to_string() },
            release: Setting { value: "true".to_string(), source: "/w/Cargo.toml".to_string() },
        });
        let text = format_machine_report(&report).join("\n");
        assert!(text.contains("    dev          true  [default]"), "{text}");
        assert!(text.contains("    release      true  [/w/Cargo.toml]"), "{text}");
        let mut out = Vec::new();
        write_machine_report(&mut out, &report, true).expect("write");
        let value: serde_json::Value = serde_json::from_slice(&out).expect("json");
        assert_eq!(
            value["cargo"]["incrementalProfiles"]["release"],
            serde_json::json!({"value": "true", "source": "/w/Cargo.toml"})
        );
    }

    fn sample_report() -> MachineReport {
        MachineReport {
            schema_version: MACHINE_JSON_SCHEMA_VERSION,
            kind: "about-machine",
            os: "linux",
            cores: Some(8),
            load_average: Some([1.0, 0.5, 0.25]),
            build_processes: vec![BuildProcess { pid: 7, name: "rustc".to_string() }],
            cargo: CargoSettings {
                host: Some("x86_64-unknown-linux-gnu".to_string()),
                jobs: Some(Setting {
                    value: "2".to_string(),
                    source: "/home/u/.cargo/config.toml".to_string(),
                }),
                rustc_wrapper: None,
                target_dir: Setting {
                    value: "/w/target".to_string(),
                    source: "default".to_string(),
                },
                linker: None,
                rustflags: None,
                incremental: Setting {
                    value: "0".to_string(),
                    source: "env:CARGO_INCREMENTAL".to_string(),
                },
                incremental_profiles: None,
                config_files: vec!["/home/u/.cargo/config.toml".to_string()],
            },
            tmpdir: Some(FsReport {
                path: "/tmp".to_string(),
                fs_type: Some("tmpfs".to_string()),
                tmpfs: true,
                total_bytes: Some(1024 * 1024 * 1024),
                available_bytes: Some(512 * 1024 * 1024),
            }),
            target_dir: None,
            sccache: None,
        }
    }

    /// Pins the `ops about machine --json` envelope and field
    /// names.
    #[test]
    fn json_document_pins_schema_and_field_names() {
        let mut out = Vec::new();
        write_machine_report(&mut out, &sample_report(), true).expect("write");
        let value: serde_json::Value = serde_json::from_slice(&out).expect("json");
        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["kind"], "about-machine");
        let keys: Vec<&str> = value.as_object().unwrap().keys().map(String::as_str).collect();
        let mut expected = vec![
            "schemaVersion",
            "kind",
            "os",
            "cores",
            "loadAverage",
            "buildProcesses",
            "cargo",
            "tmpdir",
            "targetDir",
            "sccache",
        ];
        expected.sort_unstable();
        let mut keys_sorted = keys.clone();
        keys_sorted.sort_unstable();
        assert_eq!(keys_sorted, expected);
        assert_eq!(value["cargo"]["jobs"]["source"], "/home/u/.cargo/config.toml");
        assert_eq!(value["cargo"]["targetDir"]["value"], "/w/target");
        assert_eq!(
            value["cargo"]["incremental"],
            serde_json::json!({"value": "0", "source": "env:CARGO_INCREMENTAL"})
        );
        assert!(value["cargo"]["incrementalProfiles"].is_null());
        assert_eq!(value["tmpdir"]["tmpfs"], true);
        assert_eq!(value["tmpdir"]["fsType"], "tmpfs");
        assert_eq!(value["buildProcesses"][0]["name"], "rustc");
        let text = String::from_utf8(out).unwrap();
        assert!(text
            .trim_start()
            .starts_with("{\n  \"schemaVersion\": 1,\n  \"kind\": \"about-machine\""));
    }

    #[test]
    fn text_report_flags_tmpfs_and_names_sources() {
        let text = format_machine_report(&sample_report()).join("\n");
        assert!(text.contains("jobs           2  [/home/u/.cargo/config.toml]"), "{text}");
        assert!(text.contains("WARNING: tmpfs"), "{text}");
        assert!(text.contains("incremental    0  [env:CARGO_INCREMENTAL]"), "{text}");
        assert!(!text.contains("    dev "), "{text}");
        assert!(text.contains("rustc (7)"), "{text}");
        assert!(text.contains("1024 MiB (512 MiB free)"), "{text}");
    }

    /// Repo- and process-controlled strings (a checked-in linker or
    /// rustflags, a `ps` process name, a config path, a mount's fs type)
    /// reach the text report escaped: no ESC, CR, LF or bidi override
    /// survives to repaint the terminal or forge a line.
    #[test]
    fn text_report_escapes_control_and_bidi_chars_in_untrusted_values() {
        let evil = "x\u{1b}]8;;https://evil\u{7}\r\nWARNING: none\u{202e}";
        let setting = || Setting { value: evil.to_string(), source: evil.to_string() };
        let mut report = sample_report();
        report.build_processes = vec![BuildProcess { pid: 9, name: evil.to_string() }];
        report.cargo.host = Some(evil.to_string());
        report.cargo.linker = Some(setting());
        report.cargo.rustflags = Some(setting());
        report.cargo.target_dir = setting();
        report.target_dir = Some(FsReport {
            path: evil.to_string(),
            fs_type: Some(evil.to_string()),
            tmpfs: false,
            total_bytes: None,
            available_bytes: None,
        });
        let lines = format_machine_report(&report);
        for line in &lines {
            for bad in ['\u{1b}', '\r', '\n', '\u{7}', '\u{202e}'] {
                assert!(!line.contains(bad), "{bad:?} survived in {line:?}");
            }
        }
        let text = lines.join("\n");
        assert!(
            text.contains("x\\x1b]8;;https://evil\\x07\\x0d\\x0aWARNING: none\\u{202e}"),
            "{text}"
        );
    }
}
