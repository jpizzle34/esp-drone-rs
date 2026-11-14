# Wokwi simulation

Simulate **8520 brushed motors**, the **serial REPL**, **joystick mode**, and **MPU-6050 IMU** in [Wokwi](https://wokwi.com/) before flashing the Elegoo board.

Files live in [`Firmware/esp-drone-rs/`](../../Firmware/esp-drone-rs/):

| File | Purpose |
|------|---------|
| [`wokwi.toml`](../../Firmware/esp-drone-rs/wokwi.toml) | Points Wokwi at the Rust ELF + registers custom chips |
| [`diagram.json`](../../Firmware/esp-drone-rs/diagram.json) | ESP32 + 4× SI2300 + 4× DC motor (POC pins) |
| [`chips/`](../../Firmware/esp-drone-rs/chips/) | Custom Wokwi chips (`.chip.c`, `.chip.json`, `.chip.wasm`) |
| [`chips/si2300.chip.wasm`](../../Firmware/esp-drone-rs/chips/si2300.chip.wasm) | Compiled SI2300 low-side switch (required for VS Code sim) |
| [`chips/dc-motor.chip.wasm`](../../Firmware/esp-drone-rs/chips/dc-motor.chip.wasm) | Compiled 8520 DC motor (spinning prop display) |
| [`chips/dc-motor.chip.json`](../../Firmware/esp-drone-rs/chips/dc-motor.chip.json) | Motor pinout + 3.3 V supply control |
| [`chips/dc-motor.chip.c`](../../Firmware/esp-drone-rs/chips/dc-motor.chip.c) | Motor simulation source |
| [`chips/si2300.chip.json`](../../Firmware/esp-drone-rs/chips/si2300.chip.json) | SI2300 pinout (gate, drain, source) |
| [`chips/chip-diode.chip.wasm`](../../Firmware/esp-drone-rs/chips/chip-diode.chip.wasm) | 1N5819 Schottky flyback (freewheel) |
| [`chips/chip-diode.chip.c`](../../Firmware/esp-drone-rs/chips/chip-diode.chip.c) | Diode sim source ([drf5n/Wokwi-Chip-Diode](https://github.com/drf5n/Wokwi-Chip-Diode)) |

Build output goes to `Firmware/esp-drone-rs/target/` (same folder as `wokwi.toml`).

## Circuit (matches POC_LEFT_HEADER)

Each motor channel is the POC breadboard: an **SI2300** N-channel MOSFET as a low-side switch, gate tied straight to the GPIO, plus a flyback diode. The four motors are in parallel on the supply. No series resistors and no pull-downs.

```
GPIO (PWM) ───────────► SI2300 GATE
motor supply ─────────► motor +  (8520 + terminal; 3V3 POC or 1S LiPo)
                    ┌──►|── 1N5819 (band / cathode → motor +)
motor − ────────────┴──► SI2300 DRAIN
SI2300 SOURCE ────────► GND
```

SOT-23 pin order, marking facing you: pin 1 gate, pin 2 source, pin 3 drain. The Wokwi part is a digital switch, so the simulated gate still reads as a logic level.

**Flyback diode:** one **1N5819** per motor. The band (cathode) goes to **motor + / supply**, the anode to **motor − / SI2300 drain**. It clamps inductive kick when the MOSFET turns off.

| Motor | GPIO | Header | Frame corner | Wokwi part |
|-------|------|--------|--------------|------------|
| M1 | 32 | D32 | Front-right | `motor_m1` |
| M2 | 33 | D33 | Back-right | `motor_m2` |
| M3 | 25 | D25 | Back-left | `motor_m3` |
| M4 | 26 | D26 | Front-left | `motor_m4` |
| Status | 27 | D27 | — | Orange LED, cathode to GND |
| IMU SDA | 14 | D14 | — | MPU-6050 SDA |
| IMU SCL | 13 | D13 | — | MPU-6050 SCL |

Motors are custom **`chip-dc-motor`** parts (8520 coreless, 48×48 spinning prop). PWM is **15 kHz LEDC** from firmware — same as hardware.

**POC power note:** The Wokwi diagram ties motor **+** to the devkit **3V3** pin for a safe, low-torque simulation. On hardware, use **8520 @ 3.8 V** (1S LiPo) for meaningful thrust — motor **+** must not be powered from the ESP **3V3** pin on a real quad; only **GND** is common with the ESP32.

## Setup (one time)

1. Install the [Wokwi for VS Code](https://marketplace.visualstudio.com/items?itemName=wokwi.wokwi-vscode) extension.
2. Install the ESP Rust toolchain — see [README setup](../../README.md#4-install-esp-rust-tools) (`espup install`).
3. **Open the folder** `Firmware/esp-drone-rs` in VS Code (Wokwi reads `wokwi.toml` from the workspace root).

## Build and run

From `Firmware/esp-drone-rs/` (the script loads the ESP environment automatically — no manual `source` needed):

```bash
cd Firmware/esp-drone-rs
./scripts/build-for-wokwi.sh   # first time, or if chip WASM is missing
# day-to-day firmware changes:
cargo build                    # same debug ELF Wokwi uses
```

`build-for-wokwi.sh` also compiles custom Wokwi chip WASM if `wokwi-cli` is available. ELF path: `target/xtensa-esp32-espidf/debug/esp-drone-rs`.

In VS Code:

1. **Terminal → Run Build Task** (or `Ctrl+Shift+B`) — choose **Build for Wokwi** (works when the repo root or `esp-drone-rs` folder is open).
2. Command palette → **Wokwi: Start Simulator**.
3. **Stop and restart** the simulator after rebuilding so it reloads the ELF.

Keep the Wokwi tab focused so the simulation keeps running.

## Expected behaviour

1. Serial banner with pin map (`serial command mode`).
2. Orange **D27** — short blink during init.
3. `Serial command task ready — type 'help'` and `>` prompt. **Motors stay off** until commanded.
4. Orange — two quick blinks, then 500 ms heartbeat.
5. In the Wokwi serial monitor, try:
   - `arm` — M1→M4 chirp, then armed
   - `motor 2 30` — one motor (requires armed)
   - `joy on` then **E** / **W** / **A** — mix (**Q** disarm)
   - `disarm` — instant all-off
   - `imu on` / `imu off` / `imu cal` — MPU-6050 attitude stream and gyro re-zero

Full command list: [serial-commands.md](./serial-commands.md).

## LED-only diagram (optional)

Replace `chip-dc-motor` + SI2300 chains with an LED to GND on each motor pin for GPIO verification, or use per-motor serial commands (`motor 1 30`, …). See [motor-led-flash-test.md](./motor-led-flash-test.md).

## Troubleshooting

| Issue | Fix |
|-------|-----|
| **`Missing chip Breakout`** (SI2300, motor, or diode) | Custom chips need **`.chip.wasm`** + `[[chip]]` in `wokwi.toml`. Run `./scripts/build-for-wokwi.sh`, then **restart** the simulator. |
| Firmware file not found | Run `cargo build` or `./scripts/build-for-wokwi.sh`. ELF: `target/xtensa-esp32-espidf/debug/esp-drone-rs` |
| Stale firmware in sim | **Stop** Wokwi completely, rebuild, **Start Simulator** again |
| Wrong workspace root | Open `Firmware/esp-drone-rs`, not the repo root |
| `~/export-esp.sh` not found | Run `espup install`. `cargo build` and the helper scripts load it automatically |
| `LIBCLANG_PATH` / linker errors | Re-run `espup install` so `~/export-esp.sh` exists, then `cargo build` |
| Simulation hangs / boot loop | Click the Wokwi panel; check serial for panic after `NVS initialized` |
| No `>` prompt | Wait for `Serial command task ready`; click serial monitor input |
| Motors never spin | Type `arm` first, then `motor 2 30` or `joy on` |
| Wrong motor order | Compare labels M1–M4 with [poc-left-header-wiring.md](./poc-left-header-wiring.md) |
| IMU init failed | Add MPU-6050 (SDA→GPIO14, SCL→GPIO13, VCC→3V3, GND) or ignore for motor-only tests |
| Multiple motors spin together | Wiring fault in diagram — each GPIO must drive only one SI2300 gate |
| Joystick keys ignored | Type `joy on` first; use **E/C/W/A/S/D**, not command lines |

## Optional: Wokwi CLI (CI)

For automated checks you can use [wokwi-ci](https://docs.wokwi.com/vscode/ci) with a `wokwi-cli` token — not required for local development.
