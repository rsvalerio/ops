# cargo-ops Design Document

> An opinionated, batteries-included Rust development CLI. Zero config, maximum quality.

## Vision & Principles

**The Problem:** Setting up a quality Rust development workflow requires configuring multiple tools (clippy, fmt, nextest, machete, audit), writing custom xtask commands, and encoding best practices manually in every project.

**The Solution:** `cargo-ops` is a drop-in CLI that delivers an opinionated, production-ready development workflow out of the box. Install it once, run `cargo ops init` in any Rust project, and get a thorough quality pipeline with theme-based output.

**Core Principles:**
- **Opinionated over configurable** - We make decisions so you don't have to
- **Drop-in ready** - Works immediately on any Rust project
- **Rich feedback** - Theme-based step lines (icons, command, time) and clear error reporting
- **Extensible** - Easy to add custom commands via configuration and extensions
- **Strong foundations** - Plain-text output with a clear visualization spec for themes

## User-Facing Behavior

### Command Execution

```bash
cargo ops <command>
```

Runs a named command defined in configuration or provided by extensions. Commands come from merged config and extensions:
1. **Merged config** (later overrides earlier): internal default (when no local file) → global config (`~/.config/cargo-ops/config.toml`) → local `.ops.toml` → environment (`CARGO_OPS_*`)
2. **Extensions** register at startup: built-in extensions (e.g. metadata data provider) are registered when the CLI runs, so their commands and data providers are available for the session

### Output: Theme-Based Plain Text

All output is plain text (no TUI). Step display is implemented with **indicatif**’s **MultiProgress**: one progress bar (spinner) per step.

**All steps visible, changing status:** At plan start, all steps are shown as spinners with the **exact command** (e.g. `cargo build --all-targets`). When a step starts, its bar shows a running icon + command (e.g. `⟳ cargo build ...`) and the spinner animates. When the step finishes or fails, the bar is finished with **finish_with_message** showing icon + command + dot-padding + elapsed time (e.g. `  ✅ cargo build --all-targets ............... 0.35s`). **Theme** (classic vs compact) only affects the icon in that final message (✅/✗ vs ✓/✗). Subprocess stdout/stderr is collected during execution; on step failure, the last N lines of stderr are shown. Output is not streamed in real time. When stderr is not a TTY, progress is hidden and final step lines are written with writeln so CI/logs still see the outcome.

**Example (theme = `classic`, default):**

```text
Running: build, clippy, test
⠁ cargo build --all-targets
⠂ cargo clippy --all-targets -- -D warnings
⠄ cargo test --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.48s
✅ cargo build --all-targets  0.53s
✅ cargo clippy --all-targets -- -D warnings  0.42s
✅ cargo test --all-targets  0.80s
Done in 23.62s
```

**Error reporting:** A failing step is finished with ✗ and the elapsed time, then `failed: <message>` on the next line (via multi.println or writeln when hidden).

### Visualization Spec

Step lines are rendered by indicatif:

- **While running:** Spinner + message (e.g. `⟳ cargo build --all-targets`). Indicatif manages cursor and redraw.
- **When finished:** `finish_with_message` sets the final line via **`output::format_step_line(icon, command, duration_secs, columns)`**: icon + command + dot-padding to align the duration at the configured column width. Theme controls the icon (classic = ✅/✗, compact = ✓/✗). Config `output.columns` defines the line width used for padding.

**Future: JSON output (`--json` flag):** Structured output for automation and tooling integration.

See [components.md](components.md) for the full component catalog.

## Configuration Model (TOML)

### Output and Theme Configuration

```toml
[output]
theme = "classic"   # "classic" (default) or "compact"
columns = 80        # Line width in columns (no runtime change)
```

**Built-in themes:** Theme only affects the **finish message** icon in `format_step_line`.

- **classic** (default): Finish message uses ✅ (success) or ✗ (failed). Example: `  ✅ cargo fmt --all ............. 0.39s`
- **compact**: Finish message uses ✓ (success) or ✗ (failed). Example: `  ✓ build ...................... 2.30s`

Step display (spinner + message, then finish) is handled by indicatif; theme only customizes the final line icon.

### Command Configuration

Commands can be defined as either **exec commands** (run a program) or **composite commands** (run multiple commands):

```toml
# Single exec command
[commands.build]
program = "cargo"
args = ["build", "--all-targets"]

# Composite command (sequential)
[commands.verify]
commands = ["build", "clippy", "test"]
parallel = false

# Composite command (parallel)
[commands.lint]
commands = ["fmt", "clippy", "check"]
parallel = true
fail_fast = true   # default true; when false, run all steps even if one fails
```

Configuration supports:
- Environment variables: `env = { VAR = "value" }`
- Working directory: `cwd = "./subdir"`
- Timeout: `timeout_secs = 300` (seconds); enforced via `tokio::time::timeout`

## Command System Architecture

### Internal Command Representation

Commands are normalized into strongly-typed internal structures:

```rust
enum CommandSpec {
    Exec(ExecCommandSpec {
        program: String,
        args: Vec<String>,
        env: HashMap<String, String>,
        cwd: Option<PathBuf>,
        timeout: Option<Duration>,
    }),
    Composite(CompositeCommandSpec {
        commands: Vec<CommandId>,
        parallel: bool,
        fail_fast: bool,  // default true; when parallel, stop remaining steps on first failure
    }),
}
```

### Execution Plan

The command system builds a `RunPlan` from configuration and extension-registered commands:

- **Sequential plan**: List of commands executed in order, stopping on first failure
- **Parallel plan**: When `parallel = true`, steps run concurrently. When `fail_fast = true` (default), the first failure sets an abort flag and remaining tasks skip execution; when `fail_fast = false`, all tasks run to completion and overall success is false if any failed
- **DAG support**: Future enhancement for complex dependency graphs

### Execution Events

The runner produces typed events that drive theme-based output:

```rust
enum RunnerEvent {
    PlanStarted { command_ids: Vec<CommandId> },
    StepStarted { id: CommandId, display_cmd: Option<String> },
    StepOutput { id: CommandId, line: String, stderr: bool },
    StepFinished { id: CommandId, duration_secs: f64, display_cmd: Option<String> },
    StepFailed { id: CommandId, duration_secs: f64, message: String, display_cmd: Option<String> },
    RunFinished { duration_secs: f64, success: bool },
}
```

`display_cmd` is the full command string (e.g. `cargo build --all-targets`) used by the **classic** theme; the **compact** theme uses the step `id` instead.

## Extension Architecture (Compile-Time)

### Extension System Overview

The extension system uses compile-time registration with a trait-based architecture. This provides:
- Type safety and compile-time validation
- Zero runtime overhead
- Clear extension points for future dynamic loading

### Extension Trait

```rust
trait Extension: Send + Sync {
    fn name(&self) -> &'static str;
    fn register_commands(&self, registry: &mut CommandRegistry);
    fn register_data_providers(&self, registry: &mut DataRegistry);
}
```

### Command Registry

Extensions can register new commands that become available alongside config-defined commands:

```rust
type CommandRegistry = HashMap<CommandId, CommandSpec>;
```

### Data Provider Registry

Data providers expose structured data (as JSON) that other extensions and commands can consume:

```rust
trait DataProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn provide(&self, ctx: &Context) -> Result<serde_json::Value>;
}

struct DataRegistry {
    providers: HashMap<String, Box<dyn DataProvider>>,
}
```

**Data Provider Features:**
- Return `serde_json::Value` for interop between extensions
- Caching in `Context` (per run) so multiple commands can reuse the same data
- Lazy evaluation - data is only fetched when requested

### Example: Metadata Extension

The `metadata` extension demonstrates the data provider pattern:

```rust
struct MetadataExtension;

impl Extension for MetadataExtension {
    fn register_data_providers(&self, registry: &mut DataRegistry) {
        registry.register("metadata", Box::new(MetadataProvider));
    }
}
```

### Extension Context

The `Context` struct provides shared state and utilities for extensions:

```rust
struct Context {
    config: Config,
    data_cache: HashMap<String, serde_json::Value>,
    working_directory: PathBuf,
}
```

## Extensibility Roadmap

### Current Foundation (Compile-Time Extensions)

- Extension trait system with command and data provider registries
- Strong typing throughout for safety and clarity
- Theme-based output with visualization spec and configurable columns

### Future Enhancements

**Enhanced Data Providers:**
- Streaming data providers for large outputs
- Data provider dependencies (provider A depends on provider B)
- Incremental updates and change detection

**Themes:**
- User-defined theme names and line-format rules
- Optional theme files (e.g. custom icons/spacer) while respecting the visualization spec

## Implementation Notes

### Dependencies

Core dependencies:
- `config`: Configuration parsing (TOML, env)
- `serde` / `serde_json`: Serialization for config and extensions
- `tokio`: Async runtime for command execution
- `clap`: CLI parsing
- `tracing` / `tracing-subscriber`: Logging

### Module Structure

```
cargo-ops/
├── src/
│   ├── main.rs
│   ├── config.rs       # TOML config, OutputConfig (theme, columns)
│   ├── command.rs      # Command specs, execution, RunnerEvent
│   ├── output.rs       # Theme formatting (classic, compact)
│   ├── extension.rs    # Extension trait and registries
│   └── extensions/
│       ├── mod.rs
│       └── metadata.rs # Metadata data provider
```

### Testing Strategy

- **Unit tests**: Config parsing, command resolution, theme formatting
- **Integration tests**: End-to-end command execution with mock commands
