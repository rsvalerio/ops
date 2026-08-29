//! Shared cargo-metadata JSON fixtures for this crate's tests.
//!
//! DUP-4 / TASK-1540: the cargo-metadata skeleton used to be open-coded as a
//! `serde_json::json!` literal at 23 call sites across `tests/*` and
//! `ingestor.rs`. Each one restated 15-20 boilerplate fields the test below it
//! never exercised, so a schema-shape change meant editing every copy and a
//! reviewer could not see which fields a given test actually cared about.
//!
//! ARCH-9 / TASK-1898: the *view* fixtures (`pkg`, `workspace`,
//! `sample_metadata`) went with the unconsumed typed accessor layer they
//! existed to feed. What remains are the **ingest fixtures**
//! ([`ingest_metadata`], [`ingest_dep`]), which are written to disk and read
//! back through `DuckDB`'s `read_json_auto`. They are deliberately *fat*:
//! every nullable string carries an explicit `""`, because a column that is
//! null in every row infers as INTEGER and the view's casts then fail.
//! Trimming these to "only what the test exercises" would break schema
//! inference — the boilerplate is load-bearing, which is why it lives here
//! exactly once instead of at four call sites.

use ops_duckdb::IngestDir;
use serde_json::{json, Value};
use std::path::PathBuf;

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

/// Write a fixture to `metadata.json` inside the ingest directory, where the
/// loader looks for it.
///
/// SEC-25 / TASK-2054: staged through the verified [`IngestDir`] anchor, the
/// same way `MetadataIngestor::collect` stages it in production, and returns
/// the entry path so callers can assert on cleanup.
pub fn write_metadata_json(dir: &IngestDir, value: &Value) -> PathBuf {
    dir.write_atomic("metadata.json", &serde_json::to_vec_pretty(value).unwrap())
        .unwrap();
    dir.entry_path("metadata.json")
}

/// Open a verified ingest anchor inside `tmp`, mirroring what
/// `provide_via_ingestor` builds before it calls an ingestor.
pub fn ingest_anchor(tmp: &tempfile::TempDir) -> IngestDir {
    IngestDir::open(&tmp.path().join("data.duckdb.ingest")).expect("open ingest dir")
}
