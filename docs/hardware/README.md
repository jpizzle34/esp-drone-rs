# Hardware documentation

Pinout and wiring for the Rust firmware (`Firmware/esp-drone-rs/`).

**Default boot behaviour:** sequential **motor spin test** (M1 → M4, one motor at a time) plus a status LED heartbeat on D27. See the [README](../../README.md#expected-output) and [wokwi-simulation.md](./wokwi-simulation.md).

| Document | Description |
|----------|-------------|
| [elegoo-esp32-wroom32.md](./elegoo-esp32-wroom32.md) | Elegoo board overview, header layout, pin profiles |
| [poc-left-header-wiring.md](./poc-left-header-wiring.md) | **Active POC** — all signals on the left header |
| [wokwi-simulation.md](./wokwi-simulation.md) | **Wokwi** — simulate the 8520 motor spin test (MOSFET + DC motor) in VS Code |
| [motor-led-flash-test.md](./motor-led-flash-test.md) | **Optional** — LED-only GPIO verification (swap test in `main.rs`; not the default boot test) |

## Build and flash

All Rust commands run from **`Firmware/esp-drone-rs/`**. Helper scripts in `scripts/` source the ESP toolchain automatically — see the [README](../../README.md#helper-scripts).

```bash
cd Firmware/esp-drone-rs
./scripts/flash.sh          # release — build + flash + monitor
./scripts/build-for-wokwi.sh  # debug build for Wokwi simulation
```

Source of truth in code: [`elegoo_esp32_wroom32.rs`](../../Firmware/esp-drone-rs/src/board/elegoo_esp32_wroom32.rs) (pins) and [`motors/mod.rs`](../../Firmware/esp-drone-rs/src/motors/mod.rs) (boot tests).
