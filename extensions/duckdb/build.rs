// Path A (docs/duckdb-prebuilt-lib.md, Phase 4): the workspace duckdb
// dependency no longer enables `bundled`, so libduckdb-sys links a prebuilt
// library found through DUCKDB_LIB_DIR. When that variable is unset, the
// build script's bare `-lduckdb` fallback turns into an opaque linker error
// (`mold: fatal: library not found: duckdb`) only at link time — long after
// this crate compiled fine. Print the fix where the confusion starts.
//
// `cargo check`/`clippy` pass without the variable (they never link), so a
// warning, not an error, is the right severity. Set DUCKDB_PREBUILT_QUIET=1
// to silence it — release builds do, because they re-enable `bundled` and
// link the amalgamation instead.

fn main() {
    println!("cargo:rerun-if-env-changed=DUCKDB_LIB_DIR");
    println!("cargo:rerun-if-env-changed=DUCKDB_PREBUILT_QUIET");
    if std::env::var_os("DUCKDB_LIB_DIR").is_none()
        && std::env::var_os("DUCKDB_PREBUILT_QUIET").is_none()
    {
        println!(
            "cargo:warning=DUCKDB_LIB_DIR is not set: cargo check/clippy work, \
             but linking builds (cargo build/test/install) will fail with \
             `library not found: duckdb`"
        );
        println!(
            "cargo:warning=fix: eval \"$(scripts/fetch-duckdb.sh)\" — see \
             docs/duckdb-prebuilt-lib.md"
        );
    }
}
