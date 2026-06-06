# Wokwi simulation

Simulate the **8520 brushed motor** bench test in [Wokwi](https://wokwi.com/) before flashing the Elegoo board.

Files live in [`Firmware/esp-drone-rs/`](../../Firmware/esp-drone-rs/):

| File | Purpose |
|------|---------|
| [`wokwi.toml`](../../Firmware/esp-drone-rs/wokwi.toml) | Points Wokwi at the Rust ELF + registers custom chips |
| [`diagram.json`](../../Firmware/esp-drone-rs/diagram.json) | ESP32 + 4× MOSFET + 4× DC motor (POC pins) |
| [`mosfet-n.chip.wasm`](../../Firmware/esp-drone-rs/mosfet-n.chip.wasm) | Compiled MOSFET driver (required for VS Code sim) |
| [`dc-motor.chip.wasm`](../../Firmware/esp-drone-rs/dc-motor.chip.wasm) | Compiled 8520 DC motor (spinning prop display) |
| [`dc-motor.chip.json`](../../Firmware/esp-drone-rs/dc-motor.chip.json) | Motor pinout + 3.3 V supply control |
| [`dc-motor.chip.c`](../../Firmware/esp-drone-rs/dc-motor.chip.c) | Motor simulation source |
| [`mosfet-n.chip.json`](../../Firmware/esp-drone-rs/mosfet-n.chip.json) | MOSFET pinout |
| [`chip-diode.chip.wasm`](../../Firmware/esp-drone-rs/chip-diode.chip.wasm) | Schottky flyback diode (freewheel) |
| [`chip-diode.chip.c`](../../Firmware/esp-drone-rs/chip-diode.chip.c) | Diode sim source ([drf5n/Wokwi-Chip-Diode](https://github.com/drf5n/Wokwi-Chip-Diode)) |

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

**POC power note:** The diagram ties motor **+** to the devkit **3V3** pin to match the firmware spin-test assumption (8520 @ 3.3 V). On a real build, motor **+** comes from **1S LiPo / VIN**; only **GND** must be common with the ESP32.

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

Matches the firmware `run_sequential_spin_test()` in [`motors/mod.rs`](../../Firmware/esp-drone-rs/src/motors/mod.rs):

1. Serial banner with pin map and `Motor spin test begin`.
2. Orange **D27** — short blink before the test.
3. **One motor spins at a time** (~200 ms at ~15% PWM, ~1 s gap), order **M1 → M2 → M3 → M4**.
4. Orange — two quick blinks, then 500 ms heartbeat.
5. Serial logs each step, e.g. `Spin 1/4: M1 GPIO32 (D32) — front-right`.

## LED-only diagram (optional)

The previous LED-only bench diagram is equivalent for **GPIO verification** — replace `chip-dc-motor` + MOSFET chains with LED + 330 Ω to GND on each motor pin. The motor diagram is preferred when validating spin direction and sequential timing.

## Troubleshooting

| Issue | Fix |
|-------|-----|
| **`Missing chip Breakout`** (MOSFET, motor, or diode) | Custom chips need **`.chip.wasm`** + `[[chip]]` in `wokwi.toml`. Need `mosfet-n`, `dc-motor`, and `chip-diode` WASM files. Run `./scripts/build-for-wokwi.sh`, then **restart** the simulator. |
| Firmware file not found | Run `./scripts/build-for-wokwi.sh` first |
| Wrong workspace root | Open `Firmware/esp-drone-rs`, not the repo root |
| `export-esp.sh` not found | Run `espup install`, then `source ~/export-esp.sh` |
| Simulation hangs / no serial | Click the Wokwi panel; check build succeeded |
| Motors never spin | Confirm ELF built; check serial for `Motor spin test begin` |
| Wrong motor order | Compare labels M1–M4 with [`poc-left-header-wiring.md`](./poc-left-header-wiring.md) |
| Multiple motors spin together | Wiring fault in diagram — each GPIO must drive only one MOSFET gate |

## Optional: Wokwi CLI (CI)

For automated checks you can use [wokwi-ci](https://docs.wokwi.com/vscode/ci) with a `wokwi-cli` token — not required for local development.
