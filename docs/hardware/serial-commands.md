# Serial commands (UART REPL)

Interactive motor and IMU control over USB serial (**115200 baud**). The firmware prints a `>` prompt in **command mode**; type a line and press **Enter**.

**Runtime help:** type `help` at the `>` prompt (matches [`commands.rs`](../../Firmware/esp-drone-rs/src/comm/serial/commands.rs)).

Used by:

- `./scripts/flash.sh` (hardware monitor)
- Wokwi serial monitor — see [wokwi-simulation.md](./wokwi-simulation.md)

## Boot behaviour

After reset the firmware boots **disarmed** — motors stay off until `arm`. Boot sequence:

1. Pin map log
2. **D27** status LED — one short blink
3. Motor PWM + IMU init (motors **off**)
4. Serial REPL starts (`Serial command task ready — disarmed; type 'help'`)
5. **D27** — two quick blinks, then 500 ms heartbeat

IMU samples run in the background but **do not print** until `imu on`.

## Arming gate (Phase 2)

| State | Motor commands | `joy on` |
|-------|----------------|----------|
| **Disarmed** (default) | Blocked — `ERR disarmed — type 'arm' first` | Blocked |
| **Armed** | PWM applied | Allowed |

- **`arm`** — runs a short **M1 → M4 chirp** (~30% PWM, 150 ms per motor), then enables output
- **`disarm`** / **`stop`** — instant all-off and return to disarmed
- Joystick **Q** — quit joystick and **disarm**

## Command mode

| Command | Action |
|---------|--------|
| `help` or `?` | List commands |
| `arm` | Arm motors (chirp + enable output) |
| `disarm` | Disarm — all motors off |
| `motor <0-100>` | Set **M1** duty percent (**armed only**) |
| `motor <1-4> <0-100>` | Set one motor duty percent (**armed only**) |
| `motor all <0-100>` | Set **all four** motors to the same duty (**armed only**) |
| `motor pulse <0-100> <ms>` | All motors at duty for **1–30000 ms**, then stop (**armed only**) |
| `stop` | Same as `disarm` |
| `joy on` | Enter **joystick mode** (**armed only**) |
| `joy off` | Disarm |
| `imu` | IMU stream status |
| `imu on` | Stream IMU at **10 Hz** (default) |
| `imu on <hz>` | Stream IMU at **1–100 Hz** |
| `imu off` | Stop IMU stream |

IMU lines use plain `println!` (no log prefix) while streaming so the monitor stays readable.

**Echo:** command mode echoes typed characters over serial (helps Wokwi). Joystick mode does not. If letters appear **twice** on a USB serial monitor, disable local echo in that app.

### Examples

```text
> motor 2 30
ERR disarmed — type 'arm' first
> arm
OK armed — motor output enabled
> motor 2 30
OK M2 duty 30%
> motor all 20
OK all motors duty 20%
> joy on
OK joystick mode — E/C throttle, WASD tilt, Space stop, Q disarm
> disarm
OK disarmed — all motors off
```

## Joystick mode (`joy on`)

Requires **armed** state. Command mode is **suspended** — keys are not typed into the `>` line.

| Key | Action |
|-----|--------|
| **E** / **C** | Throttle up / down (**±5%** per keypress) |
| **W** / **S** | Pitch forward / back |
| **A** / **D** | Roll left / right |
| **Space** | Reset stick axes, motors off (**stay armed**) |
| **Q** | Quit joystick and **disarm** |
| **?** | Show joystick help |

Each change applies an **X-quad mix** (M1 front-right, M2 back-right, M3 back-left, M4 front-left) and prints per-motor duties, e.g.:

```text
thr 30% pitch +10 roll -5 | M1=35 M2=35 M3=25 M4=15
```

**Safety:** hold the frame down on hardware; use low throttle with props attached.

## Optional sequential bench tests

[`drivers/motors/bench.rs`](../../Firmware/esp-drone-rs/src/drivers/motors/bench.rs) provides longer `run_bench_test` sequences (LED or spin) for wiring checks — not run on boot. The **`arm` chirp** reuses the same per-motor stepping at shorter timing. See [motor-led-flash-test.md](./motor-led-flash-test.md).

## Implementation

| File | Role |
|------|------|
| [`safety/motor_gate.rs`](../../Firmware/esp-drone-rs/src/safety/motor_gate.rs) | Arming gate — all PWM paths |
| [`safety/arming.rs`](../../Firmware/esp-drone-rs/src/safety/arming.rs) | Armed / disarmed state |
| [`comm/serial.rs`](../../Firmware/esp-drone-rs/src/comm/serial.rs) | Input loop, command vs joystick modes |
| [`comm/serial/commands.rs`](../../Firmware/esp-drone-rs/src/comm/serial/commands.rs) | Text command parser |
| [`comm/serial/joystick.rs`](../../Firmware/esp-drone-rs/src/comm/serial/joystick.rs) | E/C + WASD stick and X-quad mixer |
