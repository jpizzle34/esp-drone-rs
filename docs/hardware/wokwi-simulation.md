# Wokwi simulation

Simulate the **8520 brushed motor** bench test in [Wokwi](https://wokwi.com/) before flashing the Elegoo board.

Files live in [`Firmware/esp-drone-rs/`](../../Firmware/esp-drone-rs/):

| File | Purpose |
|------|---------|
| [`wokwi.toml`](../../Firmware/esp-drone-rs/wokwi.toml) | Points Wokwi at the Rust ELF + registers custom chips |
| [`diagram.json`](../../Firmware/esp-drone-rs/diagram.json) | ESP32 + 4× MOSFET + 4× DC motor (POC pins) |
| [`chips/`](../../Firmware/esp-drone-rs/chips/) | Custom Wokwi chips (`.chip.c`, `.chip.json`, `.chip.wasm`) |
| [`chips/mosfet-n.chip.wasm`](../../Firmware/esp-drone-rs/chips/mosfet-n.chip.wasm) | Compiled MOSFET driver (required for VS Code sim) |
| [`chips/dc-motor.chip.wasm`](../../Firmware/esp-drone-rs/chips/dc-motor.chip.wasm) | Compiled 8520 DC motor (spinning prop display) |
| [`chips/dc-motor.chip.json`](../../Firmware/esp-drone-rs/chips/dc-motor.chip.json) | Motor pinout + 3.3 V supply control |
| [`chips/dc-motor.chip.c`](../../Firmware/esp-drone-rs/chips/dc-motor.chip.c) | Motor simulation source |
| [`chips/mosfet-n.chip.json`](../../Firmware/esp-drone-rs/chips/mosfet-n.chip.json) | MOSFET pinout |
| [`chips/chip-diode.chip.wasm`](../../Firmware/esp-drone-rs/chips/chip-diode.chip.wasm) | Schottky flyback diode (freewheel) |
| [`chips/chip-diode.chip.c`](../../Firmware/esp-drone-rs/chips/chip-diode.chip.c) | Diode sim source ([drf5n/Wokwi-Chip-Diode](https://github.com/drf5n/Wokwi-Chip-Diode)) |

Build output goes to `Firmware/esp-drone-rs/target/` (same folder as `wokwi.toml`).

## Circuit (matches POC_LEFT_HEADER)

Each motor channel mirrors ESP-Drone / ESP-FLY hardware (low-side N-MOSFET + flyback diode):

```
GPIO (PWM) ──[220 Ω]──► MOSFET GATE ──[10 kΩ]──► GND
motor supply ─────────► motor +  (8520 + terminal; 3V3 POC or 1S LiPo)
                    ┌──►|── Schottky flyback (cathode → motor +)
motor − ────────────┴──► MOSFET DRAIN
MOSFET SOURCE ────────► GND
```

**Flyback diode:** cathode to **motor + / supply**, anode to **motor − / MOSFET drain**. Clamps inductive kick when the FET turns off. Use a **Schottky** (e.g. SS14, 1N5819) on breadboard builds — one per motor.

| Motor | GPIO | Header | Frame corner | Wokwi part |
|-------|------|--------|--------------|------------|
| M1 | 32 | D32 | Front-right | `motor_m1` |
| M2 | 33 | D33 | Back-right | `motor_m2` |
| M3 | 25 | D25 | Back-left | `motor_m3` |
| M4 | 26 | D26 | Front-left | `motor_m4` |
| Status | 27 | D27 | — | Orange LED + 330 Ω |

Motors are custom **`chip-dc-motor`** parts (8520 coreless, 48×48 spinning prop). PWM is **15 kHz LEDC** from firmware — same as hardware.

**POC power note:** The Wokwi diagram ties motor **+** to the devkit **3V3** pin for a safe, low-torque simulation. On hardware, the spin test is tuned for **8520 @ 3.8 V** (1S LiPo) with ~**3.2 V** average at ~84% PWM — see [`drivers/motors/bench.rs`](../../Firmware/esp-drone-rs/src/drivers/motors/bench.rs). Motor **+** must not be powered from the ESP **3V3** pin on a real quad; only **GND** is common with the ESP32.

## Setup (one time)

1. Install the [Wokwi for VS Code](https://marketplace.visualstudio.com/items?itemName=wokwi.wokwi-vscode) extension.
2. Install the ESP Rust toolchain — see [README setup](../../README.md#4-install-esp-rust-tools) (`espup install`).
3. **Open the folder** `Firmware/esp-drone-rs` in VS Code (Wokwi reads `wokwi.toml` from the workspace root).

## Build and run

From `Firmware/esp-drone-rs/` (the script loads the ESP environment automatically — no manual `source` needed):

```bash
cd Firmware/esp-drone-rs
./scripts/build-for-wokwi.sh
```

This runs a **debug** build and compiles custom Wokwi chip WASM files if `wokwi-cli` is available.

In VS Code:

1. **Terminal → Run Build Task** (or `Ctrl+Shift+B`) — choose **Build for Wokwi** (works when the repo root or `esp-drone-rs` folder is open).
2. Command palette → **Wokwi: Start Simulator**.

Keep the Wokwi tab focused so the simulation keeps running.

## Expected behaviour

Matches the firmware `run_bench_test(..., BenchMode::Spin)` in [`drivers/motors/bench.rs`](../../Firmware/esp-drone-rs/src/drivers/motors/bench.rs):

1. Serial banner with pin map and `=== Motor spin test begin ===`.
2. Orange **D27** — short blink before the test.
3. **One motor spins at a time** (1 s pulse at ~**84%** PWM / ~3.2 V avg @ 3.8 V supply, 1 s gap), order **M1 → M2 → M3 → M4**.
4. Orange — two quick blinks, then 500 ms heartbeat.
5. Serial logs each step, e.g. `Spin 1/4: M1 GPIO32 (D32) — front-right`.

## LED-only diagram (optional)

The previous LED-only bench diagram is equivalent for **GPIO verification** — replace `chip-dc-motor` + MOSFET chains with LED + 330 Ω to GND on each motor pin. The motor diagram is preferred when validating spin direction and sequential timing.

## Troubleshooting

| Issue | Fix |
|-------|-----|
| **`Missing chip Breakout`** (MOSFET, motor, or diode) | Custom chips need **`.chip.wasm`** + `[[chip]]` in `wokwi.toml`. Need `mosfet-n`, `dc-motor`, and `chip-diode` WASM files. Run `./scripts/build-for-wokwi.sh`, then **restart** the simulator. |
| Firmware file not found | Run `./scripts/build-for-wokwi.sh` first. ELF path: `target/xtensa-esp32-espidf/debug/esp-drone-rs` |
| Wrong workspace root | Open `Firmware/esp-drone-rs`, not the repo root (Wokwi resolves paths relative to the VS Code workspace folder) |
| `~/export-esp.sh` not found | Run `espup install`. Scripts source it automatically; or `source ~/export-esp.sh` before manual `cargo` |
| `LIBCLANG_PATH` / linker errors | ESP env not loaded — use `./scripts/build-for-wokwi.sh` instead of raw `cargo build` |
| Simulation hangs / no serial | Click the Wokwi panel; check build succeeded |
| Motors never spin | Confirm ELF built; check serial for `Motor spin test begin` |
| Wrong motor order | Compare labels M1–M4 with [`poc-left-header-wiring.md`](./poc-left-header-wiring.md) |
| Multiple motors spin together | Wiring fault in diagram — each GPIO must drive only one MOSFET gate |

## Optional: Wokwi CLI (CI)

For automated checks you can use [wokwi-ci](https://docs.wokwi.com/vscode/ci) with a `wokwi-cli` token — not required for local development.
