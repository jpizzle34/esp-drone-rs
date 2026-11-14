# esp-drone-rs

Rust firmware for an ESP32 quadcopter, built on [ESP-IDF](https://docs.espressif.com/projects/esp-idf/) via the [esp-rs](https://github.com/esp-rs) toolchain. This repo is a fork of the [ESP-FLY](https://www.seeedstudio.com/) / ESP-Drone ecosystem; active development is the Rust crate in `Firmware/esp-drone-rs/`. The original C firmware in `Firmware/esp-drone/` is kept for reference only.

**Current firmware:** boots **disarmed** to a serial REPL, **arming gate** (`arm` / `disarm`), joystick mode (`joy on` when armed), IMU ready with stream **off by default**, and attitude mode (`att on` when armed — thrust stays 0 until `att thrust`). Status LED on D27. Commands: [docs/hardware/serial-commands.md](docs/hardware/serial-commands.md).

---

## What you need

| Item                          | Required for      | Notes                                                                          |
| ----------------------------- | ----------------- | ------------------------------------------------------------------------------ |
| **Linux or WSL2**             | Build & flash     | Recommended. Native macOS also works. Windows without WSL is not covered here. |
| **Git**                       | Clone             |                                                                                |
| **Rust** (`rustup`)           | Build             | Installed automatically when you run `espup`                                   |
| **USB cable + ESP32 board**   | Flash to hardware | Elegoo ESP32-WROOM-32 (or similar) for the current POC pinout                  |
| **VS Code + Wokwi extension** | Simulation only   | Optional — test firmware without hardware                                      |

No drone frame or motors are required to **build** the project. You can exercise motors and the IMU in the [Wokwi simulator](docs/hardware/wokwi-simulation.md) first.

---

## Quick overview

```
Co-Create_ESP-FLY/          ← clone the repo here (any folder name is fine)
├── Firmware/
│   ├── esp-drone-rs/       ← **Rust firmware — build and flash from here**
│   │   ├── src/
│   │   │   ├── main.rs     ← composition root (boot, serial REPL, IMU, idle loop)
│   │   │   ├── board/      ← pin profiles ([`BoardProfile`](Firmware/esp-drone-rs/src/board/profile.rs), [`PeripheralPins`](Firmware/esp-drone-rs/src/board/profile.rs))
│   │   │   ├── drivers/    ← hardware drivers (motors + IMU active; power stub)
│   │   │   ├── sensors/    ← IMU sample task (active)
│   │   │   ├── estimation/ ← Mahony attitude (gyro + accel)
│   │   │   ├── flight/     ← attitude PID, X mixer, 250 Hz stabilizer
│   │   │   ├── comm/       ← serial REPL + joystick mode (active)
│   │   │   └── safety/     ← arming gate, MotorGate (active)
│   │   ├── .cargo/config.toml
│   │   ├── rust-toolchain.toml
│   │   ├── Cargo.toml
│   │   ├── chips/          ← Wokwi custom chips (.chip.c/.json/.wasm)
│   │   ├── target/         ← build output (after first build)
│   │   ├── .embuild/       ← ESP-IDF cache (after first build)
│   │   └── scripts/        ← build.sh, flash.sh, build-for-wokwi.sh
│   └── esp-drone/          ← legacy C firmware (reference)
└── docs/hardware/          ← pinout, wiring, Wokwi, flash-test docs
```

All Rust tooling (`target/`, `.embuild/`, `Cargo.lock`) lives inside `Firmware/esp-drone-rs/`.

---

## Step-by-step: get running locally

### 1. Clone the repository

```bash
git clone https://github.com/jpizzle34/esp-drone-rs.git
cd esp-drone-rs/Firmware/esp-drone-rs
```

If you use SSH:

```bash
git clone git@github.com:jpizzle34/esp-drone-rs.git
cd esp-drone-rs/Firmware/esp-drone-rs
```

### 2. Install system packages (Linux / WSL2)

On **Ubuntu / Debian / WSL2**:

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  git \
  curl \
  pkg-config \
  libssl-dev \
  cmake \
  ninja-build \
  python3 \
  python3-venv
```

These are needed to compile ESP-IDF (downloaded automatically on first build).

### 3. Install Rust (if you do not have it)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Follow the prompts, then reload your shell (or run `source "$HOME/.cargo/env"`).

Verify:

```bash
rustc --version
cargo --version
```

### 4. Install ESP Rust tools

```bash
cargo install espup espflash ldproxy
```

Then install the ESP32 Rust toolchain (this downloads several hundred MB and may take several minutes):

```bash
espup install
```

That creates `~/export-esp.sh`, which sets `LIBCLANG_PATH` and adds the Xtensa GCC linker to `PATH`.

You do not need to `source` it yourself. `cargo build` and `cargo run` load it automatically (see `scripts/esp-env.sh` in [`.cargo/config.toml`](Firmware/esp-drone-rs/.cargo/config.toml)). The helper scripts do the same.

If `~/export-esp.sh` is missing, re-run `espup install`.

### 5. First build

From **`Firmware/esp-drone-rs/`**:

```bash
./scripts/build.sh
```

The first build is slow (often **15–30+ minutes**): it downloads ESP-IDF **v5.5.3** into `.embuild/` and compiles it. Later builds are much faster.

On success you will have:

```
target/xtensa-esp32-espidf/release/esp-drone-rs
```

That ELF is the firmware binary. `./scripts/build.sh` defaults to **release**; use `./scripts/build.sh debug` for a debug build.

### Helper scripts

All scripts live in `Firmware/esp-drone-rs/scripts/` and `cd` to the crate root automatically. `cargo build` and `cargo run` load `~/export-esp.sh` on their own; the helper scripts do too.

| Script | Command | What it does |
| ------ | ------- | ------------ |
| `build.sh` | `./scripts/build.sh` | `cargo build --release` (compile only) |
| `build.sh` | `./scripts/build.sh debug` | `cargo build` (debug, compile only) |
| `flash.sh` | `./scripts/flash.sh` | `cargo run --release` (build + flash + monitor) |
| `flash.sh` | `./scripts/flash.sh debug` | `cargo run` (debug, build + flash + monitor) |
| `build-for-wokwi.sh` | `./scripts/build-for-wokwi.sh` | Debug build + compile custom Wokwi chips |

`flash.sh` delegates to `cargo run`. Flashing uses the `espflash` runner configured in [`.cargo/config.toml`](Firmware/esp-drone-rs/.cargo/config.toml).

---

## Option A — Simulate in Wokwi (no hardware)

Good first step if you do not have a board yet.

1. Install [VS Code](https://code.visualstudio.com/) and the [Wokwi for VS Code](https://marketplace.visualstudio.com/items?itemName=wokwi.wokwi-vscode) extension.
2. Open the folder **`Firmware/esp-drone-rs`** in VS Code (Wokwi reads `wokwi.toml` from the workspace root).
3. Build from a terminal:

    ```bash
    cd Firmware/esp-drone-rs
    ./scripts/build-for-wokwi.sh
    ```

4. In VS Code: Command Palette → **Wokwi: Start Simulator**.

You should see serial logs, a `>` prompt, and motors respond to commands (e.g. `motor 2 30` or `joy on`). Full details: [docs/hardware/wokwi-simulation.md](docs/hardware/wokwi-simulation.md) and [docs/hardware/serial-commands.md](docs/hardware/serial-commands.md).

---

## Option B — Flash to real hardware

### Hardware (current POC)

Development target: **Elegoo ESP32-WROOM-32** with the `POC_LEFT_HEADER` pin profile — motors on **D32, D33, D25, D26**, status LED on **D27**. See [docs/hardware/elegoo-esp32-wroom32.md](docs/hardware/elegoo-esp32-wroom32.md) and [docs/hardware/poc-left-header-wiring.md](docs/hardware/poc-left-header-wiring.md).

Connect the board over USB. On Linux/WSL2, your user usually needs access to the serial port:

```bash
sudo usermod -aG dialout "$USER"
# log out and back in (or reboot WSL) for this to take effect
```

Find the port (often `/dev/ttyUSB0` for CP2102 boards):

```bash
ls /dev/ttyUSB* /dev/ttyACM* 2>/dev/null
```

### Build, flash, and monitor

From **`Firmware/esp-drone-rs/`** (no manual `source` required):

```bash
./scripts/flash.sh          # release — build + flash + serial monitor
./scripts/flash.sh debug      # debug
```

Equivalent using `cargo` directly (`~/export-esp.sh` is loaded for you):

```bash
cargo run --release
cargo run
```

### Expected output

After reset, the serial monitor should show a pin map, peripheral init, IMU ready (if wired), and the serial REPL (**disarmed**). Motors stay off until `arm`. Example:

```
I (...) esp_drone_rs: ESP-Drone Rust firmware — serial command mode (disarmed)
...
I (...) Serial command task ready — disarmed; type 'help'
Commands:
  help                       Show this help
  arm                        Arm motors (M1→M4 chirp, then output enabled)
  disarm                     Disarm — all motors off immediately
  ...
> motor 2 30
ERR disarmed — type 'arm' first
> arm
OK armed — motor output enabled
> motor 2 30
OK M2 duty 30%
> imu on
OK IMU stream on (10 Hz)
IMU accel=(+0.01,-0.02,+1.00)g gyro=(+0,+1,-0)°/s roll=+1.2° pitch=-0.5° yaw=+0.0° temp=28.3°C
> imu off
OK IMU stream off
...
I (...) Idle — status LED heartbeat on D27 / GPIO27
```

### Serial commands

Type commands in the same monitor used by `./scripts/flash.sh` (**115200 baud**). A `>` prompt is printed in command mode.

**Full reference:** [docs/hardware/serial-commands.md](docs/hardware/serial-commands.md) (also type `help` on the device).

| Command | Action |
|---------|--------|
| `help` | List commands |
| `arm` | Arm motors (M1→M4 chirp, then output enabled) |
| `disarm` / `stop` | Disarm — all motors off immediately |
| `motor …` | Per-motor / all / pulse duty (**armed only**) |
| `joy on` | Joystick — **E/C** + **WASD** (**armed**; **Q** disarm) |
| `joy off` | Disarm |
| `imu` / `imu on` / `imu off` / `imu cal` | Attitude, stream control, gyro re-zero |

IMU data uses plain `println!` lines (no log prefix) only while `imu on` is active, so the monitor stays readable for typing.

If the MPU-6050 is not connected, motor commands still work when armed; `imu` reports not available.

For **LED-only** GPIO checks or longer bench sequences, see [docs/hardware/motor-led-flash-test.md](docs/hardware/motor-led-flash-test.md).

**Safety:** motors are **disarmed at boot** — type `arm` before spin tests; `disarm` or `stop` kills PWM immediately. Hold the frame down on hardware.

Press **Ctrl+C** to exit the monitor. Press **RST** on the board to reboot into the idle REPL (motors off).

---

## Daily workflow

Work from `Firmware/esp-drone-rs/`:

```bash
cd Firmware/esp-drone-rs

# Recommended — scripts load the ESP env automatically
./scripts/build.sh            # compile only (release)
./scripts/build.sh debug      # compile only (debug)
./scripts/flash.sh            # build + flash + monitor (release)
./scripts/flash.sh debug      # build + flash + monitor (debug)

# Same thing via cargo — ~/export-esp.sh is loaded automatically
cargo build --release
cargo run --release
```

---

## VS Code setup (optional)

You can open either folder:

| Folder opened | rust-analyzer | Build tasks (Ctrl+Shift+B) |
| ------------- | ------------- | -------------------------- |
| **Repo root** (`Co-Create_ESP-FLY/`) | via [`rust-analyzer.toml`](rust-analyzer.toml) + `linkedProjects` in [`.vscode/settings.json`](.vscode/settings.json) | Flash / Build / Wokwi tasks in [`.vscode/tasks.json`](.vscode/tasks.json) |
| **`Firmware/esp-drone-rs/`** | auto-discovers `Cargo.toml` | tasks in [`Firmware/esp-drone-rs/.vscode/tasks.json`](Firmware/esp-drone-rs/.vscode/tasks.json) |

Recommended extensions (see `Firmware/esp-drone-rs/.vscode/extensions.json`):

- **rust-analyzer**
- **Wokwi for VS Code** (simulation — open `Firmware/esp-drone-rs` for Wokwi)

---

## Troubleshooting

| Problem                                | What to try                                                                                                  |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `~/export-esp.sh` not found            | Run `espup install`, then `cargo build` or `./scripts/build.sh`                                            |
| `xtensa-esp32-espidf` target not found | Run `espup install` (the `esp` toolchain is selected by `rust-toolchain.toml`)                             |
| `LIBCLANG_PATH` / linker errors        | `~/export-esp.sh` missing or stale — re-run `espup install`                                                |
| Build fails on `cmake` / `ninja`       | Install system packages from step 2                                                                          |
| First build very slow                  | Normal — ESP-IDF is compiling; wait for it to finish                                                         |
| `Permission denied` on `/dev/ttyUSB0`  | Add user to `dialout` group (see Option B)                                                                   |
| No serial port in WSL2                 | Pass USB through to WSL ([usbipd-win](https://learn.microsoft.com/en-us/windows/wsl/connect-usb) on Windows) |
| Wokwi: firmware not found              | Run `./scripts/build-for-wokwi.sh` from `Firmware/esp-drone-rs`                                              |
| Wokwi: wrong folder                    | Open **`Firmware/esp-drone-rs`**, not the repo root                                                          |
| Stack overflow / boot loop on device   | Already mitigated in `sdkconfig.defaults`; if it persists, check wiring and serial for panic messages        |
| IMU init failed / no sensor stream     | Check MPU-6050 on D14 (SDA) and D13 (SCL), 3V3, and GND; I2C address should be `0x68`                        |

More Wokwi-specific issues: [docs/hardware/wokwi-simulation.md](docs/hardware/wokwi-simulation.md).

---

## Documentation

| Doc                                                                                | Description                           |
| ---------------------------------------------------------------------------------- | ------------------------------------- |
| [docs/hardware/README.md](docs/hardware/README.md)                                 | Hardware doc index                    |
| [docs/hardware/elegoo-esp32-wroom32.md](docs/hardware/elegoo-esp32-wroom32.md)     | Board headers and pin profiles        |
| [docs/hardware/poc-left-header-wiring.md](docs/hardware/poc-left-header-wiring.md) | Breadboard POC wiring                 |
| [docs/hardware/wokwi-simulation.md](docs/hardware/wokwi-simulation.md)             | Run firmware in Wokwi                 |
| [docs/hardware/serial-commands.md](docs/hardware/serial-commands.md)             | UART REPL + joystick mode reference       |
| [docs/hardware/motor-led-flash-test.md](docs/hardware/motor-led-flash-test.md)     | Optional LED / sequential bench tests |

**Firmware source layout** (`Firmware/esp-drone-rs/src/`):

| Module | Status | Role |
|--------|--------|------|
| [`board/`](Firmware/esp-drone-rs/src/board/) | Active | [`BoardProfile`](Firmware/esp-drone-rs/src/board/profile.rs) + [`PeripheralPins`](Firmware/esp-drone-rs/src/board/profile.rs); Elegoo POC active, ESPLANE_V1 planned |
| [`drivers/motors/`](Firmware/esp-drone-rs/src/drivers/motors/) | Active | LEDC PWM, [`MotorWiring`](Firmware/esp-drone-rs/src/drivers/motors/wiring.rs), optional bench tests |
| [`drivers/imu/`](Firmware/esp-drone-rs/src/drivers/imu/) | Active | MPU-6050 I2C, [`ImuWiring`](Firmware/esp-drone-rs/src/drivers/imu/wiring.rs) |
| [`drivers/power/`](Firmware/esp-drone-rs/src/drivers/power/) | Stub | Battery ADC |
| [`sensors/`](Firmware/esp-drone-rs/src/sensors/) | Active | IMU sample task, gyro bias, Mahony (`sensors::init`) |
| [`estimation/`](Firmware/esp-drone-rs/src/estimation/) | Active | Mahony attitude (gyro + accel) |
| [`flight/`](Firmware/esp-drone-rs/src/flight/) | Active | Attitude PID, X-quad mixer, 250 Hz stabilizer (`att on`) |
| [`comm/serial`](Firmware/esp-drone-rs/src/comm/serial.rs) | Active | UART REPL — motor duty, `joy on` stick, IMU stream, attitude setpoints |
| [`safety/`](Firmware/esp-drone-rs/src/safety/) | Active | [`MotorGate`](Firmware/esp-drone-rs/src/safety/motor_gate.rs) arming — disarmed by default |

Pin maps: [`elegoo_esp32_wroom32.rs`](Firmware/esp-drone-rs/src/board/elegoo_esp32_wroom32.rs) (active POC), [`esplane_v1.rs`](Firmware/esp-drone-rs/src/board/esplane_v1.rs) (planned production PCB). Module tree is documented in [`drivers/mod.rs`](Firmware/esp-drone-rs/src/drivers/mod.rs).

---

## Legacy ESP-FLY / C firmware

The original **ESP-FLY** kit targets the Seeed XIAO ESP32-S3 and ships with Espressif’s C **ESP-Drone** firmware. That product documentation and the C tree under `Firmware/esp-drone/` remain useful as hardware reference, but **this repo’s active firmware is Rust on a classic ESP32 (Xtensa) devkit POC**.

For the original kit overview and assembly, see Seeed’s product page and the videos linked in git history / upstream docs.

---

## License

This project is licensed under **GPL-3.0** — see [LICENSE](LICENSE). Firmware derives from ESP-Drone (Espressif) and community modifications; hardware design credits remain with Seeed Studio and Max Imagination for the ESP-FLY kit.

---

## Contributing

1. Fork and clone the repo.
2. Follow the setup steps above until `./scripts/build.sh` succeeds.
3. Make changes in `Firmware/esp-drone-rs/`.
4. Test in Wokwi and/or on hardware before opening a pull request.

Questions about wiring or pin changes should update both the Rust `board` module and the matching file under `docs/hardware/`.
