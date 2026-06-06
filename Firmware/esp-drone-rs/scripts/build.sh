#!/usr/bin/env bash
# Compile firmware only (ESP32 + Rust ESP-IDF).
set -euo pipefail
cd "$(dirname "$0")/.."

which idf.py >/dev/null || {
    # shellcheck disable=SC1091
    source "${HOME}/export-esp.sh" >/dev/null 2>&1
}

case "${1:-}" in
"" | "release")
    cargo build --release
    ;;
"debug")
    cargo build
    ;;
*)
    echo 'Wrong argument. Only "debug" or "release" are supported'
    exit 1
    ;;
esac
