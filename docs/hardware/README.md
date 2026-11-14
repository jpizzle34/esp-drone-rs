# Hardware documentation

Pinout and wiring for the Rust firmware (`Firmware/esp-drone-rs/`).

**Default boot behaviour:** **disarmed** serial REPL (`arm` / `disarm`, `motor`, `joy on` when armed, `imu on/off`). See [serial-commands.md](./serial-commands.md).

| Document | Description |
|----------|-------------|
| [serial-commands.md](./serial-commands.md) | **UART REPL** — motor commands, joystick mode, IMU stream |
| [elegoo-esp32-wroom32.md](./elegoo-esp32-wroom32.md) | Elegoo board overview, header layout, pin profiles |
| [poc-left-header-wiring.md](./poc-left-header-wiring.md) | **Active POC** — all signals on the left header |
| [wokwi-simulation.md](./wokwi-simulation.md) | **Wokwi** — simulate motors + MPU-6050 in VS Code |
| [motor-led-flash-test.md](./motor-led-flash-test.md) | **Optional** — LED GPIO checks and sequential bench tests |
| [IMU live dashboard](../../tools/imu-dashboard/README.md) | Browser graphs for the serial IMU stream (`tools/imu-dashboard`) |

## Build and flash

All Rust commands run from **`Firmware/esp-drone-rs/`**. `cargo build` / `cargo run` and the helper scripts in `scripts/` load the ESP toolchain automatically — see the [README](../../README.md#helper-scripts).

```bash
cd Firmware/esp-drone-rs
./scripts/flash.sh          # release — build + flash + monitor
./scripts/build-for-wokwi.sh  # debug build for Wokwi (or use `cargo build`)
```

## Firmware source (`Firmware/esp-drone-rs/src/`)

```text
src/
├── main.rs           composition root
├── board/            BoardProfile + PeripheralPins (Elegoo POC active, ESPLANE_V1 planned)
├── drivers/
│   ├── motors/       LEDC PWM + MotorWiring + optional bench tests [active]
│   ├── imu/          MPU6050 I2C + ImuWiring [active]
│   └── power/        battery ADC [stub]
├── sensors/          IMU sample task, gyro bias (sensors::init) [active]
├── estimation/       Mahony attitude [active]
├── flight/           stabilizer, PID, mixer [stub]
├── comm/             serial REPL + joystick [active]
└── safety/           MotorGate arming [active]
```

**Board pin maps**

- **POC (active):** [`elegoo_esp32_wroom32.rs`](../../Firmware/esp-drone-rs/src/board/elegoo_esp32_wroom32.rs) — `POC_LEFT_HEADER` metadata; GPIO claimed via [`BoardProfile::take`](../../Firmware/esp-drone-rs/src/board/profile.rs) → [`PeripheralPins::split`](../../Firmware/esp-drone-rs/src/board/profile.rs)
- **ESPLANE_V1 (planned):** [`esplane_v1.rs`](../../Firmware/esp-drone-rs/src/board/esplane_v1.rs) — GPIO constants; full [`BoardProfile`](../../Firmware/esp-drone-rs/src/board/profile.rs) not implemented yet

**Boot sequence:** init motors (off, disarmed) + IMU → serial REPL → status LED heartbeat. All PWM via [`safety/motor_gate.rs`](../../Firmware/esp-drone-rs/src/safety/motor_gate.rs).
