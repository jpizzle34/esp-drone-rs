# esp-drone-rs

Rust firmware for an ESP32 quadcopter, built on [ESP-IDF](https://docs.espressif.com/projects/esp-idf/) via the [esp-rs](https://github.com/esp-rs) toolchain. This repo is a fork of the [ESP-FLY](https://www.seeedstudio.com/) / ESP-Drone ecosystem; active development is the Rust crate in `Firmware/esp-drone-rs/`. The original C firmware in `Firmware/esp-drone/` is kept for reference only.

**Current firmware:** a motor spin bench test on boot (sequential M1 → M4 PWM), plus a status LED heartbeat. See [docs/hardware/](docs/hardware/) for wiring and expected behaviour.

---

## What you need

| Item                          | Required for      | Notes                                                                          |
| ----------------------------- | ----------------- | ------------------------------------------------------------------------------ |
| **Linux or WSL2**             | Build & flash     | Recommended. Native macOS also works. Windows without WSL is not covered here. |
| **Git**                       | Clone             |                                                                                |
| **Rust** (`rustup`)           | Build             | Installed automatically when you run `espup`                                   |
| **USB cable + ESP32 board**   | Flash to hardware | Elegoo ESP32-WROOM-32 (or similar) for the current POC pinout                  |
| **VS Code + Wokwi extension** | Simulation only   | Optional — test firmware without hardware                                      |

No drone frame or motors are required to **build** the project. You can run the full motor test in the [Wokwi simulator](docs/hardware/wokwi-simulation.md) first.

---

## Quick overview

```
Co-Create_ESP-FLY/          ← clone the repo here (any folder name is fine)
├── .cargo/config.toml      ← ESP32 target, espflash runner, ESP-IDF version
├── rust-toolchain.toml     ← pins the `esp` Rust toolchain
├── Cargo.toml              ← workspace root
├── Firmware/
│   ├── esp-drone-rs/       ← **Rust firmware you build and flash**
│   └── esp-drone/          ← legacy C firmware (reference)
└── docs/hardware/          ← pinout, wiring, Wokwi, flash-test docs
```

---

## Step-by-step: get running locally

### 1. Clone the repository

```bash
git clone https://github.com/jpizzle34/esp-drone-rs.git
cd esp-drone-rs
```

If you use SSH:

```bash
git clone git@github.com:jpizzle34/esp-drone-rs.git
cd esp-drone-rs
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

Then install the ESP32 Rust toolchain and ESP-IDF (this downloads several hundred MB and may take several minutes):

```bash
espup install
```

Load the environment **in every new terminal** before building:

```bash
source ~/export-esp.sh
```

You should see `PATH` and `LIBCLANG_PATH` updated. If `~/export-esp.sh` is missing, re-run `espup install`.

**Tip:** add this to your `~/.bashrc` or `~/.zshrc` so you do not forget:

```bash
source ~/export-esp.sh
```

### 5. First build

From the **repository root** (not `Firmware/esp-drone-rs/`):

```bash
source ~/export-esp.sh
cargo build
```

The first build is slow (often **15–30+ minutes**): it downloads ESP-IDF **v5.5.3** into `.embuild/` and compiles it. Later builds are much faster.

On success you will have:

```
target/xtensa-esp32-espidf/debug/esp-drone-rs
```

That ELF is the firmware binary.

---

## Option A — Simulate in Wokwi (no hardware)

Good first step if you do not have a board yet.

1. Install [VS Code](https://code.visualstudio.com/) and the [Wokwi for VS Code](https://marketplace.visualstudio.com/items?itemName=wokwi.wokwi-vscode) extension.
2. Open the folder **`Firmware/esp-drone-rs`** in VS Code (Wokwi reads `wokwi.toml` from the workspace root).
3. Build from a terminal:

    ```bash
    source ~/export-esp.sh
    cd Firmware/esp-drone-rs
    ./scripts/build-for-wokwi.sh
    ```

4. In VS Code: Command Palette → **Wokwi: Start Simulator**.

You should see serial logs and four motors spin one at a time (M1 → M4). Full details: [docs/hardware/wokwi-simulation.md](docs/hardware/wokwi-simulation.md).

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

From the repo root:

```bash
source ~/export-esp.sh
cargo run
```

`cargo run` builds, flashes via [espflash](https://github.com/esp-rs/espflash), and opens the serial monitor (115200 baud) — configured in [`.cargo/config.toml`](.cargo/config.toml).

Or use the helper script from the firmware crate:

```bash
source ~/export-esp.sh
cd Firmware/esp-drone-rs
./scripts/flash.sh
```

### Expected output

After reset, the serial monitor should show a pin map, then a sequential motor spin test (M1 → M2 → M3 → M4), then a blinking status LED. Example:

```
I (...) esp_drone_rs: ESP-Drone Rust firmware — motor spin bench test
I (...) Motors (LEDC PWM): M1=GPIO32 ... M4=GPIO26
W (...) === Motor spin test begin ===
I (...) Spin 1/4: M1 GPIO32 (D32) — front-right
...
I (...) Idle — status LED heartbeat on D27 / GPIO27
```

**Safety:** hold the frame down during the motor test; props can spin. See [docs/hardware/motor-led-flash-test.md](docs/hardware/motor-led-flash-test.md).

Press **Ctrl+C** to exit the monitor. Press **RST** on the board to run the boot test again.

---

## Daily workflow

Every new terminal session:

```bash
cd esp-drone-rs          # your clone path
source ~/export-esp.sh
cargo build              # compile only
cargo run                # build + flash + monitor (hardware)
```

Release build (smaller/faster on device):

```bash
cargo build --release
espflash flash --monitor target/xtensa-esp32-espidf/release/esp-drone-rs
```

---

## VS Code setup (optional)

Open the **repository root** for Rust-analyzer on the workspace. Recommended extensions (see `Firmware/esp-drone-rs/.vscode/extensions.json`):

- **rust-analyzer**
- **Wokwi for VS Code** (simulation)

Rust-analyzer settings for the ESP target are already in `Firmware/esp-drone-rs/.vscode/settings.json`.

---

## Troubleshooting

| Problem                                | What to try                                                                                                  |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `~/export-esp.sh` not found            | Run `espup install`, then `source ~/export-esp.sh`                                                           |
| `xtensa-esp32-espidf` target not found | Run `espup install`; ensure you sourced `export-esp.sh`                                                      |
| Build fails on `cmake` / `ninja`       | Install system packages from step 2                                                                          |
| First build very slow                  | Normal — ESP-IDF is compiling; wait for it to finish                                                         |
| `Permission denied` on `/dev/ttyUSB0`  | Add user to `dialout` group (see Option B)                                                                   |
| No serial port in WSL2                 | Pass USB through to WSL ([usbipd-win](https://learn.microsoft.com/en-us/windows/wsl/connect-usb) on Windows) |
| Wokwi: firmware not found              | Run `./scripts/build-for-wokwi.sh` from `Firmware/esp-drone-rs`                                              |
| Wokwi: wrong folder                    | Open **`Firmware/esp-drone-rs`**, not the repo root                                                          |
| Stack overflow / boot loop on device   | Already mitigated in `sdkconfig.defaults`; if it persists, check wiring and serial for panic messages        |

More Wokwi-specific issues: [docs/hardware/wokwi-simulation.md](docs/hardware/wokwi-simulation.md).

---

## Documentation

| Doc                                                                                | Description                           |
| ---------------------------------------------------------------------------------- | ------------------------------------- |
| [docs/hardware/README.md](docs/hardware/README.md)                                 | Hardware doc index                    |
| [docs/hardware/elegoo-esp32-wroom32.md](docs/hardware/elegoo-esp32-wroom32.md)     | Board headers and pin profiles        |
| [docs/hardware/poc-left-header-wiring.md](docs/hardware/poc-left-header-wiring.md) | Breadboard POC wiring                 |
| [docs/hardware/wokwi-simulation.md](docs/hardware/wokwi-simulation.md)             | Run firmware in Wokwi                 |
| [docs/hardware/motor-led-flash-test.md](docs/hardware/motor-led-flash-test.md)     | Boot test behaviour and pass criteria |

Pin definitions in code: `Firmware/esp-drone-rs/src/board/`.

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
2. Follow the setup steps above until `cargo build` succeeds.
3. Make changes in `Firmware/esp-drone-rs/`.
4. Test in Wokwi and/or on hardware before opening a pull request.

Questions about wiring or pin changes should update both the Rust `board` module and the matching file under `docs/hardware/`.
