#!/usr/bin/env bash
# Build firmware for Wokwi simulation (ESP32 + Rust ESP-IDF).
set -euo pipefail
cd "$(dirname "$0")/.."
# shellcheck disable=SC1091
source "${HOME}/export-esp.sh"
cargo build
echo ""
echo "Wokwi: open this folder in VS Code, then run 'Wokwi: Start Simulator'"
echo "  ELF: target/xtensa-esp32-espidf/debug/esp-drone-rs"
