use super::*;

const WORKSPACE: &str =
    "[workspace]\nmembers = [\"crates/*\"]\n\n[workspace.package]\nrust-version = \"1.85\"\n";

fn workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("Cargo.toml"), WORKSPACE).expect("root manifest");
    for name in ["a", "b"] {
        let member = dir.path().join("crates").join(name);
        std::fs::create_dir_all(&member).expect("member dir");
        std::fs::write(
            member.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
        )
        .expect("member manifest");
    }
    dir
}

fn read(dir: &tempfile::TempDir, rel: &str) -> String {
    std::fs::read_to_string(dir.path().join(rel)).expect("read")
}

fn no_waivers() -> IndexMap<String, String> {
    IndexMap::new()
}

#[test]
fn every_template_parses() {
    for file in &FILES {
        template_value(file.template).unwrap_or_else(|e| panic!("{}: {e:#}", file.path));
    }
    let lints = template_value(LINTS_TEMPLATE).expect("lints template");
    assert!(lints.get("lints").and_then(|l| l.get("clippy")).is_some());
}

#[test]
fn scaffold_then_check_is_clean() {
    let dir = workspace();
    let written = scaffold(dir.path(), false).expect("scaffold");
    let targets: Vec<(&str, &Outcome)> = written
        .iter()
        .map(|w| (w.target.as_str(), &w.outcome))
        .collect();
    assert_eq!(
        targets,
        vec![
            ("clippy.toml", &Outcome::Created),
            ("deny.toml", &Outcome::Created),
            ("rustfmt.toml", &Outcome::Created),
            (".config/nextest.toml", &Outcome::Created),
            ("mise.toml", &Outcome::Created),
            ("Cargo.toml:workspace.lints", &Outcome::Added),
            ("crates/a/Cargo.toml:lints", &Outcome::Added),
            ("crates/b/Cargo.toml:lints", &Outcome::Added),
        ]
    );
    let report = check(dir.path(), &no_waivers()).expect("check");
    assert!(report.drift.is_empty(), "{:?}", report.drift);

    let manifest = read(&dir, "Cargo.toml");
    assert!(
        manifest.starts_with(WORKSPACE),
        "existing content kept: {manifest}"
    );
    assert!(manifest.contains("[workspace.lints.clippy]"));
    assert!(
        !manifest.contains("\n[lints."),
        "headers rewritten for a workspace"
    );
    assert!(read(&dir, "crates/a/Cargo.toml").ends_with("\n\n[lints]\nworkspace = true\n"));
}

#[test]
fn clippy_msrv_follows_rust_version_and_is_not_baseline() {
    let dir = workspace();
    scaffold(dir.path(), false).expect("scaffold");
    assert!(read(&dir, "clippy.toml").contains("\nmsrv = \"1.85\"\n"));
    let edited = read(&dir, "clippy.toml").replace("msrv = \"1.85\"", "msrv = \"1.90\"");
    std::fs::write(dir.path().join("clippy.toml"), edited).expect("write");
    assert!(check(dir.path(), &no_waivers())
        .expect("check")
        .drift
        .is_empty());
}

#[test]
fn a_non_numeric_rust_version_is_not_rendered() {
    assert!(is_plain_version("1.85"));
    assert!(is_plain_version("1.85.0"));
    assert!(!is_plain_version("1.85\"\nx = \"y"));
    assert!(!is_plain_version("1..85"));
    assert!(!is_plain_version(""));
}

#[test]
fn scaffold_keeps_local_edits_without_force() {
    let dir = workspace();
    std::fs::write(dir.path().join("deny.toml"), "# mine\n").expect("seed");
    let written = scaffold(dir.path(), false).expect("scaffold");
    assert_eq!(written[1].outcome, Outcome::Kept);
    assert_eq!(read(&dir, "deny.toml"), "# mine\n");

    let again = scaffold(dir.path(), false).expect("second scaffold");
    assert!(
        again.iter().all(|w| w.outcome == Outcome::Kept),
        "{again:?}"
    );
    assert_eq!(
        read(&dir, "Cargo.toml")
            .matches("[workspace.lints.clippy]")
            .count(),
        1
    );
}

#[test]
fn force_replaces_files_and_the_lint_table() {
    let dir = workspace();
    scaffold(dir.path(), false).expect("scaffold");
    std::fs::write(dir.path().join("deny.toml"), "# mine\n").expect("seed");
    let manifest = read(&dir, "Cargo.toml").replace("panic = \"warn\"", "panic = \"allow\"");
    std::fs::write(dir.path().join("Cargo.toml"), manifest).expect("edit");

    let written = scaffold(dir.path(), true).expect("force scaffold");
    assert_eq!(written[1].outcome, Outcome::Replaced);
    assert_eq!(written[5].outcome, Outcome::Replaced);
    assert!(read(&dir, "deny.toml").contains("[licenses]"));
    let manifest = read(&dir, "Cargo.toml");
    assert_eq!(manifest.matches("[workspace.lints.clippy]").count(), 1);
    assert!(manifest.contains("\npanic = \"warn\""));
    assert!(check(dir.path(), &no_waivers())
        .expect("check")
        .drift
        .is_empty());
}

#[test]
fn check_reports_drift_with_locations() {
    let dir = workspace();
    scaffold(dir.path(), false).expect("scaffold");
    std::fs::remove_file(dir.path().join("rustfmt.toml")).expect("rm");
    let clippy = read(&dir, "clippy.toml").replace(
        "cognitive-complexity-threshold = 25",
        "cognitive-complexity-threshold = 30",
    );
    std::fs::write(dir.path().join("clippy.toml"), clippy).expect("edit");
    std::fs::write(
        dir.path().join("crates/b/Cargo.toml"),
        "[package]\nname = \"b\"\n",
    )
    .expect("edit");

    let report = check(dir.path(), &no_waivers()).expect("check");
    let locations: Vec<&str> = report.drift.iter().map(|d| d.location.as_str()).collect();
    assert_eq!(
        locations,
        vec![
            "clippy.toml:cognitive-complexity-threshold",
            "rustfmt.toml",
            "crates/b/Cargo.toml:lints.workspace",
        ]
    );
}

#[test]
fn additions_are_not_drift_and_stricter_lints_pass() {
    let dir = workspace();
    scaffold(dir.path(), false).expect("scaffold");
    let deny = read(&dir, "deny.toml").replace(
        "yanked = \"warn\"",
        "yanked = \"warn\"\nignore = [\"RUSTSEC-2020-0163\"]",
    );
    std::fs::write(dir.path().join("deny.toml"), deny).expect("edit");
    let manifest = read(&dir, "Cargo.toml").replace("= \"warn\"", "= \"deny\"");
    std::fs::write(dir.path().join("Cargo.toml"), manifest).expect("edit");
    assert!(check(dir.path(), &no_waivers())
        .expect("check")
        .drift
        .is_empty());
}

#[test]
fn waivers_cover_files_and_tables_and_stale_ones_are_reported() {
    let dir = workspace();
    scaffold(dir.path(), false).expect("scaffold");
    std::fs::remove_file(dir.path().join("rustfmt.toml")).expect("rm");
    let manifest = read(&dir, "Cargo.toml").replace("todo = \"warn\"\n", "");
    std::fs::write(dir.path().join("Cargo.toml"), manifest).expect("edit");

    let waivers: IndexMap<String, String> = [
        ("rustfmt.toml", "keeps rustfmt defaults"),
        ("Cargo.toml:workspace.lints.clippy", "own clippy policy"),
        ("deny.toml:graph", "nothing drifts here"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .collect();
    let report = check(dir.path(), &waivers).expect("check");
    assert!(report.drift.is_empty(), "{:?}", report.drift);
    assert_eq!(report.waived.len(), 2);
    assert_eq!(report.unused_waivers, vec!["deny.toml:graph".to_owned()]);
}

#[test]
fn waiver_prefix_matching_respects_segment_boundaries() {
    assert!(waiver_covers("rustfmt.toml", "rustfmt.toml"));
    assert!(waiver_covers("rustfmt.toml", "rustfmt.toml:max_width"));
    assert!(waiver_covers(
        "Cargo.toml:workspace.lints",
        "Cargo.toml:workspace.lints.clippy.panic"
    ));
    assert!(!waiver_covers(
        "clippy.toml:allow",
        "clippy.toml:allow-unwrap-in-tests"
    ));
    assert!(!waiver_covers(
        "Cargo.toml",
        "crates/a/Cargo.toml:lints.workspace"
    ));
}

#[test]
fn single_package_gets_lints_directly() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"solo\"\nversion = \"0.1.0\"\nrust-version = \"1.80\"\n",
    )
    .expect("manifest");
    let written = scaffold(dir.path(), false).expect("scaffold");
    assert_eq!(
        written.last().map(|w| w.target.as_str()),
        Some("Cargo.toml:lints")
    );
    let manifest = read(&dir, "Cargo.toml");
    assert!(manifest.contains("\n[lints.clippy]\n"));
    assert!(read(&dir, "clippy.toml").contains("msrv = \"1.80\""));
    assert!(check(dir.path(), &no_waivers())
        .expect("check")
        .drift
        .is_empty());
}

#[test]
fn a_root_that_is_also_a_package_opts_itself_in() {
    let dir = workspace();
    let root = format!("[package]\nname = \"root\"\nversion = \"0.1.0\"\n\n{WORKSPACE}");
    std::fs::write(dir.path().join("Cargo.toml"), root).expect("manifest");
    scaffold(dir.path(), false).expect("scaffold");
    let manifest = read(&dir, "Cargo.toml");
    assert!(
        manifest.contains("\n[lints]\nworkspace = true\n"),
        "{manifest}"
    );
    assert!(check(dir.path(), &no_waivers())
        .expect("check")
        .drift
        .is_empty());
}

#[test]
fn missing_manifest_is_an_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let err = scaffold(dir.path(), false).expect_err("no Cargo.toml");
    assert!(format!("{err:#}").contains("Cargo.toml"));
}

#[cfg(unix)]
#[test]
fn a_symlinked_config_dir_is_refused() {
    let dir = workspace();
    let outside = tempfile::tempdir().expect("outside");
    std::os::unix::fs::symlink(outside.path(), dir.path().join(".config")).expect("symlink");
    let err = scaffold(dir.path(), false).expect_err("symlinked .config");
    assert!(format!("{err:#}").contains("symlink"));
    assert!(!outside.path().join("nextest.toml").exists());
}

fn mise_template() -> Value {
    let file = FILES
        .iter()
        .find(|f| f.path == "mise.toml")
        .expect("mise.toml is a foundation file");
    template_value(file.template).expect("mise template")
}

#[test]
fn mise_template_pins_rust_components_and_a_check_only_ops() {
    let mise = mise_template();
    let tools = mise.get("tools").expect("[tools]");
    let rust = tools.get("rust").expect("rust pin");
    assert_eq!(
        rust.get("components").and_then(Value::as_str),
        Some("rustfmt,clippy")
    );
    let ops = tools.get("ops").and_then(Value::as_str).expect("ops pin");
    let parts: Vec<u64> = ops
        .split('.')
        .map(|p| p.parse().expect("numeric ops pin"))
        .collect();
    assert!(
        parts >= vec![0, 77, 0],
        "ops {ops} predates check-only verify"
    );
    for tool in [
        "cargo-deny",
        "cargo-machete",
        "cargo-nextest",
        "cargo-edit",
        "trivy",
        "aqua:taiki-e/cargo-llvm-cov",
    ] {
        assert!(tools.get(tool).is_some(), "{tool} is pinned");
    }
    for alias in ["ops", "cargo-nextest", "cargo-machete", "cargo-edit"] {
        assert!(
            mise.get("tool_alias").and_then(|a| a.get(alias)).is_some(),
            "{alias} is aliased"
        );
    }
}

#[test]
fn ops_own_mise_toml_matches_the_template() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mise.toml");
    let text = std::fs::read_to_string(&path).expect("ops's mise.toml");
    let actual = Value::Table(toml::from_str(&text).expect("ops's mise.toml parses"));
    let mut drift = Vec::new();
    compare::compare(
        &mise_template(),
        Some(&actual),
        "mise.toml",
        Rule::Exact,
        &mut drift,
    );
    assert!(drift.is_empty(), "{drift:?}");
}

#[test]
fn mise_pin_drift_is_reported_and_tool_additions_are_not() {
    let dir = workspace();
    scaffold(dir.path(), false).expect("scaffold");
    let mise = read(&dir, "mise.toml")
        .replace("trivy = \"0.70.0\"", "trivy = \"0.69.0\"")
        .replace("cargo-deny = \"0.20.2\"\n", "")
        + "cocogitto = \"7.0.0\"\n";
    std::fs::write(dir.path().join("mise.toml"), mise).expect("edit");

    let report = check(dir.path(), &no_waivers()).expect("check");
    assert_eq!(
        report.drift,
        vec![
            Drift::new("mise.toml:tools.cargo-deny", "missing"),
            Drift::new(
                "mise.toml:tools.trivy",
                "expected \"0.70.0\", found \"0.69.0\""
            ),
        ]
    );

    let waivers: IndexMap<String, String> = [
        (
            "mise.toml:tools.trivy",
            "held back until the scanner fix ships",
        ),
        ("mise.toml:tools.cargo-deny", "installed by the distro"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .collect();
    let report = check(dir.path(), &waivers).expect("check");
    assert!(report.drift.is_empty(), "{:?}", report.drift);
    assert_eq!(report.waived.len(), 2);
    assert!(report.unused_waivers.is_empty());
}

#[test]
fn a_missing_mise_toml_is_drift_and_scaffold_writes_it() {
    let dir = workspace();
    scaffold(dir.path(), false).expect("scaffold");
    std::fs::remove_file(dir.path().join("mise.toml")).expect("rm");
    let report = check(dir.path(), &no_waivers()).expect("check");
    assert_eq!(report.drift, vec![Drift::new("mise.toml", "missing")]);

    let written = scaffold(dir.path(), false).expect("rescaffold");
    let mise = written
        .iter()
        .find(|w| w.target == "mise.toml")
        .expect("mise.toml target");
    assert_eq!(mise.outcome, Outcome::Created);
    assert!(check(dir.path(), &no_waivers())
        .expect("check")
        .drift
        .is_empty());
}
