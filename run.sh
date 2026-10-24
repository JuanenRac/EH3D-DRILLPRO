#!/usr/bin/env bash
# Runs DRILLPRO. Builds it first if needed. Extra arguments are passed on:
#   ./run.sh [FILE] [--lang en|es|it|de|fr|pt|zh|ja] [--data DIR]
set -u
cd "$(dirname "$0")"
pause() { [ -n "${NOPAUSE:-}" ] || { read -r -p "Press Enter to close..." _ || true; }; }
trap pause EXIT
if [ ! -x "target/release/drillpro" ]; then
    NOPAUSE=1 ./build.sh || exit 1
fi
data=(--data "$PWD/testdata")
for a in "$@"; do [ "$a" = "--data" ] && data=(); done
"target/release/drillpro" "${data[@]}" "$@"
