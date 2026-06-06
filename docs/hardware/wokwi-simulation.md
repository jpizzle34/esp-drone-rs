# Wokwi simulation

Simulate the motor LED bench test in [Wokwi](https://wokwi.com/) before flashing the Elegoo board.

Files live in [`Firmware/esp-drone-rs/`](../../Firmware/esp-drone-rs/):

| File | Purpose |
|------|---------|
| [`wokwi.toml`](../../Firmware/esp-drone-rs/wokwi.toml) | Points Wokwi at the Rust ELF |
| [`diagram.json`](../../Firmware/esp-drone-rs/diagram.json) | ESP32-DevKitC + 5 LEDs on POC pins |

## Circuit (matches POC_LEFT_HEADER)

| LED | GPIO | Motor / role |
|-----|------|----------------|
| Red | 32 | M1 front-right |
| Green | 33 | M2 back-right |
| Blue | 25 | M3 back-left |
| Yellow | 26 | M4 front-left |
| Orange | 27 | Status (blinks before/after test) |

Each LED has a 330 Ω resistor to GND in the diagram.

## Setup (one time)

1. Install the [Wokwi for VS Code](https://marketplace.visualstudio.com/items?itemName=wokwi.wokwi-vscode) extension.
2. Install the ESP Rust toolchain (`espup`) if not already — see Phase 0 in the project plan.
3. **Open the folder** `Firmware/esp-drone-rs` in VS Code (Wokwi reads `wokwi.toml` from the workspace root).

## Build and run

```bash
cd Firmware/esp-drone-rs
./scripts/build-for-wokwi.sh
```

In VS Code:

1. **Terminal → Run Build Task** (or `Ctrl+Shift+B`) — runs the same build script.
2. Command palette → **Wokwi: Start Simulator**.

Keep the Wokwi tab focused so the simulation keeps running.

## Expected behaviour

Same as [motor-led-flash-test.md](./motor-led-flash-test.md):

1. Serial log with pin map and `Motor LED test begin`.
2. Orange **D27** — short blink, then motor test.
3. **One** coloured LED at a time (~800 ms), order: red → green → blue → yellow (M1→M4).
4. Orange — two quick blinks, then 500 ms heartbeat.
5. Serial: `Motor LED test complete`.

## Troubleshooting

| Issue | Fix |
|-------|-----|
| Firmware file not found | Run `./scripts/build-for-wokwi.sh` first |
| Wrong workspace root | Open `Firmware/esp-drone-rs`, not the repo root |
| `export-esp.sh` not found | Run `espup install`, then `source ~/export-esp.sh` |
| Simulation hangs / no serial | Click the Wokwi panel; check build succeeded |
| LEDs always off | Confirm ELF path in `wokwi.toml` matches `target/xtensa-esp32-espidf/debug/esp-drone-rs` |

## Optional: Wokwi CLI (CI)

For automated checks you can use [wokwi-ci](https://docs.wokwi.com/vscode/ci) with a `wokwi-cli` token — not required for local development.
