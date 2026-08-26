//! Shared cargo-metadata JSON fixtures for this crate's tests.
//!
//! DUP-4 / TASK-1540: the cargo-metadata skeleton used to be open-coded as a
//! `serde_json::json!` literal at 23 call sites across `tests/*` and
//! `ingestor.rs`. Each one restated 15-20 boilerplate fields the test below it
//! never exercised, so a schema-shape change meant editing every copy and a
//! reviewer could not see which fields a given test actually cared about.
//!
//! Two families live here, and they are deliberately **not** the same shape:
//!
//! * **View fixtures** ([`pkg`], [`workspace`]) feed [`crate::Metadata`], which
//!   reads the JSON lazily. They are *minimal by construction*: the builder
//!   emits only what was set on it, so a test asserting that an absent field
//!   falls back (`edition()` → `""`, `license()` → `None`) says so by simply
//!   not setting it. A fixture that helpfully filled in defaults would gut
//!   exactly those tests while leaving them green.
//!
//! * **Ingest fixtures** ([`ingest_metadata`], [`ingest_dep`]) are written to
//!   disk and read back through `DuckDB`'s `read_json_auto`. They are
//!   deliberately *fat*: every nullable string carries an explicit `""`,
//!   because a column that is null in every row infers as INTEGER and the
//!   view's casts then fail. Trimming these to "only what the test exercises"
//!   would break schema inference — the boilerplate is load-bearing, which is
//!   why it lives here exactly once instead of at four call sites.

use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

// ── View fixtures ───────────────────────────────────────────────────────────

/// A package entry, carrying only the fields a test sets on it.
///
/// `id` and `manifest_path` are derived from the name and version so the two
/// places a package id appears — the entry itself and `workspace_members` —
/// cannot drift apart.
pub struct PkgFixture {
    name: String,
    version: String,
    id: Option<String>,
    manifest_path: Option<String>,
    extra: Map<String, Value>,
}

/// Start a package fixture. Emits `name`, `version`, a derived `id` and
/// `manifest_path`, and empty `dependencies` / `targets`; nothing else.
pub fn pkg(name: &str, version: &str) -> PkgFixture {
    PkgFixture {
        name: name.to_owned(),
        version: version.to_owned(),
        id: None,
        manifest_path: None,
        extra: Map::new(),
    }
}

impl PkgFixture {
    /// Override the derived id — for the duplicate-id and registry-package
    /// cases, where the id is the thing under test.
    pub(crate) fn id(mut self, id: &str) -> Self {
        self.id = Some(id.to_owned());
        self
    }

    /// Override the derived `/workspace/<name>/Cargo.toml`.
    pub(crate) fn manifest_path(mut self, path: &str) -> Self {
        self.manifest_path = Some(path.to_owned());
        self
    }

    pub(crate) fn edition(self, edition: &str) -> Self {
        self.field("edition", json!(edition))
    }

    pub(crate) fn deps(self, dependencies: Value) -> Self {
        self.field("dependencies", dependencies)
    }

    pub(crate) fn targets(self, targets: Value) -> Self {
        self.field("targets", targets)
    }

    /// Set any other package field (`license`, `repository`, `description`, …).
    pub(crate) fn field(mut self, key: &str, value: Value) -> Self {
        self.extra.insert(key.to_owned(), value);
        self
    }

    fn resolved_id(&self) -> String {
        self.id.clone().unwrap_or_else(|| {
            format!(
                "{} {} (path+file:///workspace/{})",
                self.name, self.version, self.name
            )
        })
    }

    fn value(self) -> Value {
        let mut map = Map::new();
        map.insert("name".to_owned(), json!(self.name));
        map.insert("version".to_owned(), json!(self.version));
        map.insert("id".to_owned(), json!(self.resolved_id()));
        map.insert(
            "manifest_path".to_owned(),
            json!(self
                .manifest_path
                .clone()
                .unwrap_or_else(|| format!("/workspace/{}/Cargo.toml", self.name))),
        );
        map.insert("dependencies".to_owned(), json!([]));
        map.insert("targets".to_owned(), json!([]));
        map.extend(self.extra);
        Value::Object(map)
    }
}

/// A cargo-metadata document under construction.
pub struct WsFixture {
    root: String,
    build_directory: Option<String>,
    /// `(name, id)` per package, in insertion order; members only.
    member_ids: Vec<(String, String)>,
    default_members: Option<Vec<String>>,
    packages: Vec<Value>,
}

/// Start a workspace rooted at `/workspace` with no packages.
pub fn workspace() -> WsFixture {
    WsFixture {
        root: "/workspace".to_owned(),
        build_directory: None,
        member_ids: Vec::new(),
        default_members: None,
        packages: Vec::new(),
    }
}

impl WsFixture {
    pub(crate) fn root(mut self, root: &str) -> Self {
        self.root = root.to_owned();
        self
    }

    pub(crate) fn build_directory(mut self, dir: &str) -> Self {
        self.build_directory = Some(dir.to_owned());
        self
    }

    /// Add a package that is a workspace member.
    pub(crate) fn member(mut self, package: PkgFixture) -> Self {
        self.member_ids
            .push((package.name.clone(), package.resolved_id()));
        self.packages.push(package.value());
        self
    }

    /// Add a package present in `packages` but absent from `workspace_members`
    /// — a registry dependency, or the duplicate-warning fixtures where
    /// membership is beside the point.
    pub(crate) fn external(mut self, package: PkgFixture) -> Self {
        self.packages.push(package.value());
        self
    }

    /// Restrict `workspace_default_members` to the named members. Defaults to
    /// every member. Panics on a name that was never added, so the list cannot
    /// silently point at nothing.
    pub(crate) fn default_members(mut self, names: &[&str]) -> Self {
        self.default_members = Some(
            names
                .iter()
                .map(|name| {
                    self.member_ids
                        .iter()
                        .find(|(member, _)| member == name)
                        .map_or_else(
                            || panic!("fixture: `{name}` is not a workspace member"),
                            |(_, id)| id.clone(),
                        )
                })
                .collect(),
        );
        self
    }

    pub(crate) fn value(self) -> Value {
        let member_ids: Vec<&str> = self.member_ids.iter().map(|(_, id)| id.as_str()).collect();
        let defaults: Vec<&str> = self.default_members.as_ref().map_or_else(
            || member_ids.clone(),
            |names| names.iter().map(String::as_str).collect(),
        );
        let mut map = Map::new();
        map.insert("workspace_root".to_owned(), json!(self.root));
        map.insert(
            "target_directory".to_owned(),
            json!(format!("{}/target", self.root.trim_end_matches('/'))),
        );
        if let Some(dir) = &self.build_directory {
            map.insert("build_directory".to_owned(), json!(dir));
        }
        map.insert("workspace_members".to_owned(), json!(member_ids));
        map.insert("workspace_default_members".to_owned(), json!(defaults));
        map.insert("packages".to_owned(), json!(self.packages));
        Value::Object(map)
    }

    pub(crate) fn metadata(self) -> crate::Metadata {
        crate::Metadata::from_value(self.value())
    }
}

/// The two-package workspace most accessor tests read from: a member `pkg-a`
/// carrying the optional-field set, plus `serde` as a registry package.
pub fn sample_metadata() -> Value {
    workspace()
        .build_directory("/workspace/target/debug/build")
        .member(
            pkg("pkg-a", "0.1.0")
                .edition("2021")
                .field("license", json!("MIT"))
                .field("repository", json!("https://github.com/example/pkg-a"))
                .field("description", json!("A sample package"))
                .deps(json!([
                    {
                        "name": "serde",
                        "req": "^1.0",
                        "kind": null,
                        "optional": true,
                        "uses_default_features": true,
                        "features": ["derive"]
                    },
                    {
                        "name": "tokio",
                        "req": "^1.0",
                        "kind": "dev",
                        "optional": false,
                        "uses_default_features": false,
                        "features": []
                    },
                    {
                        "name": "cc",
                        "req": "^1.0",
                        "kind": "build",
                        "optional": false,
                        "uses_default_features": true,
                        "features": []
                    }
                ]))
                .targets(json!([
                    {
                        "name": "pkg_a",
                        "kind": ["lib"],
                        "src_path": "/workspace/pkg-a/src/lib.rs"
                    },
                    {
                        "name": "pkg-a",
                        "kind": ["bin"],
                        "src_path": "/workspace/pkg-a/src/main.rs",
                        "required-features": ["default"]
                    }
                ])),
        )
        .external(
            pkg("serde", "1.0.0")
                .id("serde 1.0.0 (registry+https://github.com/rust-lang/crates.io-index)")
                .manifest_path("/cargo/registry/serde-1.0.0/Cargo.toml")
                .edition("2018"),
        )
        .value()
}

pub fn test_pkg_a(metadata: &crate::Metadata) -> crate::Package<'_> {
    metadata.package_by_name("pkg-a").expect("fixture: pkg-a")
}

pub fn test_pkg_serde(metadata: &crate::Metadata) -> crate::Package<'_> {
    metadata.package_by_name("serde").expect("fixture: serde")
}

// ── Ingest fixtures ─────────────────────────────────────────────────────────

const REGISTRY: &str = "registry+https://github.com/rust-lang/crates.io-index";

/// A dependency entry for the ingest path. Every nullable string is an
/// explicit `""` — see the module docs on `DuckDB` schema inference.
pub struct IngestDep {
    name: String,
    req: String,
    source: Value,
    target: String,
}

pub fn ingest_dep(name: &str, req: &str) -> IngestDep {
    IngestDep {
        name: name.to_owned(),
        req: req.to_owned(),
        source: json!(REGISTRY),
        target: String::new(),
    }
}

impl IngestDep {
    /// A path dependency carries `source: null`; TASK-0982 pins that those are
    /// not dropped from the view.
    pub(crate) fn path_source(mut self) -> Self {
        self.source = Value::Null;
        self
    }

    /// The `cfg(...)` a target-conditional dependency was declared under.
    pub(crate) fn target(mut self, target: &str) -> Self {
        self.target = target.to_owned();
        self
    }

    fn value(self) -> Value {
        json!({
            "name": self.name,
            "source": self.source,
            "req": self.req,
            "kind": "normal",
            "optional": false,
            "uses_default_features": true,
            "features": [],
            "target": self.target,
            "rename": "",
            "registry": ""
        })
    }
}

/// A single-package cargo-metadata document for the ingest path.
pub struct IngestMetadata {
    root: String,
    source: Value,
    deps: Vec<IngestDep>,
}

/// Start a `test-crate 0.1.0` document rooted at `/test`.
pub fn ingest_metadata() -> IngestMetadata {
    IngestMetadata {
        root: "/test".to_owned(),
        source: json!(REGISTRY),
        deps: Vec::new(),
    }
}

impl IngestMetadata {
    /// Override `workspace_root`. `target_directory` stays `/test/target`,
    /// which is what makes the multi-row fixture differ in one field only.
    pub(crate) fn root(mut self, root: &str) -> Self {
        self.root = root.to_owned();
        self
    }

    /// Override the package's own `source` (`""` for a workspace package).
    pub(crate) fn source(mut self, source: Value) -> Self {
        self.source = source;
        self
    }

    pub(crate) fn dep(mut self, dep: IngestDep) -> Self {
        self.deps.push(dep);
        self
    }

    pub(crate) fn value(self) -> Value {
        let deps: Vec<Value> = self.deps.into_iter().map(IngestDep::value).collect();
        json!({
            "packages": [{
                "name": "test-crate",
                "version": "0.1.0",
                "id": "test-crate 0.1.0 (path+file:///test)",
                "source": self.source,
                "dependencies": deps,
                "targets": [],
                "features": {},
                "manifest_path": "/test/Cargo.toml",
                "metadata": {},
                "publish": [],
                "authors": [],
                "categories": [],
                "keywords": [],
                "readme": "",
                "repository": "",
                "homepage": "",
                "documentation": "",
                "edition": "2021",
                "links": "",
                "default_run": "",
                "rust_version": "",
                "license": "",
                "license_file": "",
                "description": ""
            }],
            "workspace_members": ["test-crate 0.1.0 (path+file:///test)"],
            "workspace_default_members": ["test-crate 0.1.0 (path+file:///test)"],
            "resolve": {"nodes": [], "root": ""},
            "target_directory": "/test/target",
            "version": 1,
            "workspace_root": self.root,
            "metadata": {}
        })
    }
}

/// Write a fixture to `<dir>/metadata.json`, where the loader looks for it.
pub fn write_metadata_json(dir: &Path, value: &Value) -> PathBuf {
    let json_path = dir.join("metadata.json");
    std::fs::write(&json_path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    json_path
}
