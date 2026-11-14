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

The IMU samples at **250 Hz** in the background (Mahony roll/pitch/yaw in the chip frame). Lines **do not print** until `imu on`. Gyro bias is averaged at boot — leave the board still for about a second after reset. Yaw has no magnetometer and will drift. `imu cal` repeats the bias average and re-aligns roll/pitch to gravity. Bad I2C / all-zero samples are rejected: fusion and the last good attitude are held until the sensor recovers.

## Arming gate (Phase 2)

| State | Motor commands | `joy on` | `att on` |
|-------|----------------|----------|---------|
| **Disarmed** (default) | Blocked — `ERR disarmed — type 'arm' first` | Blocked | Blocked |
| **Armed** | PWM applied | Allowed | Allowed |
| **Attitude mode** | Blocked — stabilizer owns PWM | Blocked | Already on |

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
| `imu` | Attitude and stream status |
| `imu on` | Stream IMU at **10 Hz** (default) |
| `imu on <hz>` | Stream IMU at **1–100 Hz** |
| `imu off` | Stop IMU stream |
| `imu cal` | Re-zero gyro bias — keep the board still (~1 s) |
| `att` | Attitude-mode setpoints, measured attitude, and last duties |
| `att on` | Angle mode (**armed**). Holds level and the current yaw, thrust **0** |
| `att off` | Leave angle mode — motors stop, **stay armed** |
| `att level` | Roll/pitch setpoints 0, recapture yaw |
| `att thrust <0-100>` | Collective thrust percent |
| `att roll <deg>` | Roll setpoint, **-30..+30** (right side down is +) |
| `att pitch <deg>` | Pitch setpoint, **-30..+30** (nose up is +) |
| `att yaw <deg>` | Yaw setpoint in degrees |
| `pid` | Angle and rate PID gains (kp ki kd) |
| `pid defaults` | Restore the ESP32 Crazyflie defaults |
| `pid <loop> <kp> <ki> <kd>` | Set one loop: `roll`, `pitch`, `yaw`, `roll-rate`, `pitch-rate`, `yaw-rate` |

IMU lines use plain `println!` (no log prefix) while streaming so the monitor stays readable.

## Attitude mode (`att on`)

Closed-loop angle mode. The 250 Hz stabilizer reads Mahony attitude (chip frame, same axes as the IMU stream) and mixes corrections onto the POC X layout: **M1** front-right, **M2** back-right, **M3** back-left, **M4** front-left.

- **`att on`** requires **armed**. It zeros thrust, sets roll and pitch to 0, and holds the yaw at the moment you enabled it.
- Motors stay at 0 until **`att thrust`**. Thrust is collective percent; roll, pitch, and yaw are added around it.
- **`att off`** stops the motors and leaves the board armed. **`disarm`** / **`stop`** leave attitude mode as well.
- While attitude mode is on, `motor` and `joy on` are rejected — the stabilizer owns the PWM.
- If the IMU sample is missing or older than about **100 ms**, the firmware disarms and prints `OK disarmed — IMU sample lost, attitude mode off`. `imu cal` pauses sampling for about a second, so it also trips that disarm.
- About 10 times a second, while attitude mode is on:

```text
att thr 20% sp=+0.0/+0.0/+12.3 meas=+1.2/-0.4/+12.1 | M1=22 M2=22 M3=18 M4=18
```

`sp` is the setpoint (roll/pitch/yaw) and `meas` is the fused attitude. The low side spins up: a right-side-down tilt (`meas` roll positive, setpoint 0) raises **M1/M2**, and a nose-up tilt raises **M2/M3**.

**Bench check, props off:** `arm`, then `att on`, then `att thrust 15`. Tilt the board and watch the low side come up. `att off` or `disarm` before putting it down and walking away. Yaw has no magnetometer and will drift. The chip axes are not rotated into an airframe yet — this holds the IMU axes, not a finished frame.

## PID gains (`pid`)

The attitude cascade is an angle loop into a rate loop, for roll, pitch, and yaw. Gains start at the ESP32 Crazyflie defaults and can be changed while disarmed or while attitude mode is running. The stabilizer copies them on its next tick. Changing a loop clears that loop's integrator so a new Ki does not dump the old accumulated error. Integrator limits stay at the firmware defaults. Each gain must be **0–5000**.

```text
> pid
OK pid roll 5.9 2.9 0
OK pid pitch 5.9 2.9 0
OK pid yaw 6 1 0.35
OK pid roll-rate 250 500 2.5
OK pid pitch-rate 250 500 2.5
OK pid yaw-rate 120 16.7 0
> pid roll 6 2.9 0
OK pid roll 6 2.9 0
> pid defaults
```

`pid defaults` prints the six default lines again. The IMU dashboard has the same controls.

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
