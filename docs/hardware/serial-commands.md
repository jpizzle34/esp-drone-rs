# Serial commands (UART REPL)

Interactive motor and IMU control over USB serial (**115200 baud**). The firmware prints a `>` prompt in **command mode**; type a line and press **Enter**.

**Runtime help:** type `help` at the `>` prompt (matches [`commands.rs`](../../Firmware/esp-drone-rs/src/comm/serial/commands.rs)).

Used by:

- `./scripts/flash.sh` (hardware monitor)
- Wokwi serial monitor — see [wokwi-simulation.md](./wokwi-simulation.md)

## Boot behaviour

After reset the firmware does **not** auto-spin motors. Boot sequence:

1. Pin map log
2. **D27** status LED — one short blink
3. Motor PWM + IMU init (motors stay **off**)
4. Serial REPL starts (`Serial command task ready — type 'help'`)
5. **D27** — two quick blinks, then 500 ms heartbeat

IMU samples run in the background but **do not print** until `imu on`.

## Command mode

| Command | Action |
|---------|--------|
| `help` or `?` | List commands |
| `motor <0-100>` | Set **M1** duty percent |
| `motor <1-4> <0-100>` | Set one motor duty percent |
| `motor all <0-100>` | Set **all four** motors to the same duty (stays on until `stop`) |
| `motor pulse <0-100> <ms>` | All motors at duty for **1–30000 ms**, then auto-stop |
| `stop` | All motors off |
| `joy on` | Enter **joystick mode** (see below) |
| `joy off` | All motors off (from command mode) |
| `imu` | IMU stream status |
| `imu on` | Stream IMU at **10 Hz** (default) |
| `imu on <hz>` | Stream IMU at **1–100 Hz** |
| `imu off` | Stop IMU stream |

IMU lines use plain `println!` (no log prefix) while streaming so the monitor stays readable.

### Examples

```text
> motor 2 30
OK M2 duty 30%
> motor all 20
OK all motors duty 20%
> motor pulse 40 1000
OK all motors 40% for 1000 ms
> imu on
OK IMU stream on (10 Hz)
IMU accel=(+0.01,-0.02,+1.00)g gyro=(+0,+1,-0)°/s tilt roll=+1.2° pitch=-0.5° temp=28.3°C
> stop
OK all motors stopped
```

## Joystick mode (`joy on`)

Command mode is **suspended** — keys are not typed into the `>` line. Press **Q** to return to command mode.

| Key | Action |
|-----|--------|
| **E** / **C** | Throttle up / down (**±5%** per keypress) |
| **W** / **S** | Pitch forward / back |
| **A** / **D** | Roll left / right |
| **Space** | Reset all axes to 0, motors off |
| **Q** | Quit joystick mode, motors off |
| **?** | Show joystick help |

Each change applies an **X-quad mix** (M1 front-right, M2 back-right, M3 back-left, M4 front-left) and prints per-motor duties, e.g.:

```text
thr 30% pitch +10 roll -5 | M1=35 M2=35 M3=25 M4=15
```

**Safety:** motors respond immediately — no arming gate yet. Use low throttle on hardware with props secured.

## Optional sequential bench tests

[`drivers/motors/bench.rs`](../../Firmware/esp-drone-rs/src/drivers/motors/bench.rs) still provides `run_bench_test` (LED or per-motor spin) for wiring checks, but it is **not** called on boot. See [motor-led-flash-test.md](./motor-led-flash-test.md).

## Implementation

| File | Role |
|------|------|
| [`comm/serial.rs`](../../Firmware/esp-drone-rs/src/comm/serial.rs) | Input loop, command vs joystick modes |
| [`comm/serial/commands.rs`](../../Firmware/esp-drone-rs/src/comm/serial/commands.rs) | Text command parser |
| [`comm/serial/joystick.rs`](../../Firmware/esp-drone-rs/src/comm/serial/joystick.rs) | E/C + WASD stick and X-quad mixer |
