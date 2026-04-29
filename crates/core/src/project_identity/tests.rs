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
    };
    let card = AboutCard::from_identity(&id);
    assert_eq!(card.description, Some("Task runner".to_string()));
    // project, stack, license, crates, codebase, repository, author
    // (no coverage — hidden when empty + all-fields mode)
    assert_eq!(card.fields.len(), 7);
    assert_eq!(
        card.fields[0],
        (
            "project".to_string(),
            "ops\nv0.10.0\n/home/user/ops".to_string()
        )
    );
    assert_eq!(
        card.fields[1],
        ("stack".to_string(), "Rust\nEdition 2021".to_string())
    );
    assert_eq!(
        card.fields[2],
        ("license".to_string(), "Apache-2.0".to_string())
    );
    assert_eq!(card.fields[3], ("crates".to_string(), "15".to_string()));
    assert_eq!(
        card.fields[4],
        ("codebase".to_string(), "21,324 loc\n96 files".to_string())
    );
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
    };
    let card = AboutCard::from_identity(&id);
    assert!(card.description.is_none());
    // project, stack — no license, no coverage (empty).
    assert_eq!(card.fields.len(), 2);
    assert_eq!(
        card.fields[0],
        (
            "project".to_string(),
            "myproject\n/tmp/myproject".to_string()
        )
    );
    assert_eq!(card.fields[1], ("stack".to_string(), "Generic".to_string()));
}

#[test]
fn about_card_codebase_with_languages() {
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
        languages: vec![
            LanguageStat {
                name: "Go".into(),
                loc: 432_000,
                files: 3_800,
                loc_pct: 77.7,
                files_pct: 77.1,
            },
            LanguageStat {
                name: "JavaScript".into(),
                loc: 64_000,
                files: 600,
                loc_pct: 11.5,
                files_pct: 12.2,
            },
            LanguageStat {
                name: "Handlebars".into(),
                loc: 21_600,
                files: 300,
                loc_pct: 3.9,
                files_pct: 6.1,
            },
            LanguageStat {
                name: "YAML".into(),
                loc: 14_500,
                files: 150,
                loc_pct: 2.6,
                files_pct: 3.0,
            },
            LanguageStat {
                name: "SVG".into(),
                loc: 6_700,
                files: 79,
                loc_pct: 1.2,
                files_pct: 1.6,
            },
        ],
    };
    let card = AboutCard::from_identity(&id);
    let codebase = card
        .fields
        .iter()
        .find(|(k, _)| k == "codebase")
        .expect("codebase field")
        .1
        .clone();
    // Two-block layout: total LOC + top-3 breakdown, then files + top-3 breakdown.
    assert!(codebase.starts_with("555,910 loc\n"), "got: {codebase}");
    assert!(codebase.contains("Go  "), "got: {codebase}");
    assert!(codebase.contains("(77.7%)"), "got: {codebase}");
    assert!(codebase.contains("JS  "), "got: {codebase}");
    assert!(codebase.contains("HBS "), "got: {codebase}");
    assert!(codebase.contains("(+2 more)"), "got: {codebase}");
    assert!(codebase.contains("4,929 files"), "got: {codebase}");
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
    let cov = card
        .fields
        .iter()
        .find(|(k, _)| k == "coverage")
        .expect("coverage");
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
    assert!(
        output.contains("\x1b["),
        "TTY output should contain ANSI escapes: {output}"
    );
}

#[test]
fn render_non_tty_no_ansi_escapes() {
    let card = AboutCard::from_identity(&sample_identity());
    let output = card.render(false);
    assert!(
        !output.contains("\x1b["),
        "non-TTY output should not contain ANSI escapes: {output}"
    );
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
    let dep_field = card
        .fields
        .iter()
        .find(|(k, _)| k == "dependencies")
        .expect("dependencies field present");
    assert!(
        dep_field.1.starts_with("9,223,372,036,854,775,807"),
        "should saturate to i64::MAX, got: {}",
        dep_field.1
    );
    assert!(
        dep_field.1.ends_with("dependencies"),
        "expected plural label: {}",
        dep_field.1
    );
}
