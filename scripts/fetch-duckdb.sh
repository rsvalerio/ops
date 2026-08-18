#!/usr/bin/env bash
# Fetch, verify, and stage a prebuilt libduckdb — the one code path used by
# developers, CI, and releases (docs/duckdb-prebuilt-lib.md, Phase 1).
#
# Usage:
#   eval "$(scripts/fetch-duckdb.sh)"                          # dynamic, host target
#   eval "$(scripts/fetch-duckdb.sh --mode static)"            # static, host target
#   eval "$(scripts/fetch-duckdb.sh --target aarch64-unknown-linux-gnu --mode static)"
#
# Prints `export` lines on stdout for eval; progress goes to stderr. The
# exports set DUCKDB_LIB_DIR (+ DUCKDB_STATIC for static mode) and prepend the
# link flags to RUSTFLAGS that libduckdb-sys's build script does not emit
# itself: the C++ runtime and, for static mode, every satellite archive.
#
# Requires the workspace duckdb dependency to be built without `bundled`
# (Phase 2); with `bundled` on, the build script never reads these variables.
#
# Checksums are pinned in duckdb-pins.txt next to this script (DuckDB
# publishes none). An archive with no pin is an error, not a warning.

set -euo pipefail

usage() {
  cat >&2 <<'EOF'
usage: fetch-duckdb.sh [--mode dynamic|static] [--target <triple>]

  --mode    dynamic (default; dev + CI) or static (releases)
  --target  Rust target triple (default: host triple from rustc -vV)
EOF
  exit 2
}

die() { echo "fetch-duckdb.sh: error: $*" >&2; exit 1; }

MODE=dynamic
TARGET=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --mode) [[ $# -ge 2 ]] || usage; MODE=$2; shift 2 ;;
    --target) [[ $# -ge 2 ]] || usage; TARGET=$2; shift 2 ;;
    -h|--help) usage ;;
    *) usage ;;
  esac
done
case "$MODE" in
  dynamic|static) ;;
  *) die "unknown mode '$MODE' (expected dynamic or static)" ;;
esac

REPO_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
PINS_FILE=$REPO_ROOT/scripts/duckdb-pins.txt
[[ -f $PINS_FILE ]] || die "pins file not found: $PINS_FILE"

# --- target triple ---------------------------------------------------------
if [[ -z $TARGET ]]; then
  TARGET=$(rustc -vV | sed -n 's/^host: //p')
  [[ -n $TARGET ]] || die "could not detect host triple via rustc -vV"
fi

# Two tables, deliberately separate: the dynamic archive for macOS is one
# universal zip, the static archives are per-arch. There is no
# static-libs-osx-universal.zip. (Windows is absent on purpose — no dist
# target builds it and no pin covers it.)
archive_for_target() {
  local mode=$1 triple=$2
  case "$triple" in
    aarch64-apple-darwin)
      case "$mode" in
        dynamic) echo libduckdb-osx-universal.zip ;;
        static)  echo static-libs-osx-arm64.zip ;;
      esac ;;
    x86_64-apple-darwin)
      case "$mode" in
        dynamic) echo libduckdb-osx-universal.zip ;;
        static)  echo static-libs-osx-amd64.zip ;;
      esac ;;
    x86_64-unknown-linux-gnu)
      case "$mode" in
        dynamic) echo libduckdb-linux-amd64.zip ;;
        static)  echo static-libs-linux-amd64.zip ;;
      esac ;;
    aarch64-unknown-linux-gnu)
      case "$mode" in
        dynamic) echo libduckdb-linux-arm64.zip ;;
        static)  echo static-libs-linux-arm64.zip ;;
      esac ;;
    *) return 1 ;;
  esac
}

ARCHIVE=$(archive_for_target "$MODE" "$TARGET") \
  || die "no $MODE archive mapped for target '$TARGET'"

# --- version agreement: pins vs Cargo.lock ---------------------------------
# duckdb-rs encodes the DuckDB version in its crate version:
# 1.MAJOR_MINOR_PATCH.x => vMAJOR.MINOR.PATCH (e.g. 1.10505.0 => v1.5.5)
PIN_VERSION=$(sed -n 's/^version //p' "$PINS_FILE" | head -1)
[[ -n $PIN_VERSION ]] || die "no 'version' line in $PINS_FILE"

LOCK_VERSION=$(awk '
  /^name = "libduckdb-sys"/ { in_sys = 1; next }
  in_sys && /^version = /   { gsub(/"/, ""); print $3; exit }
' "$REPO_ROOT/Cargo.lock")
[[ -n $LOCK_VERSION ]] || die "could not read libduckdb-sys version from Cargo.lock"

ENCODED=${LOCK_VERSION#*.}           # 1.10505.0 -> 10505.0
ENCODED=${ENCODED%%.*}               # -> 10505
[[ $ENCODED =~ ^[0-9]+$ ]] || die "unexpected libduckdb-sys version '$LOCK_VERSION'"
LOCK_DUCKDB="v$((ENCODED / 10000)).$((ENCODED % 10000 / 100)).$((ENCODED % 100))"

if [[ $LOCK_DUCKDB != "$PIN_VERSION" ]]; then
  die "version skew: Cargo.lock libduckdb-sys $LOCK_VERSION encodes DuckDB $LOCK_DUCKDB, but $PINS_FILE pins $PIN_VERSION. Bump the pins file (version + every hash) to match."
fi

# --- the pin for this archive ----------------------------------------------
PIN=$(awk -v arch="$ARCHIVE" '$2 == arch { print $1; exit }' "$PINS_FILE")
[[ -n $PIN ]] || die "no sha256 pin for $ARCHIVE in $PINS_FILE. Refusing to fetch an unpinned archive (SEC-37 posture)."

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1
  else shasum -a 256 "$1" | cut -d' ' -f1
  fi
}

# --- fetch + verify + extract ----------------------------------------------
VERSION=${PIN_VERSION#v}
CACHE_ROOT=${CARGO_TARGET_DIR:-$REPO_ROOT/target}/duckdb-prebuilt/$VERSION
ZIP_PATH=$CACHE_ROOT/$ARCHIVE
EXTRACT_DIR=$CACHE_ROOT/${ARCHIVE%.zip}
mkdir -p "$CACHE_ROOT"

if [[ ! -f $EXTRACT_DIR/duckdb.h ]]; then
  URL=https://github.com/duckdb/duckdb/releases/download/v$VERSION/$ARCHIVE
  if [[ -f $ZIP_PATH ]]; then
    echo "reusing cached $ZIP_PATH" >&2
  else
    echo "downloading $URL" >&2
    curl --proto '=https' --tlsv1.2 -fsSL -o "$ZIP_PATH.download" "$URL"
    mv "$ZIP_PATH.download" "$ZIP_PATH"
  fi

  ACTUAL=$(sha256_of "$ZIP_PATH")
  if [[ $ACTUAL != "$PIN" ]]; then
    rm -f "$ZIP_PATH"
    die "sha256 mismatch for $ARCHIVE: expected $PIN, got $ACTUAL. Removed the corrupt archive; re-run."
  fi
  echo "sha256 ok ($PIN)" >&2

  rm -rf "$EXTRACT_DIR"
  mkdir -p "$EXTRACT_DIR"
  unzip -q "$ZIP_PATH" -d "$EXTRACT_DIR"
else
  echo "cached extract at $EXTRACT_DIR" >&2
fi

[[ -f $EXTRACT_DIR/duckdb.h ]] || die "archive $ARCHIVE did not contain duckdb.h"

# --- exports ----------------------------------------------------------------
q() { printf '%q' "$1"; }

# Emit `export NAME=<value>` where <value> is shell-escaped as one word and
# existing NAME is preserved (ours prepended). Escaping the whole value in one
# %q — not element-wise inside double quotes, which leaves stray backslashes
# (e.g. on the commas in -Wl,--start-group).
emit_export() {
  echo "export $1=$(q "$2")\${$1:+ \${$1}}"
}

if [[ $MODE == static ]]; then
  # libduckdb-sys emits only `-l duckdb_static`; the satellite archives, the
  # C++ runtime, and archive-group ordering are on us (verified Phase 0).
  shopt -s nullglob
  ARCHIVES=("$EXTRACT_DIR"/*.a)
  [[ ${#ARCHIVES[@]} -gt 0 ]] || die "no .a archives found in $EXTRACT_DIR"

  case "$TARGET" in
    *-apple-darwin)
      # ld64 — classic and the Xcode 15+ rewrite alike — rejects
      # --start-group as an unknown option, and does not need it: it
      # resolves references between static archives on its own, so a
      # plain listing is the correct (and minimal) form. If a symbol
      # ever escapes that resolution, DUCKDB_DARWIN_FORCE_LOAD=1 links
      # every member of every archive instead (larger binary, same
      # result).
      if [[ ${DUCKDB_DARWIN_FORCE_LOAD:-} == 1 ]]; then
        FLAGS=""
        for a in "${ARCHIVES[@]}"; do FLAGS="$FLAGS -C link-arg=-Wl,-force_load,$a"; done
      else
        FLAGS=""
        for a in "${ARCHIVES[@]}"; do FLAGS="$FLAGS -C link-arg=$a"; done
      fi
      FLAGS="$FLAGS -l c++"
      ;;
    *)
      FLAGS="-C link-arg=-Wl,--start-group"
      for a in "${ARCHIVES[@]}"; do FLAGS="$FLAGS -C link-arg=$a"; done
      FLAGS="$FLAGS -C link-arg=-Wl,--end-group -l stdc++"
      ;;
  esac

  echo "export DUCKDB_LIB_DIR=$(q "$EXTRACT_DIR")"
  echo "export DUCKDB_STATIC=1"
  emit_export RUSTFLAGS "$FLAGS"
else
  # The build script emits no rpath on the DUCKDB_LIB_DIR path (only its own
  # download path gets one), so test binaries need it from us. Dev/CI only —
  # never ship a binary with this rpath (docs/duckdb-prebuilt-lib.md gotcha 2).
  echo "export DUCKDB_LIB_DIR=$(q "$EXTRACT_DIR")"
  emit_export RUSTFLAGS "-C link-arg=-Wl,-rpath,$EXTRACT_DIR"
fi

echo "# eval'd: DUCKDB_LIB_DIR=$EXTRACT_DIR mode=$MODE target=$TARGET duckdb=$PIN_VERSION" >&2
