#!/usr/bin/env bash
# Load ~/export-esp.sh, then exec the real command.
# Wired up as Cargo's rustc-wrapper and espflash runner so `cargo build` and
# `cargo run` pick up LIBCLANG_PATH and the Xtensa GCC without a separate source.
set -euo pipefail

if [[ -z "${LIBCLANG_PATH:-}" ]] || ! command -v xtensa-esp32-elf-gcc >/dev/null 2>&1; then
  if [[ ! -f "${HOME}/export-esp.sh" ]]; then
    echo "error: ${HOME}/export-esp.sh not found. Run: espup install" >&2
    exit 1
  fi
  # shellcheck disable=SC1091
  source "${HOME}/export-esp.sh" >/dev/null
fi

exec "$@"
