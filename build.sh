#!/usr/bin/env bash
# Builds DRILLPRO (release, with the graphical front end).
set -u
cd "$(dirname "$0")"
pause() { [ -n "${NOPAUSE:-}" ] || { read -r -p "Press Enter to close..." _ || true; }; }
trap pause EXIT
if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust is not installed. Get it from https://rustup.rs"
    exit 1
fi
if cargo build --release --features gui; then
    echo
    echo "Build OK: target/release/drillpro"
else
    echo
    echo "Build FAILED."
    exit 1
fi
