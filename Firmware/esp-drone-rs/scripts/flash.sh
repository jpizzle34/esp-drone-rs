#!/usr/bin/env bash
# Build, flash, and monitor on hardware (ESP32 + Rust ESP-IDF).
set -euo pipefail
cd "$(dirname "$0")/.."
# shellcheck disable=SC1091
source "${HOME}/export-esp.sh"
cargo run "$@"
