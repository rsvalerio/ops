# cargo-ops CLI Output

This document describes the CLI output behavior for cargo-ops: how runner events are printed using indicatif for step display.

## Output Flow

When you run a named command (e.g. `cargo ops build` or `cargo ops verify`):

1. The runner expands the command to a plan (flat list of exec steps).
2. At **plan start**, the CLI creates an indicatif **MultiProgress** and adds one **ProgressBar** (spinner) per step. Each bar shows the exact command (e.g. `cargo build --all-targets`). A header line `Running: build, clippy, test` is printed via `MultiProgress::println`.
3. Each step runs in sequence; the runner emits `RunnerEvent` for each phase.
4. On **step start**, that step’s bar message is set to a running icon + command (e.g. `⟳ cargo build ...`) and the spinner animates via `enable_steady_tick`.
5. On **step finish** or **step fail**, that bar is finished with `finish_with_message` using a final line produced by `output::format_step_line` (icon + command + dot-padding + elapsed time, e.g. `  ✅ cargo build --all-targets ............... 0.35s`). Theme (classic vs compact) only affects the icon.
6. **Subprocess output** is collected during execution; on step failure, the last N lines of stderr are shown. Output is not streamed in real time.
7. **Run finished:** A summary line `Done in X.XXs` or `Failed in X.XXs` is printed via `multi.println` (or writeln when progress is hidden).

## Non-TTY (hidden progress)

When stderr is **not** a terminal (e.g. CI, piping), the CLI uses **ProgressDrawTarget::hidden()** so progress bars are not rendered. In that case, after each step finish or fail the CLI also **writeln**s the final step line and any `failed: <message>` to stderr, so logs and CI still see the outcome. The summary line is written with writeln as well.

## Step line format

Step lines are rendered by indicatif: while a step runs, the bar shows a spinner + message (e.g. `⟳ cargo build --all-targets`). When the step ends, `finish_with_message` sets the final line via `output::format_step_line(icon, command, duration_secs, columns)`, which produces icon + command + dot-padding + duration (e.g. `  ✅ cargo build --all-targets ............... 0.35s`). Theme controls the icon (classic = ✅/✗, compact = ✓/✗). Config `output.columns` sets the line width used for dot-padding.

## Themes

- **classic** (default): Finish line uses ✅ (success) or ✗ (failed).
- **compact**: Finish line uses ✓ (success) or ✗ (failed).

## Exit Codes

- **0** when all steps succeed
- **1** when any step fails or the command is unknown

## No Command

When invoked with no command (e.g. `cargo ops`), the CLI prints available commands (if config loads) and then help, then exits 0.

See [designdoc.md](designdoc.md) for the full architecture including config, command execution, and extensions.

See [components.md](components.md) for a visual component reference.
