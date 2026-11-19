#!/usr/bin/env bash
# vendor-loom.sh <run-dir>
#
# One-time vendoring of loom = "=0.7.3" into <run-dir>/vendor/ together with a
# <run-dir>/.cargo/config.toml source replacement. Afterwards the dummy
# project builds and tests fully offline:
#
#   cargo test --features loom-test --offline
#
# Network is used exactly once, inside this script (cargo vendor). Prints
# VENDOR_OK on success.
#
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "usage: $0 <run-dir>" >&2
    exit 2
fi

mkdir -p "$1"
run_dir="$(cd "$1" && pwd)"
vendor_dir="$run_dir/vendor"
config_dir="$run_dir/.cargo"
config_toml="$config_dir/config.toml"

if [ -e "$vendor_dir" ] || [ -e "$config_toml" ]; then
    echo "refusing to overwrite existing $vendor_dir or $config_toml" >&2
    exit 1
fi

# --- temporary manifest: sole dependency is loom = "=0.7.3" ----------------
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$tmp/src"
cat > "$tmp/Cargo.toml" <<'TOML'
[package]
name = "loom-vendor-source"
version = "0.0.0"
edition = "2021"

[dependencies]
loom = "=0.7.3"
TOML

cat > "$tmp/src/lib.rs" <<'RS'
//! Manifest used only to vendor loom = "=0.7.3".
RS

# --- vendor (network used once, here) --------------------------------------
cargo vendor --manifest-path "$tmp/Cargo.toml" "$vendor_dir"

# --- source replacement config ---------------------------------------------
mkdir -p "$config_dir"
cat > "$config_toml" <<TOML
# One-time source replacement for vendored loom (Task 4).
# Enables: cargo test --features loom-test --offline
[source.crates-io]
replace-with = "vendored-loom"

[source.vendored-loom]
directory = "vendor"
TOML

# --- prove offline operation on a minimal crate ----------------------------
proof_dir="$run_dir/.loom-proof"
mkdir -p "$proof_dir/src"

cat > "$proof_dir/Cargo.toml" <<TOML
[package]
name = "loom-proof"
version = "0.0.0"
edition = "2021"

[dependencies]
loom = "=0.7.3"

[features]
loom-test = []
TOML

cat > "$proof_dir/src/lib.rs" <<'RS'
//! Proof that vendored loom builds and runs with feature `loom-test` offline.

#[cfg(feature = "loom-test")]
use loom::sync::atomic::{AtomicUsize, Ordering};
#[cfg(feature = "loom-test")]
use std::sync::Arc;

#[cfg(feature = "loom-test")]
#[test]
fn vendored_loom_explores_schedules() {
    loom::model(|| {
        let flag = Arc::new(AtomicUsize::new(0));
        let t = loom::thread::spawn({
            let flag = Arc::clone(&flag);
            move || {
                flag.store(1, Ordering::Release);
            }
        });
        let _ = t.join();
        assert_eq!(flag.load(Ordering::Acquire), 1);
    });
}
RS

(
    cd "$proof_dir"
    LOOM_MAX_BRANCHES=4096 timeout 60 cargo test --features loom-test --offline
)

rm -rf "$proof_dir"

echo "VENDOR_OK"
