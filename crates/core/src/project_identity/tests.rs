use super::*;

#[test]
fn about_card_from_identity_full() {
    let id = ProjectIdentity {
        name: "ops".to_string(),
        version: Some("0.10.0".to_string()),
        description: Some("Task runner".to_string()),
        stack_label: "Rust".to_string(),
        stack_detail: Some("Edition 2021".to_string()),
        license: Some("Apache-2.0".to_string()),
        project_path: "/home/user/ops".to_string(),
        module_count: Some(15),
        module_label: "crates".to_string(),
        loc: Some(21324),
        file_count: Some(96),
        authors: vec!["Alice".to_string()],
        repository: Some("https://github.com/user/ops".to_string()),
        homepage: None,
        msrv: None,
        dependency_count: None,
        coverage_percent: None,
        languages: vec![],
        regions: vec![],
    };
    let card = AboutCard::from_identity(&id);
    assert_eq!(card.description, Some("Task runner".to_string()));
    // project, stack, license, crates, codebase, repository, author
    // (no coverage — hidden when empty + all-fields mode)
    assert_eq!(card.fields.len(), 7);
    assert_eq!(card.fields[0], ("project".to_string(), "ops\nv0.10.0\n/home/user/ops".to_string()));
    assert_eq!(card.fields[1], ("stack".to_string(), "Rust\nEdition 2021".to_string()));
    assert_eq!(card.fields[2], ("license".to_string(), "Apache-2.0".to_string()));
    assert_eq!(card.fields[3], ("crates".to_string(), "15".to_string()));
    assert_eq!(card.fields[4], ("codebase".to_string(), "21,324 loc\n96 files".to_string()));
}

#[test]
fn about_card_from_identity_minimal() {
    let id = ProjectIdentity {
        name: "myproject".to_string(),
        version: None,
        description: None,
        stack_label: "Generic".to_string(),
        stack_detail: None,
        license: None,
        project_path: "/tmp/myproject".to_string(),
        module_count: None,
        module_label: "modules".to_string(),
        loc: None,
        file_count: None,
        authors: vec![],
        repository: None,
        homepage: None,
        msrv: None,
        dependency_count: None,
        coverage_percent: None,
        languages: vec![],
        regions: vec![],
    };
    let card = AboutCard::from_identity(&id);
    assert!(card.description.is_none());
    // project, stack — no license, no coverage (empty).
    assert_eq!(card.fields.len(), 2);
    assert_eq!(card.fields[0], ("project".to_string(), "myproject\n/tmp/myproject".to_string()));
    assert_eq!(card.fields[1], ("stack".to_string(), "Generic".to_string()));
}

/// Without a region split (every non-Rust stack — `rust-loc` only
/// registers on Rust) the codebase field is bare totals; the language mix
/// no longer renders even when `languages` is populated.
#[test]
fn about_card_codebase_without_regions_renders_bare_totals() {
    let id = ProjectIdentity {
        name: "openbao".to_string(),
        version: None,
        description: None,
        stack_label: "Go".to_string(),
        stack_detail: None,
        license: None,
        project_path: "/p".to_string(),
        module_count: Some(7),
        module_label: "modules".to_string(),
        loc: Some(555_910),
        file_count: Some(4_929),
        authors: vec![],
        repository: None,
        homepage: None,
        msrv: None,
        dependency_count: None,
        coverage_percent: None,
        languages: vec![LanguageStat::new("Go", 432_000, 3_800, 77.7, 77.1)],
        regions: vec![],
    };
    let card = AboutCard::from_identity(&id);
    let codebase =
        card.fields.iter().find(|(k, _)| k == "codebase").expect("codebase field").1.clone();
    assert_eq!(codebase, "555,910 loc\n4,929 files");
}

/// With a region split, the field renders that split — the header total is
/// the summed region code, not `loc`, so the two can never disagree —
/// followed by the distinct file total.
#[test]
fn about_card_codebase_with_regions() {
    let id = ProjectIdentity {
        name: "ops".to_string(),
        version: None,
        description: None,
        stack_label: "Rust".to_string(),
        stack_detail: None,
        license: None,
        project_path: "/p".to_string(),
        module_count: Some(29),
        module_label: "crates".to_string(),
        loc: Some(999_999),
        file_count: Some(1_324),
        authors: vec![],
        repository: None,
        homepage: None,
        msrv: None,
        dependency_count: None,
        coverage_percent: None,
        languages: vec![],
        regions: vec![
            RegionStat::new("test", 1_074, 191_430),
            RegionStat::new("main", 936, 268_548),
            RegionStat::new("example", 66, 7_932),
        ],
    };
    let card = AboutCard::from_identity(&id);
    let codebase =
        card.fields.iter().find(|(k, _)| k == "codebase").expect("codebase field").1.clone();
    let lines: Vec<&str> = codebase.split('\n').collect();
    assert_eq!(lines[0], "467,910 loc", "got: {codebase}");
    // Canonical order regardless of input order: production, test, example.
    assert!(lines[1].contains("production"), "got: {codebase}");
    assert!(lines[1].contains("268,548 (57.4%)"), "got: {codebase}");
    assert!(lines[1].contains("936 files"), "got: {codebase}");
    assert!(lines[2].contains("test"), "got: {codebase}");
    assert!(lines[2].contains("191,430 (40.9%)"), "got: {codebase}");
    assert!(lines[3].contains("example"), "got: {codebase}");
    assert!(lines[3].contains("7,932 (1.7%)"), "got: {codebase}");
    assert_eq!(lines[4], "1,324 files", "got: {codebase}");
}

#[test]
fn about_card_coverage_hidden_when_empty() {
    let id = ProjectIdentity {
        name: "x".into(),
        stack_label: "Rust".into(),
        project_path: "/p".into(),
        module_label: "crates".into(),
        coverage_percent: None,
        ..Default::default()
    };
    let card = AboutCard::from_identity(&id);
    assert!(card.fields.iter().all(|(k, _)| k != "coverage"));
}

#[test]
fn about_card_coverage_shown_when_explicitly_selected() {
    let id = ProjectIdentity {
        name: "x".into(),
        stack_label: "Rust".into(),
        project_path: "/p".into(),
        module_label: "crates".into(),
        coverage_percent: None,
        ..Default::default()
    };
    let card = AboutCard::from_identity_filtered(
        &id,
        Some(&["project".to_string(), "coverage".to_string()]),
    );
    let cov = card.fields.iter().find(|(k, _)| k == "coverage").expect("coverage");
    assert_eq!(cov.1, "not collected");
}

fn sample_identity() -> ProjectIdentity {
    ProjectIdentity {
        name: "ops".to_string(),
        version: Some("0.10.0".to_string()),
        description: Some("Task runner".to_string()),
        stack_label: "Rust".to_string(),
        stack_detail: Some("Edition 2021".to_string()),
        license: Some("Apache-2.0".to_string()),
        project_path: "/home/user/ops".to_string(),
        module_count: Some(15),
        module_label: "crates".to_string(),
        loc: Some(21324),
        file_count: Some(96),
        authors: vec!["Alice".to_string()],
        repository: Some("https://github.com/user/ops".to_string()),
        homepage: None,
        msrv: None,
        dependency_count: None,
        coverage_percent: None,
        languages: vec![],
        regions: vec![],
    }
}

#[test]
fn render_non_tty_contains_identity_fields() {
    let card = AboutCard::from_identity(&sample_identity());
    let output = card.render(false);
    assert!(output.contains("ops"), "got: {output}");
    assert!(output.contains("v0.10.0"), "got: {output}");
    assert!(output.contains("Rust"), "got: {output}");
    assert!(output.contains("Apache-2.0"), "got: {output}");
}

#[test]
fn render_non_tty_contains_fields() {
    let card = AboutCard::from_identity(&sample_identity());
    let output = card.render(false);
    assert!(output.contains("/home/user/ops"), "got: {output}");
    assert!(output.contains("21,324 loc"), "got: {output}");
    assert!(output.contains("96 files"), "got: {output}");
    assert!(output.contains("Alice"), "got: {output}");
}

#[test]
fn render_non_tty_contains_description() {
    let card = AboutCard::from_identity(&sample_identity());
    let output = card.render(false);
    assert!(output.contains("Task runner"), "got: {output}");
}

#[test]
fn render_tty_contains_ansi_escapes() {
    let card = AboutCard::from_identity(&sample_identity());
    let output = card.render(true);
    // ANSI escape codes start with \x1b[
    assert!(output.contains("\x1b["), "TTY output should contain ANSI escapes: {output}");
}

#[test]
fn render_non_tty_no_ansi_escapes() {
    let card = AboutCard::from_identity(&sample_identity());
    let output = card.render(false);
    assert!(!output.contains("\x1b["), "non-TTY output should not contain ANSI escapes: {output}");
}

#[test]
fn render_minimal_card_no_description() {
    let id = ProjectIdentity {
        name: "bare".to_string(),
        version: None,
        description: None,
        stack_label: "Generic".to_string(),
        stack_detail: None,
        license: None,
        project_path: "/tmp".to_string(),
        module_count: None,
        module_label: "modules".to_string(),
        loc: None,
        file_count: None,
        authors: vec![],
        repository: None,
        homepage: None,
        msrv: None,
        dependency_count: None,
        coverage_percent: None,
        languages: vec![],
        regions: vec![],
    };
    let card = AboutCard::from_identity(&id);
    let output = card.render(false);
    assert!(output.contains("bare"), "got: {output}");
    assert!(output.contains("/tmp"), "got: {output}");
    // stack, project — project spans 2 lines (name + path). 3 output lines.
    assert_eq!(output.matches('\n').count(), 2);
}

#[test]
fn file_count_singular() {
    let mut id = sample_identity();
    id.file_count = Some(1);
    let card = AboutCard::from_identity(&id);
    let output = card.render(false);
    assert!(output.contains("1 file"), "got: {output}");
    assert!(!output.contains("1 files"), "should be singular: {output}");
}

#[test]
fn dependency_count_usize_max_does_not_panic() {
    // SEC-15 / TASK-0339: a usize > i64::MAX must not narrow to a negative
    // i64. We saturate to i64::MAX and render a sensible string instead.
    let mut id = sample_identity();
    id.dependency_count = Some(usize::MAX);
    let card = AboutCard::from_identity(&id);
    let dep_field =
        card.fields.iter().find(|(k, _)| k == "dependencies").expect("dependencies field present");
    assert!(
        dep_field.1.starts_with("9,223,372,036,854,775,807"),
        "should saturate to i64::MAX, got: {}",
        dep_field.1
    );
    assert!(dep_field.1.ends_with("dependencies"), "expected plural label: {}", dep_field.1);
}
