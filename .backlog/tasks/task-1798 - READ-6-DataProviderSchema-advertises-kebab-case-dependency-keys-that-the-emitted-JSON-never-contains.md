---
id: TASK-1798
title: >-
  READ-6: DataProviderSchema advertises kebab-case dependency keys that the
  emitted JSON never contains
status: Triage
assignee: []
created_date: '2026-08-27 11:25'
labels:
  - code-review-rust
  - api-design
dependencies: []
modified_files:
  - extensions-rust/cargo-toml/src/lib.rs
  - extensions-rust/cargo-toml/src/types.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:249-299` (`CargoTomlProvider::schema`) vs `extensions-rust/cargo-toml/src/types.rs:45-51` (`CargoToml::dev_dependencies` / `build_dependencies`).

**What**: the provider's published schema declares

```rust
data_field!("dev-dependencies", "Map<String, DepSpec>", "Development dependencies"),
data_field!("build-dependencies", "Map<String, DepSpec>", "Build dependencies"),
```

but the struct uses `alias`, not `rename`:

```rust
#[serde(default, alias = "dev-dependencies")]
pub dev_dependencies: BTreeMap<String, DepSpec>,
```

`alias` affects **deserialization only**. `DataProvider::provide` (`lib.rs:244-247`) returns `serde_json::to_value(&manifest)`, which serializes those fields under their Rust names — `dev_dependencies` and `build_dependencies`. So any consumer that reads the JSON by the key the schema documents (`Context::cached` / `query_data` with a `dev-dependencies` path) silently gets nothing.

Second, smaller mismatch in the same block: `data_field!("Package.version", "String", ...)` describes the type as `String`, but `Package::version` is an untagged `InheritableField`, so an unresolved manifest serializes it as the object `{"workspace": true}` rather than a string. Same for `edition`, `license`, `description`, `repository`, `authors`.

The gap is invisible to the suite because the only schema test, `provider_schema_has_expected_fields` (`src/tests/provider.rs:96-113`), asserts the schema *lists* `"dev-dependencies"` and never compares it against a serialized manifest. `provider_parses_real_cargo_toml` round-trips JSON back into `CargoToml`, which succeeds precisely because the `alias` covers the read direction.

**Why it matters**: READ-6 — the same concept is spelled two ways on the two sides of a published contract. The schema is this extension's entire documented interface for cross-extension consumers (this crate is the repo's canonical data-source-only extension, per the module docs at `lib.rs:1-56`), so a wrong key there is a wrong integration guide for every extension that follows the pattern. Fix in either direction — `#[serde(rename = "dev-dependencies", alias = "dev_dependencies")]` so the wire format matches Cargo's own spelling and the schema, or correct the schema to the snake_case keys actually emitted — but the two must agree, and a test must hold them together.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every field name in CargoTomlProvider::schema() is a key that actually appears in the JSON returned by DataProvider::provide for a manifest that populates it
- [ ] #2 dev-dependencies / build-dependencies resolve consistently in both directions (serialization and deserialization), whichever spelling is chosen
- [ ] #3 The Package.version / edition / license / description / repository / authors schema entries describe the untagged InheritableField shape, not a bare String
- [ ] #4 A test serializes a manifest exercising all documented sections and asserts every schema field name is present in the resulting serde_json::Value, replacing the current name-list-only assertion in src/tests/provider.rs:96
- [ ] #5 Existing consumers (extensions-rust/about, extensions-rust/create-review-tasks) still round-trip the provider output
<!-- AC:END -->
