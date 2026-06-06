# Hardware documentation

Pinout and wiring for the Rust firmware (`Firmware/esp-drone-rs/`).

| Document | Description |
|----------|-------------|
| [elegoo-esp32-wroom32.md](./elegoo-esp32-wroom32.md) | Elegoo board overview, header layout, pin profiles |
| [poc-left-header-wiring.md](./poc-left-header-wiring.md) | **Active POC** — all signals on the left header |
| [motor-led-flash-test.md](./motor-led-flash-test.md) | **Flash test** — sequential M1–M4 LED verification on boot |
| [wokwi-simulation.md](./wokwi-simulation.md) | **Wokwi** — simulate 8520 motor spin test (MOSFET + DC motor) in VS Code |

## Build and flash

All Rust commands run from **`Firmware/esp-drone-rs/`**. Helper scripts in `scripts/` source the ESP toolchain automatically — see the [README](../../README.md#helper-scripts).

```bash
cd Firmware/esp-drone-rs
./scripts/flash.sh          # release — build + flash + monitor
./scripts/build-for-wokwi.sh  # debug build for Wokwi simulation
```

Source of truth in code: [`Firmware/esp-drone-rs/src/board/elegoo_esp32_wroom32.rs`](../Firmware/esp-drone-rs/src/board/elegoo_esp32_wroom32.rs).
