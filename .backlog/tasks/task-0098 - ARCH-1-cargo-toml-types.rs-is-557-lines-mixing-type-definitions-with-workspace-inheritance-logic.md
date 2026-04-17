---
id: TASK-0098
title: >-
  ARCH-1: cargo-toml/types.rs is 557 lines mixing type definitions with
  workspace inheritance logic
status: To Do
assignee: []
created_date: '2026-04-17 11:56'
updated_date: '2026-04-17 12:07'
labels:
  - rust-code-review
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/types.rs` (557 lines)

**What**: A file named `types.rs` holds not only the strongly-typed Cargo.toml DTOs (`CargoToml`, `Package`, `Workspace`, `InheritableField<T>`, `InheritableString`, `InheritableVec`, `ReadmeSpec`, `PublishSpec`, `DepSpec`, etc.), but also all of the workspace inheritance resolution logic (`CargoToml::resolve_inheritance`, `CargoToml::resolve_package_inheritance`, free functions `resolve_string_field`, `resolve_deps_inheritance`, and the `resolve_dep_from_workspace` helpers). Two unrelated concerns — shape (pure `serde` types) and behavior (cross-section resolution) — share the file.

**Why it matters**: Violates ARCH-1 ("modules by concern") and the ARCH-8 heuristic "`types.rs` is for pure types". A reader looking for the `InheritableField` definition has to scroll past ~200 lines of resolution logic; a reviewer touching inheritance semantics has to skim past every field to find the impls. Splitting makes the public type surface immediately readable and keeps inheritance logic (the tested algorithm) in its own tested file.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A new inheritance module (e.g. inheritance.rs or resolve.rs) holds resolve_package_inheritance, resolve_string_field, resolve_deps_inheritance, resolve_dep_from_workspace, and any helpers that cross sections
- [ ] #2 types.rs keeps only the serde DTOs plus impl methods that are pure accessors/predicates on a single struct (e.g. CargoToml::is_workspace, package_name)
- [ ] #3 Existing inheritance tests live next to (or move to) the new module and still pass unchanged
- [ ] #4 cargo clippy --all-targets -- -D warnings and cargo test -p ops-cargo-toml both pass
running 66 tests
test tests::cargo_toml_edge_case_tests::inheritable_field_value_and_inherited ... ok
test tests::cargo_toml_edge_case_tests::dep_spec_simple_accessors ... ok
test tests::cargo_toml_edge_case_tests::inheritable_field_default ... ok
test tests::cargo_toml_edge_case_tests::parse_with_target_specific_dependencies ... ok
test tests::cargo_toml_edge_case_tests::parse_with_empty_package_name ... ok
test tests::cargo_toml_edge_case_tests::parse_with_missing_required_version ... ok
test tests::cargo_toml_edge_case_tests::parse_with_profile_settings ... ok
test tests::cargo_toml_edge_case_tests::publish_spec_true ... ok
test tests::cargo_toml_edge_case_tests::parse_with_lib_and_multiple_bins ... ok
test tests::cargo_toml_edge_case_tests::parse_minimal_valid ... ok
test tests::cargo_toml_edge_case_tests::publish_spec_empty_registries ... ok
test tests::cargo_toml_edge_case_tests::parse_with_missing_required_name ... ok
test tests::cargo_toml_edge_case_tests::readme_spec_bool_variant ... ok
test tests::cargo_toml_edge_case_tests::dep_spec_git_with_branch_tag_rev ... ok
test tests::cargo_toml_edge_case_tests::dep_spec_detailed_default_features_false ... ok
test tests::cargo_toml_edge_case_tests::merge_features_deduplicates ... ok
test tests::cargo_toml_edge_case_tests::readme_spec_table_variant ... ok
test tests::cargo_toml_edge_case_tests::readme_spec_true_variant ... ok
test tests::cargo_toml_edge_case_tests::resolve_inheritance_no_workspace ... ok
test tests::cargo_toml_edge_case_tests::resolve_deeply_nested_workspace_inheritance ... ok
test tests::cargo_toml_edge_case_tests::resolve_detailed_ws_dep_optional_or_logic ... ok
test tests::cargo_toml_edge_case_tests::resolve_detailed_ws_dep_propagates_git_fields ... ok
test tests::cargo_toml_edge_case_tests::resolve_inheritance_dev_and_build_deps ... ok
test tests::cargo_toml_edge_case_tests::resolve_inheritance_with_many_deps ... ok
test tests::cargo_toml_edge_case_tests::resolve_package_inheritance_authors ... ok
test tests::cargo_toml_edge_case_tests::resolve_package_inheritance_no_package ... ok
test tests::cargo_toml_edge_case_tests::resolve_package_inheritance_missing_ws_value_stays_inherited ... ok
test tests::cargo_toml_edge_case_tests::resolve_package_inheritance_no_workspace ... ok
test tests::cargo_toml_edge_case_tests::resolve_package_inheritance_all_string_fields ... ok
test tests::cargo_toml_edge_case_tests::resolve_package_inheritance_no_workspace_package ... ok
test tests::cargo_toml_edge_case_tests::resolve_simple_ws_dep_with_local_optional_and_features ... ok
test tests::cargo_toml_edge_case_tests::resolve_package_inheritance_version_and_edition ... ok
test tests::cargo_toml_edge_case_tests::workspace_exclude_and_default_members ... ok
test tests::cargo_toml_edge_case_tests::workspace_members_accessor ... ok
test tests::cargo_toml_edge_case_tests::resolve_workspace_inheritance_default_features_false ... ok
test tests::cargo_toml_edge_case_tests::workspace_members_none_without_workspace ... ok
test tests::cargo_toml_edge_case_tests::resolve_workspace_inheritance_with_optional_override ... ok
test tests::extension_tests::extension_name ... ok
test tests::extension_tests::extension_registers_data_provider ... ok
test tests::provider_tests::provider_schema_has_expected_fields ... ok
test tests::types_tests::dep_spec_package_rename ... ok
test tests::types_tests::parse_detailed_dependencies ... ok
test tests::types_tests::parse_dev_and_build_dependencies ... ok
test tests::types_tests::parse_features ... ok
test tests::provider_tests::provider_parses_real_cargo_toml ... ok
test tests::extension_tests::extension_with_root_propagates_to_provider ... ok
test tests::types_tests::parse_simple_dependencies ... ok
test tests::provider_tests::provider_missing_cargo_toml ... ok
test tests::types_tests::parse_package_with_all_fields ... ok
test tests::provider_tests::provider_resolve_root_auto_discover_fails_without_cargo_toml ... ok
test tests::find_root_tests::find_root_not_found ... ok
test tests::types_tests::parse_virtual_workspace ... ok
test tests::types_tests::parse_simple_package ... ok
test tests::types_tests::parse_workspace_with_root_package ... ok
test tests::types_tests::parse_workspace_dependencies ... ok
test tests::types_tests::publish_spec_variants ... ok
test tests::types_tests::readme_spec_variants ... ok
test tests::provider_tests::provider_unreadable_file_returns_error ... ok
test tests::types_tests::resolve_workspace_inheritance_simple ... ok
test tests::find_root_tests::find_root_in_current_dir ... ok
test tests::types_tests::resolve_inheritance_missing_workspace_dep ... ok
test tests::provider_tests::provider_resolves_inheritance_in_output ... ok
test tests::types_tests::resolve_workspace_inheritance_with_local_features ... ok
test tests::provider_tests::provider_invalid_toml ... ok
test tests::provider_tests::provider_resolve_root_auto_discovers ... ok
test tests::find_root_tests::find_root_in_parent ... ok

test result: ok. 66 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

running 3 tests
test extensions-rust/cargo-toml/src/lib.rs - (line 26) ... ignored
test extensions-rust/cargo-toml/src/lib.rs - CargoTomlExtension (line 88) ... ignored
test extensions-rust/cargo-toml/src/types.rs - types::CargoToml (line 17) ... ignored

test result: ok. 0 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 0.00s both pass
<!-- AC:END -->
