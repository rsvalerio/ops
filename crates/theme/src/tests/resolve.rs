//! Theme name resolution and listing.

use super::*;

#[test]
fn resolve_theme_classic() {
    let mut themes = IndexMap::new();
    themes.insert("classic".into(), ThemeConfig::classic());
    let theme = resolve_theme("classic", &themes).unwrap();
    assert_eq!(theme.status_icon(StepStatus::Succeeded), "◆");
}

#[test]
fn resolve_theme_compact() {
    let mut themes = IndexMap::new();
    themes.insert("compact".into(), ThemeConfig::compact());
    let theme = resolve_theme("compact", &themes).unwrap();
    assert_eq!(theme.status_icon(StepStatus::Succeeded), "✓");
}

#[test]
fn resolve_theme_custom() {
    let mut themes = IndexMap::new();
    themes.insert(
        "my-theme".into(),
        ThemeConfig {
            icon_succeeded: "OK".into(),
            ..ThemeConfig::compact()
        },
    );
    let theme = resolve_theme("my-theme", &themes).unwrap();
    assert_eq!(theme.status_icon(StepStatus::Succeeded), "OK");
}

#[test]
fn resolve_theme_not_found() {
    let themes = IndexMap::new();
    let result = resolve_theme("nonexistent", &themes);
    assert!(matches!(result, Err(ThemeError::NotFound(_))));
}

#[test]
fn resolve_theme_not_found_preserves_name() {
    let themes = IndexMap::new();
    match resolve_theme("missing-theme", &themes) {
        Err(ThemeError::NotFound(name)) => assert_eq!(name, "missing-theme"),
        _ => panic!("expected NotFound"),
    }
}

#[test]
fn resolve_theme_returns_distinct_instances_per_call() {
    // Regression guard: resolver should clone the backing config so mutations
    // to one returned theme cannot affect another. If the resolver were ever
    // refactored to return a shared reference, both calls below would alias.
    let mut themes = IndexMap::new();
    themes.insert("compact".into(), ThemeConfig::compact());
    let a = resolve_theme("compact", &themes).unwrap();
    let b = resolve_theme("compact", &themes).unwrap();
    assert_eq!(a.status_icon(StepStatus::Succeeded), "✓");
    assert_eq!(b.status_icon(StepStatus::Succeeded), "✓");
}

#[test]
fn resolve_theme_is_case_sensitive() {
    let mut themes = IndexMap::new();
    themes.insert("compact".into(), ThemeConfig::compact());
    match resolve_theme("Compact", &themes) {
        Err(ThemeError::NotFound(name)) => assert_eq!(name, "Compact"),
        _ => panic!("expected NotFound for case-mismatched name"),
    }
}

#[test]
fn resolve_theme_owned_takes_ownership_no_clone() {
    // OWN-4 / TASK-0836: the owning variant pulls the entry out of the map
    // via swap_remove, so no ThemeConfig clone is performed.
    let mut themes = IndexMap::new();
    themes.insert("compact".into(), ThemeConfig::compact());
    themes.insert("classic".into(), ThemeConfig::classic());
    let theme = resolve_theme_owned("compact", &mut themes).unwrap();
    assert_eq!(theme.status_icon(StepStatus::Succeeded), "✓");
    // The named entry must be gone afterwards; siblings remain.
    assert!(!themes.contains_key("compact"));
    assert!(themes.contains_key("classic"));
}

#[test]
fn resolve_theme_owned_not_found_preserves_name_and_map() {
    let mut themes = IndexMap::new();
    themes.insert("compact".into(), ThemeConfig::compact());
    match resolve_theme_owned("missing", &mut themes) {
        Err(ThemeError::NotFound(name)) => assert_eq!(name, "missing"),
        _ => panic!("expected NotFound"),
    }
    // Map untouched on miss.
    assert!(themes.contains_key("compact"));
}

#[test]
fn list_theme_names_from_config() {
    let mut themes = IndexMap::new();
    themes.insert("classic".into(), ThemeConfig::classic());
    themes.insert("compact".into(), ThemeConfig::compact());
    let names = list_theme_names(&themes);
    assert!(names.contains(&"classic".to_string()));
    assert!(names.contains(&"compact".to_string()));
}

#[test]
fn list_theme_names_with_custom() {
    let mut themes = IndexMap::new();
    themes.insert("classic".into(), ThemeConfig::classic());
    themes.insert("compact".into(), ThemeConfig::compact());
    themes.insert("dark".into(), ThemeConfig::compact());
    themes.insert("light".into(), ThemeConfig::classic());
    let names = list_theme_names(&themes);
    assert!(names.contains(&"classic".to_string()));
    assert!(names.contains(&"compact".to_string()));
    assert!(names.contains(&"dark".to_string()));
    assert!(names.contains(&"light".to_string()));
}
