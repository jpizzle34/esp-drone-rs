# Hardware documentation

Pinout and wiring for the Rust firmware (`Firmware/esp-drone-rs/`).

**Default boot behaviour:** sequential **motor spin test** (M1 → M4), then **serial command prompt** (`help`, `motor`, `imu on/off`). IMU stream is off until `imu on`. Status LED heartbeat on D27. See the [README](../../README.md#serial-commands-after-boot-bench-test) and [wokwi-simulation.md](./wokwi-simulation.md).

| Document | Description |
|----------|-------------|
| [elegoo-esp32-wroom32.md](./elegoo-esp32-wroom32.md) | Elegoo board overview, header layout, pin profiles |
| [poc-left-header-wiring.md](./poc-left-header-wiring.md) | **Active POC** — all signals on the left header |
| [wokwi-simulation.md](./wokwi-simulation.md) | **Wokwi** — simulate motor spin test + MPU-6050 in VS Code |
| [motor-led-flash-test.md](./motor-led-flash-test.md) | **Optional** — LED-only GPIO verification (swap test in `main.rs`; not the default boot test) |

## Build and flash

All Rust commands run from **`Firmware/esp-drone-rs/`**. Helper scripts in `scripts/` source the ESP toolchain automatically — see the [README](../../README.md#helper-scripts).

```bash
cd Firmware/esp-drone-rs
./scripts/flash.sh          # release — build + flash + monitor
./scripts/build-for-wokwi.sh  # debug build for Wokwi simulation
```

## Firmware source (`Firmware/esp-drone-rs/src/`)

```text
src/
├── main.rs           composition root
├── board/            BoardProfile + PeripheralPins (Elegoo POC active, ESPLANE_V1 planned)
├── drivers/
│   ├── motors/       LEDC PWM + MotorWiring + bench tests [active]
│   ├── imu/          MPU6050 I2C + ImuWiring [active]
│   └── power/        battery ADC [stub]
├── sensors/          IMU stream task (sensors::start) [active]
├── estimation/       attitude fusion [stub]
├── flight/           stabilizer, PID, mixer [stub]
├── comm/             serial bench REPL [active]
└── safety/           arming gate, failsafe [stub]
```

**Board pin maps**

- **POC (active):** [`elegoo_esp32_wroom32.rs`](../../Firmware/esp-drone-rs/src/board/elegoo_esp32_wroom32.rs) — `POC_LEFT_HEADER` metadata; GPIO claimed via [`BoardProfile::take`](../../Firmware/esp-drone-rs/src/board/profile.rs) → [`PeripheralPins::split`](../../Firmware/esp-drone-rs/src/board/profile.rs)
- **ESPLANE_V1 (planned):** [`esplane_v1.rs`](../../Firmware/esp-drone-rs/src/board/esplane_v1.rs) — GPIO constants; full [`BoardProfile`](../../Firmware/esp-drone-rs/src/board/profile.rs) not implemented yet

**Boot sequence:** [`drivers/motors/bench.rs`](../../Firmware/esp-drone-rs/src/drivers/motors/bench.rs) (`run_bench_test`) → [`comm/serial.rs`](../../Firmware/esp-drone-rs/src/comm/serial.rs) (interactive commands) + [`sensors/imu.rs`](../../Firmware/esp-drone-rs/src/sensors/imu.rs) (IMU ready, stream off until `imu on`).
