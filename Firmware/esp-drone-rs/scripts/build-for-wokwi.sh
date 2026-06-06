#!/usr/bin/env bash
# Build firmware + custom Wokwi chips for simulation (ESP32 + Rust ESP-IDF).
set -euo pipefail
cd "$(dirname "$0")/.."

WOKWI_CLI="${WOKWI_CLI:-wokwi-cli}"
if ! command -v "${WOKWI_CLI}" >/dev/null 2>&1; then
  if [[ -x /tmp/wokwi-cli ]]; then
    WOKWI_CLI=/tmp/wokwi-cli
  fi
fi

build_chip() {
  local src=$1
  local wasm=$2
  if [[ -f "${wasm}" ]]; then
    return 0
  fi
  if command -v "${WOKWI_CLI}" >/dev/null 2>&1; then
    echo "Building ${wasm} (one-time; needs wokwi-cli)..."
    "${WOKWI_CLI}" chip compile "${src}" -o "${wasm}"
  else
    echo "warning: ${wasm} missing and wokwi-cli not found." >&2
    echo "  Install: https://github.com/wokwi/wokwi-cli/releases" >&2
    echo "  Wokwi sim will show 'Missing chip' until WASM is built." >&2
    return 1
  fi
}

build_chip chips/mosfet-n.chip.c chips/mosfet-n.chip.wasm || true
build_chip chips/dc-motor.chip.c chips/dc-motor.chip.wasm || true
build_chip chips/chip-diode.chip.c chips/chip-diode.chip.wasm || true

bash scripts/build.sh debug
echo ""
echo "Wokwi: open this folder in VS Code, then run 'Wokwi: Start Simulator'"
echo "  ELF: target/xtensa-esp32-espidf/debug/esp-drone-rs"
echo "  Chips: mosfet-n, dc-motor, chip-diode (see wokwi.toml [[chip]])"
