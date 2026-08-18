# Plan: link a prebuilt libduckdb instead of compiling the amalgamation

Smaller sibling of [`duckdb-cli-backend.md`](duckdb-cli-backend.md). Same goal —
stop compiling the DuckDB C++ amalgamation on every cold build — but **zero Rust
code changes**. The trade is build-system and release work instead of a refactor.

> **Status: all phases done, verified end to end.** The static link — the one
> thing this plan was gated on — was proven on `x86_64-unknown-linux-gnu` on
> 2026-08-18, and a Release `dry-run` then built all four matrix legs green
> (mac binaries link no `libduckdb`). The mechanism now serves dev (dynamic),
> CI (dynamic), and releases (static).

## What `libduckdb-sys` actually supports

Read from `libduckdb-sys-1.10505.0/build.rs` (the version in `Cargo.lock`; note
`Cargo.toml` declares `1.10502` and resolves to `1.10505.0`, i.e. **DuckDB
v1.5.5**).

`find_duckdb()` resolves in this order:

1. `DUCKDB_LIB_DIR` — link against a lib in that directory. Optionally
   `DUCKDB_INCLUDE_DIR` for headers.
2. `DUCKDB_DOWNLOAD_LIB=1|true` — download the official prebuilt from
   `https://github.com/duckdb/duckdb/releases/download/v{version}/{archive}`.
3. vcpkg (msvc only), then pkg-config, then a bare `-lduckdb` fallback.

`link_directive()` picks the link mode: `DUCKDB_STATIC` unset or `"0"` gives
`dylib=duckdb`; any other value gives `static=duckdb_static`.

Two properties worth noting:

- **The version cannot drift.** The download version is derived from the crate
  version (`duckdb_version_from_pkg_version`), so the prebuilt lib always matches
  the generated bindings. This is a real advantage over the CLI-backend plan,
  where storage-format skew is a live hazard.
- **`DUCKDB_INCLUDE_DIR` is not needed.** `HeaderLocation::from_env` falls back to
  `<DUCKDB_LIB_DIR>/duckdb.h`, and every official archive ships `duckdb.h`
  alongside the libraries. One env var, not two.

### `bundled` gates everything — drop it first

`build.rs` dispatches on the feature at the top of `main()`:

```rust
#[cfg(feature = "bundled")]      build_bundled_backend::main(&out_dir, &out_path);
#[cfg(not(feature = "bundled"))] build_linked::main(&out_dir, &out_path)
```

`find_duckdb` lives in `build_linked`. So while `Cargo.toml` declares
`duckdb = { version = "1.10502", features = ["bundled"] }`, **`DUCKDB_LIB_DIR`
and `DUCKDB_STATIC` are read by nothing** — the amalgamation compiles regardless
and the experiment silently proves nothing. Dropping `features = ["bundled"]` from
the workspace dependency is a *precondition* of any experiment here, not a later
cleanup step.

### Do not enable the `json` feature

`libduckdb-sys` maps `json = ["bundled"]` and `parquet = ["bundled"]` — turning
either on drags the amalgamation back in and defeats the whole exercise. Not a
problem: JSON is already present without them. The official static archive ships
`libjson_extension.a` explicitly, and `ops about` — which ingests tokei output
through `read_json_auto` — runs correctly against the statically linked build.

## Four gotchas, all real

**1. The build script's download is unverified.** `ensure_libduckdb` fetches the
zip and renames it into place with no checksum or signature check. This repo
SHA-pins every third-party GitHub Action (SEC-35/TASK-1661) and deleted an
unpinned release download as "pure supply-chain surface for no benefit"
(SEC-37/TASK-1663). `DUCKDB_DOWNLOAD_LIB=1` is exactly that pattern. **Do not
enable it.** Fetch the archive ourselves with a pinned SHA-256 and point
`DUCKDB_LIB_DIR` at the result — same outcome, verified, and consistent with the
existing posture. Note that DuckDB publishes **no checksum assets at all** on its
releases, so the pin has to be ours: hash the archive once, commit the digest.

**2. The download path poisons the rpath for distribution.**
`configure_link_search` emits `-Wl,-rpath,<download_dir>` where `download_dir` is
an absolute path under the *build machine's* `target/`. Fine locally; fatal in a
shipped binary, which would carry an rpath to a directory that does not exist on
the user's machine. Another reason the release path must be static.

**3. Nothing links the C++ runtime for you.** `link_windows_system_libs()` — the
only place `-lstdc++` is emitted — is `#[cfg(feature = "bundled")]` *and*
Windows-gated by its callers. On the non-bundled path the build script emits no
C++ runtime at all, so a static link fails with a wall of `std::` undefined
symbols until you add `-lstdc++` yourself (`-lc++` on macOS). This reads like
"the static link doesn't work"; it is one flag.

**4. Static linking needs 22 archives, not one.** The official
`static-libs-linux-amd64.zip` (29.9 MB, 131 MiB unpacked) contains 22 `.a` files
plus `duckdb.h`:

```
libduckdb_static.a  (82 MB)   libjson_extension.a       libparquet_extension.a
libcore_functions_extension.a libicu_extension.a        libautocomplete_extension.a
libduckdb_fmt.a               libduckdb_pg_query.a      libduckdb_re2.a
libduckdb_utf8proc.a          libduckdb_miniz.a         libduckdb_yyjson.a
libduckdb_zstd.a              libduckdb_jemalloc.a      libduckdb_mbedtls.a
libduckdb_fastpforlib.a       libduckdb_hyperloglog.a   libduckdb_fsst.a
libduckdb_skiplistlib.a       libduckdb_generated_extension_loader.a
libtpch_extension.a           libtpcds_extension.a      duckdb.h
```

`build.rs` emits `cargo:rustc-link-lib=static=duckdb_static` and nothing else, so
the remaining 21 archives must be supplied externally. Link order between them
does matter under GNU ld — but you do not have to work it out. Pass all 22 inside
`-Wl,--start-group … -Wl,--end-group` and let the linker iterate to a fixed point.

Note also that `DUCKDB_DOWNLOAD_LIB` fetches `libduckdb-<platform>.zip` (shared
library only) — the static libs live in a *separate* `static-libs-<platform>.zip`
that the build script never downloads. Download-mode and static-mode are mutually
exclusive.

## The archive maps are not the same map

The build script's own table maps all four dist targets onto the *dynamic*
archives, with one universal binary covering both macOS targets:

| target | dynamic (`DUCKDB_DOWNLOAD_LIB`) | static (Path B) |
|---|---|---|
| `aarch64-apple-darwin` | `libduckdb-osx-universal.zip` | `static-libs-osx-arm64.zip` |
| `x86_64-apple-darwin` | `libduckdb-osx-universal.zip` | `static-libs-osx-amd64.zip` |
| `x86_64-unknown-linux-gnu` | `libduckdb-linux-amd64.zip` | `static-libs-linux-amd64.zip` |
| `aarch64-unknown-linux-gnu` | `libduckdb-linux-arm64.zip` | `static-libs-linux-arm64.zip` |

**There is no `static-libs-osx-universal.zip`.** All four dist targets are still
covered, but the fetch script needs two distinct tables — Path B is not a string
substitution on Path A's table.

## Two paths

**Path A — dynamic, dev and CI only.** Fetch `libduckdb-<platform>.zip`, verify
the checksum, extract to a cache dir, export `DUCKDB_LIB_DIR`. Removes the C++
compile from every local build and every CI job. Does not touch releases, which
keep using `bundled` until Path B lands. Low risk, quick payoff, fully
reversible.

**Path B — static, for releases.** Fetch `static-libs-<platform>.zip`, verify,
extract, set `DUCKDB_LIB_DIR` + `DUCKDB_STATIC=1`, and add the extra link flags.
Produces a self-contained `ops` binary — no shared library to ship, no rpath, no
new runtime dependency, no change to the Homebrew formula or the shell installer.

Path B is the actual prize; Path A is a safe way to bank most of the benefit
first.

## Phase 0 — proving the static link (done)

Run on `x86_64-unknown-linux-gnu`. Reproduce with:

```sh
# 1. drop the feature that disables the whole mechanism
#    Cargo.toml:  duckdb = { version = "1.10502" }        # was features = ["bundled"]

# 2. fetch and verify — DuckDB publishes no checksums, so pin our own
curl -sSLo static-libs-linux-amd64.zip \
  https://github.com/duckdb/duckdb/releases/download/v1.5.5/static-libs-linux-amd64.zip
echo "deb47c5300f3c99725e84cdb14d214c3b12bbd748b613b1698b938c894cb68eb  static-libs-linux-amd64.zip" \
  | sha256sum -c
unzip -q static-libs-linux-amd64.zip -d "$LIBDIR"

# 3. point the build script at it
export DUCKDB_LIB_DIR="$LIBDIR"
export DUCKDB_STATIC=1

# 4. group all 22 archives, then the C++ runtime
FLAGS="-L native=$LIBDIR -C link-arg=-Wl,--start-group"
for a in "$LIBDIR"/*.a; do FLAGS="$FLAGS -C link-arg=$a"; done
export RUSTFLAGS="$FLAGS -C link-arg=-Wl,--end-group -C link-arg=-lstdc++"

cargo build -p ops --features stack-rust,tokei,coverage
```

Results:

- **Links clean, first attempt**, with `--start-group` doing the ordering work.
- `ldd target/debug/ops` shows **no `libduckdb`**: `libstdc++`, `libgcc_s`,
  `libm`, `libc`. That is the *identical* dependency set as the existing
  `bundled` release binary — the static path adds **no new runtime dependency**.
- `ops about` produces correct output, exercising `read_json_auto` and the
  DuckDB views end to end without the `json` feature.
- `cargo test --workspace` is green: **2456 passed, 0 failed** across 29 test
  targets. (Watch the exit code, not the tail: piping cargo through `tail`
  reports the pipe's status and will hide a failure. An earlier run here failed
  one `ops-deps` tracing test — a load-sensitive flake in a crate that does not
  depend on DuckDB at all, and green in isolation.)

One cost worth naming: all 29 test targets statically link the archive set, so
the link phase gets noticeably heavier in dev and CI. That is an argument for
keeping Path A (dynamic) on those jobs and reserving the static link for release
builds, which is what the two-path split already proposes.

Use `--features stack-rust,tokei,coverage`, not `--all-features`: the latter is a
workspace-feature flag that does not affect the `duckdb` dependency, and would
silently re-enable `bundled` the day anyone adds a `json`/`parquet` passthrough.

## Phase 1 — a single fetch mechanism (done)

`scripts/fetch-duckdb.sh` + `scripts/duckdb-pins.txt`. One code path for
developers, CI, and releases:

```sh
eval "$(scripts/fetch-duckdb.sh)"                  # dynamic, host target
eval "$(scripts/fetch-duckdb.sh --mode static)"    # static, host target
eval "$(scripts/fetch-duckdb.sh --target aarch64-unknown-linux-gnu --mode static)"
```

It maps the triple through the correct table per mode (including the two
separate macOS static archives), verifies the pinned SHA-256 *before* extract,
caches under `target/duckdb-prebuilt/<version>/`, and prints the env exports
on stdout — `DUCKDB_LIB_DIR` (+ `DUCKDB_STATIC=1` for static) plus the
`RUSTFLAGS` the build script does not emit itself: the C++ runtime, the
satellite archives in a `--start-group`, and (dynamic mode) the rpath test
binaries need to find `libduckdb.so`, since the `DUCKDB_LIB_DIR` path emits no
rpath of its own.

`duckdb-pins.txt` is the single file a version bump touches: the `version`
line plus every hash. DuckDB publishes no checksums, so these are
self-computed pins (SEC-37/TASK-1663 posture — unlike the cargo-dist installer
noted in `release.yml`, here the pinned archive *is* the entire linked
payload, so the pin covers everything). The script also cross-checks the pins
version against the `libduckdb-sys` version encoded in `Cargo.lock`
(`1.10505.0` ⇒ `v1.5.5`) and refuses on skew, so bumping the dependency
without bumping the pins fails loudly instead of linking a mismatched lib.

Verified on `x86_64-unknown-linux-gnu`, both modes, from the script's own
exports: static — `cargo build -p ops-duckdb` in 54 s and 183 tests green;
dynamic — 183 tests green with the test binary's `RUNPATH` resolving
`libduckdb.so` from the cache dir. Failure modes tested: unknown triple,
unknown mode, pins/Cargo.lock skew (rejected with an actionable message), and
a corrupt cached archive (detected, deleted, told to re-run).

Not yet exercised off Linux: the macOS legs. Apple's linker — classic ld64
and the Xcode 15+ rewrite alike — rejects `--start-group` as an unknown
option, but also does not need it: it resolves references between static
archives on its own, so the script lists them plainly there (this is also how
CMake consumers link DuckDB's static archives on macOS). If a symbol ever
escaped that resolution, `DUCKDB_DARWIN_FORCE_LOAD=1` links every member of
every archive instead (larger binary, same result).

## Phase 2 — Path A in CI (done)

Three changes:

1. **`bundled` is dropped** from the workspace `duckdb` dependency. The lock
   diff is tiny: `cc` and `jobserver` fall out of `libduckdb-sys`'s build
   deps (`cc` itself stays — other crates use it).
2. **The fetch step is in `ci.yml`** — but only in the jobs that *link* a
   binary: `build` and `test`. The original plan said check/build/test/msrv;
   that was wrong twice over. `cargo check`/`clippy`/`msrv` never link, and
   `libduckdb-sys`'s build script doesn't need the archive without `bundled`
   (bindings are pregenerated) — verified: `cargo check -p ops-duckdb` passes
   with no `DUCKDB_LIB_DIR` at all. A linking build without it dies with the
   bare-fallback error (`mold: fatal: library not found: duckdb`) — the same
   failure mode Phase 4 must document for local dev.
3. **Releases stay on `bundled`** via an explicit step in `release.yml`
   (`build-local-artifacts`) that re-adds the feature before `dist build`,
   with a grep guard that fails loudly if the sed pattern rots. Reason: with
   one workspace dependency line, dropping `bundled` reaches the release
   build too — and the static (Path B) link is unverified on macOS, while
   shipping a binary against the dynamic `libduckdb.so` is not an option
   (rpath gotcha above). **Phase 3 has since deleted this step** — it exists
   only in git history from Phase 2 to Phase 3.

### What CI actually gains — the premise correction

The plan assumed "each [CI job] pays the full C++ build". It doesn't:
`actions-rust-lang/setup-rust-toolchain` caches compiled targets per job, and
on a warm run `libduckdb-sys` comes up `Fresh` — the most recent `main` Test
job ran `cargo test --all --all-features` in **87 s**. The amalgamation is
paid on **cache miss**: every duckdb version bump, cache eviction, and
fork-PR (no cache access). So Phase 2's CI win is *determinism* (no
cache-dependent 10+ minute outlier runs) and *cache pressure* (the compiled
amalgamation no longer occupies the multi-GB target cache) — the large
per-run win was always local dev, where no such cache exists.

One-time cost, worth knowing: setting `RUSTFLAGS` (the rpath flag) changes
every crate's fingerprint, so the first CI run after this lands rebuilds
everything once and re-warms sccache.

Local developer workflow change: `eval "$(scripts/fetch-duckdb.sh)"` before
any linking build (Phase 4 documents this).

**Measurements** (cold `cargo build -p ops --all-features`, scratch target
dirs, warm sccache — the rustc graph is cached in both runs, so the delta is
the amalgamation itself):

| | wall time | peak RSS | target dir |
|---|---|---|---|
| bundled (amalgamation) | **8m 49s** | 3.2 GB | 7.5 GB |
| prebuilt dynamic (Path A) | **1m 26s** | 1.0 GB | 2.0 GB |

(The bundled run's final link aborted when the scratch tmpfs filled; the link
re-ran separately in 5.5 s, included above. The 848 MB debug binary is part of
that 7.5 GB.) **~7.4 minutes, 85%, off every cold linking build** — locally,
and on every CI cache miss.

## Phase 3 — Path B in releases (done)

The bridge is gone: `release.yml`'s `build-local-artifacts` now has a
"Fetch prebuilt static libduckdb" step instead of the bundled re-enable, and
each leg links the static archive set. Facts that made it simple, checked
against `dist plan` for the pinned cargo-dist 0.31.0:

- **Every matrix leg builds exactly one target** — no universal-macOS leg —
  so the step fetches for `${{ matrix.targets[0] }}` alone, and the two
  separate macOS static archives map cleanly onto the two macOS legs.
- **Every dist runner builds natively** (`ubuntu-22.04-arm` covers
  aarch64-linux) — no cross-compilation, so the absolute archive paths in
  the script's `RUSTFLAGS` are valid where cargo runs.
- **The rustflags reach `dist build` through `GITHUB_ENV`** — `dist build`
  shells out to cargo, which inherits the job env. The alternative (a
  committed `.cargo/config.toml`) cannot work: it would have to embed the
  absolute cache path.

The step evals the script and persists `DUCKDB_LIB_DIR`, `DUCKDB_STATIC=1`,
and `RUSTFLAGS` across steps, mirroring ci.yml's Phase 2 step. The
`DUCKDB_PREBUILT_QUIET` line went away with the bridge step — it existed to
silence ops-duckdb's warning for a build that deliberately linked the
amalgamation; release builds now set `DUCKDB_LIB_DIR`, so the warning does
not fire (the env knob itself stays in `build.rs` as a general escape
hatch).

Verified on `x86_64-unknown-linux-gnu` with a faithful replay of the leg
(`--profile=dist --target=… -p ops --all-features` from the script's own
exports): links clean, `ldd` shows no `libduckdb` and exactly the bundled
release's dependency set (`libstdc++`, `libgcc_s`, `libm`, `libc`), and
`about code` / `about loc` exercise the DuckDB paths end to end.

**One accepted cost: binary size.** The static binary is 62 MB vs 49 MB for
the last bundled release (tarball 24 MB vs 19 MB). The delta is almost
entirely `.rodata` (3×) and `.text` — DuckDB's official static distribution
simply carries more than the amalgamation config duckdb-rs compiles, and it
is all live: the archive members are built with function-sections, but
`-Wl,--gc-sections` sheds nothing (62 MB either way), so no flag recovers
it. If 5 MB of tarball ever matters more than ~9 min × 4 legs of release
CI, the Phase 2 bridge in git history is the revert.

The macOS legs and the aarch64-linux leg were exercised by a `dry-run`
dispatch of the Release workflow (2026-08-18, branch
`build/prebuilt-libduckdb`): **all four legs green**, and the produced
aarch64-macos binary links only system dylibs (`libSystem`, `libc++`,
`libiconv` — no `libduckdb`), 53 MB. The plain-archive-listing Darwin
default (see Phase 1) linked clean on the first attempt.

That dry-run also surfaced a pre-existing breakage, fixed alongside: the
release legs had no Rust toolchain step of their own and silently depended
on the runner image staying ahead of the workspace `rust-version` — the
macOS images ship 1.96.0, the workspace requires 1.97 (raised in bc5217d,
after the last release dispatch, so nothing had noticed). The legs now
install the pinned toolchain via rustup, deterministic against image drift.

## Phase 4 — document the developer setup (done)

Three pieces:

- **`AGENTS.md`** gains a "DuckDB prebuilt library" section: the
  `eval "$(scripts/fetch-duckdb.sh)"` line before any linking build
  (`cargo build`/`test`/`install`, `ops verify qa`), the fact that
  `check`/`clippy`/`fmt` work without it, the network-on-first-fetch and cache
  behaviour. `README.md`'s Local development section shows the same eval.
- **The failure mode is now self-explanatory.** `extensions/duckdb/build.rs`
  prints a `cargo:warning` whenever `DUCKDB_LIB_DIR` is unset — stating that
  check/clippy pass but linking will fail with `library not found: duckdb`,
  and the exact fix — so the warning sits directly above the eventual link
  error instead of the mold message being the only clue. A warning, not an
  error, because non-linking builds are legitimately fine without the variable
  (CI's check/clippy/msrv jobs rely on that). Verified in all three states:
  unset → warns; set → silent; `DUCKDB_PREBUILT_QUIET=1` → silent.
- **`DUCKDB_PREBUILT_QUIET=1`** suppresses the warning for builds that link
  the amalgamation on purpose. The release workflow's bundled re-enable step
  sets it, since `bundled` makes the prebuilt-lib contract false there.

## Relationship to the CLI-backend plan

These are alternatives, not a sequence. Compared with
[`duckdb-cli-backend.md`](duckdb-cli-backend.md):

| | prebuilt lib | CLI backend |
|---|---|---|
| Rust changes | none | Phase 1 refactor (~20 closure signatures, `lock()` removal, 103 test sites) |
| Runtime dependency | none (verified) | a `duckdb` binary must be found |
| Version skew risk | none (derived from crate version) | real (storage format) |
| Distribution | static archive fetch in CI | vendor a binary into the archives |
| Main risk | ~~the static link~~ — proven | subprocess overhead, in-memory DB semantics |

**This plan's one hard question is answered.** The static link works, adds no
runtime dependency, and needed no Rust changes. See
[`duckdb-alternatives.md`](duckdb-alternatives.md) for whether DuckDB should hold
the job at all — this plan keeps all 140 DuckDB-related crates in the dependency
graph, which is the larger prize SQLite would collect.
