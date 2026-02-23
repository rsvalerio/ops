# Extensions

Extensions extend cargo-ops with custom commands and data providers. Built-in extensions are available automatically; custom extensions can be added by implementing the `Extension` trait.

---

## `cargo ops about`

Display concise project metadata with ASCII art branding (TTY only).

**Usage:**

```bash
cargo ops about
```

**Example output (TTY):**

```
   ____          __  __      _     _
  / ___|   ___  |  \/  | ___| | __| |___
 | |      / _ \ | |\/| |/ _ \ |/ _` / __|
 | |___  |  __/ | |  | |  __/ | (_| \__ \
  \____|  \___| |_|  |_|\___|_|\__,_|___/

  cargo-ops v0.1.0
  Opinionated Rust development CLI
  Edition 2021 • MIT OR Apache-2.0

  Workspace: ~/projects/cargo-ops
  Members:   cargo-ops

  Targets: 1 bin, 1 test
  Deps:    11 runtime, 5 dev
```

**Non-TTY output:** Same information without ASCII art, suitable for scripts and CI.

**Requirements:**

- Must run from a directory containing a valid `Cargo.toml`
- Requires `cargo` to be available in PATH

---

## Built-in Extensions

### Metadata Extension

**Purpose:** Provides workspace information by running `cargo metadata --format-version 1`.

#### For Users

**What it does:**

The metadata extension runs `cargo metadata` once per session and caches the result. This data is available to any command or extension that needs information about the current Cargo workspace.

**Requirements:**

- Must run from a directory containing a valid `Cargo.toml`
- Requires `cargo` to be available in PATH

**Data provided:**

The extension returns the full JSON output from `cargo metadata --format-version 1`. Key fields include:

| Field | Description |
|-------|-------------|
| `packages` | Array of all packages in the workspace |
| `workspace_root` | Absolute path to the workspace root |
| `target_directory` | Path to the target directory |
| `resolve` | Dependency graph with all crate IDs |
| `workspace_members` | Package IDs of workspace members |

**Limitations:**

- No timeout is applied. Network issues or slow registries can cause indefinite hangs. Press `Ctrl+C` to interrupt.
- Fails with an error if run outside a Cargo project directory.

#### For Developers

**Using the typed Metadata wrapper:**

The `Metadata` struct provides convenient accessor methods for common operations:

```rust
use crate::extensions::{Metadata, Package};
use crate::extension::{Context, DataRegistry};

fn my_command(ctx: &mut Context, registry: &DataRegistry) -> Result<(), anyhow::Error> {
    let metadata = Metadata::from_context(ctx, registry)?;
    
    // Workspace info
    println!("Workspace root: {}", metadata.workspace_root());
    println!("Target dir: {}", metadata.target_directory());
    
    // Iterate workspace members
    for pkg in metadata.members() {
        println!("Member: {} v{}", pkg.name(), pkg.version());
    }
    
    // Find a specific package
    if let Some(serde) = metadata.package_by_name("serde") {
        println!("Serde version: {}", serde.version());
    }
    
    Ok(())
}
```

**Package accessors:**

```rust
let pkg = metadata.package_by_name("my-crate").unwrap();

// Basic info
pkg.name();
pkg.version();
pkg.id();
pkg.edition();
pkg.manifest_path();
pkg.license();      // Option<&str>
pkg.repository();   // Option<&str>
pkg.description();  // Option<&str>

// Membership checks
pkg.is_member();           // true if workspace member
pkg.is_default_member();   // true if default member

// Dependencies by kind
for dep in pkg.dependencies() {
    println!("Normal dep: {} {}", dep.name(), dep.version_req());
}

for dep in pkg.dev_dependencies() {
    println!("Dev dep: {}", dep.name());
}

for dep in pkg.build_dependencies() {
    println!("Build dep: {}", dep.name());
}

// All dependencies
for dep in pkg.all_dependencies() {
    println!("{}: {:?} optional={}", dep.name(), dep.kind(), dep.is_optional());
}

// Targets
if let Some(lib) = pkg.lib_target() {
    println!("Lib: {}", lib.src_path());
}

for bin in pkg.bin_targets() {
    println!("Binary: {} at {}", bin.name(), bin.src_path());
}
```

**Dependency accessors:**

```rust
let dep = pkg.dependencies().next().unwrap();

dep.name();
dep.version_req();              // "^1.0"
dep.kind();                     // DependencyKind::Normal | Dev | Build
dep.is_optional();              // bool
dep.uses_default_features();    // bool
dep.features();                 // impl Iterator<Item = &str>
dep.rename();                   // Option<&str>
dep.target();                   // Option<&str> - e.g. "wasm32-unknown-unknown"
dep.source();                   // Option<&str> - registry or path
```

**Target accessors:**

```rust
let target = pkg.lib_target().unwrap();

target.name();
target.src_path();
target.kinds();                  // impl Iterator<Item = &str> - ["lib"], ["bin"], etc.
target.is_lib();                 // bool
target.is_bin();                 // bool
target.is_test();                // bool
target.is_example();             // bool
target.is_bench();               // bool
target.required_features();      // impl Iterator<Item = &str>
target.edition();                // Option<&str>
target.doc_path();               // Option<&str>
```

**Metadata methods summary:**

| Method | Return Type | Description |
|--------|-------------|-------------|
| `workspace_root()` | `&str` | Absolute path to workspace root |
| `target_directory()` | `&str` | Absolute path to target directory |
| `build_directory()` | `Option<&str>` | Build directory if present |
| `packages()` | `impl Iterator<Item = Package>` | All packages in dependency graph |
| `members()` | `impl Iterator<Item = Package>` | Workspace member packages only |
| `default_members()` | `impl Iterator<Item = Package>` | Default workspace members |
| `package_by_name(name)` | `Option<Package>` | Find package by name |
| `package_by_id(id)` | `Option<Package>` | Find package by ID string |

**Raw JSON access (legacy):**

For fields not covered by the typed accessors, you can still access the raw JSON:

```rust
use crate::extension::{Context, DataRegistry};

fn my_command(ctx: &mut Context, registry: &DataRegistry) -> Result<(), anyhow::Error> {
    let raw_json = ctx.get_or_provide("metadata", registry)?;
    
    // Access any field directly
    if let Some(resolve) = raw_json.get("resolve") {
        // ... work with resolve graph
    }
    
    Ok(())
}
```

**Caching behavior:**

The first call to `ctx.get_or_provide("metadata", &registry)` or `Metadata::from_context()` executes `cargo metadata`. Subsequent calls return the cached value without re-running the command.

**JSON schema reference:**

The full schema is defined by Cargo. Key structures:

```json
{
  "packages": [
    {
      "name": "my-crate",
      "version": "0.1.0",
      "id": "my-crate 0.1.0 (path+file:///path/to/crate)",
      "license": "MIT",
      "dependencies": [
        {
          "name": "serde",
          "req": "^1.0",
          "kind": null,
          "target": null
        }
      ],
      "targets": [
        {
          "kind": ["lib"],
          "name": "my_crate",
          "src_path": "/path/to/src/lib.rs"
        }
      ],
      "manifest_path": "/path/to/Cargo.toml",
      "edition": "2021"
    }
  ],
  "workspace_root": "/path/to/workspace",
  "target_directory": "/path/to/workspace/target",
  "resolve": {
    "nodes": [...],
    "root": null
  },
  "workspace_members": ["my-crate 0.1.0 (path+file:///...)"]
}
```

For the complete schema, run:

```bash
cargo metadata --format-version 1 | jq
```

**Error handling:**

```rust
match ctx.get_or_provide("metadata", &registry) {
    Ok(data) => { /* use data */ },
    Err(e) => {
        // Possible causes:
        // - No Cargo.toml in working directory
        // - cargo binary not found
        // - Invalid Cargo.toml
        // - Network/registry issues
        eprintln!("Failed to get metadata: {}", e);
    }
}
```

---

## Creating Custom Extensions

Extensions implement the `Extension` trait and can register:
1. **Commands** — executable steps that appear in the run plan
2. **Data providers** — sources of cached data available to other extensions

### DataProvider Trait

Data providers supply on-demand data that is cached per session.

```rust
use crate::extension::{Context, DataProvider};

pub struct MyProvider;

impl DataProvider for MyProvider {
    fn name(&self) -> &'static str {
        "my_data"
    }

    fn provide(&self, ctx: &Context) -> Result<serde_json::Value, anyhow::Error> {
        // Compute or fetch data
        let data = serde_json::json!({
            "working_dir": ctx.working_directory.to_string_lossy(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        Ok(data)
    }
}
```

**Registration:**

```rust
impl Extension for MyExtension {
    fn name(&self) -> &'static str {
        "my_extension"
    }

    fn register_commands(&self, _registry: &mut CommandRegistry) {
        // No commands for this extension
    }

    fn register_data_providers(&self, registry: &mut DataRegistry) {
        registry.register("my_data", Box::new(MyProvider));
    }
}
```

**Usage:**

```rust
let data = ctx.get_or_provide("my_data", &registry)?;
```

### Extension Trait

The `Extension` trait defines how extensions integrate with cargo-ops.

```rust
pub trait Extension: Send + Sync {
    /// Unique identifier for the extension.
    fn name(&self) -> &'static str;

    /// Register commands that can be executed.
    fn register_commands(&self, registry: &mut CommandRegistry);

    /// Register data providers (optional, default does nothing).
    fn register_data_providers(&self, _registry: &mut DataRegistry) {}
}
```

**When to use commands vs data providers:**

| Use Case | Register As |
|----------|-------------|
| Run an external tool (cargo, npm, etc.) | Command via `CommandSpec::Exec` |
| Combine multiple commands | Command via `CommandSpec::Composite` |
| Provide data to other extensions | Data provider |
| Read-only workspace information | Data provider |

### Example: Custom Git Status Extension

```rust
use crate::extension::{CommandRegistry, Context, DataProvider, DataRegistry, Extension};
use std::process::Command;

pub struct GitExtension;

impl Extension for GitExtension {
    fn name(&self) -> &'static str {
        "git"
    }

    fn register_commands(&self, _registry: &mut CommandRegistry) {}

    fn register_data_providers(&self, registry: &mut DataRegistry) {
        registry.register("git_status", Box::new(GitStatusProvider));
    }
}

struct GitStatusProvider;

impl DataProvider for GitStatusProvider {
    fn name(&self) -> &'static str {
        "git_status"
    }

    fn provide(&self, ctx: &Context) -> Result<serde_json::Value, anyhow::Error> {
        let output = Command::new("git")
            .args(["status", "--porcelain", "--branch"])
            .current_dir(&ctx.working_directory)
            .output()?;

        if !output.status.success() {
            anyhow::bail!("git status failed");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();
        
        let branch = lines.first()
            .and_then(|l| l.strip_prefix("## "))
            .and_then(|l| l.split("...").next())
            .unwrap_or("unknown");

        let modified = lines.iter().skip(1).filter(|l| !l.starts_with("??")).count();
        let untracked = lines.iter().skip(1).filter(|l| l.starts_with("??")).count();

        Ok(serde_json::json!({
            "branch": branch,
            "modified": modified,
            "untracked": untracked,
            "clean": modified == 0 && untracked == 0,
        }))
    }
}
```

**Using the provider:**

```rust
let git = ctx.get_or_provide("git_status", &registry)?;
if git["clean"].as_bool().unwrap_or(false) {
    println!("Working directory is clean on branch {}", git["branch"]);
}
```

---

## Extension Loading

Extensions are loaded at startup. The built-in `MetadataExtension` is registered automatically. Future versions may support dynamic loading from external crates.

---

## Context Reference

The `Context` struct provides access to configuration and cached data:

```rust
pub struct Context {
    pub config: Arc<Config>,
    pub data_cache: HashMap<String, Arc<serde_json::Value>>,
    pub working_directory: PathBuf,
}
```

| Method | Description |
|--------|-------------|
| `new(config, working_directory)` | Create a new context |
| `get_or_provide(key, registry)` | Get cached data or compute via provider |

---

## DataRegistry Reference

```rust
pub struct DataRegistry { /* ... */ }
```

| Method | Description |
|--------|-------------|
| `new()` | Create an empty registry |
| `register(name, provider)` | Register a data provider |
| `get(name)` | Get a provider by name |
| `provide(name, ctx)` | Execute a provider directly |
