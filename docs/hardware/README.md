# Hardware documentation

Pinout and wiring for the Rust firmware (`Firmware/esp-drone-rs/`).

| Document | Description |
|----------|-------------|
| [elegoo-esp32-wroom32.md](./elegoo-esp32-wroom32.md) | Elegoo board overview, header layout, pin profiles |
| [poc-left-header-wiring.md](./poc-left-header-wiring.md) | **Active POC** — all signals on the left header |
| [motor-led-flash-test.md](./motor-led-flash-test.md) | **Flash test** — sequential M1–M4 LED verification on boot |
| [wokwi-simulation.md](./wokwi-simulation.md) | **Wokwi** — simulate motor LED test in VS Code before hardware |

Source of truth in code: [`Firmware/esp-drone-rs/src/board/elegoo_esp32_wroom32.rs`](../Firmware/esp-drone-rs/src/board/elegoo_esp32_wroom32.rs).
