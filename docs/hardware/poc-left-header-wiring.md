# POC wiring — left header only

All POC signals are on the **left header** so a breadboard can sit on one side of the board with short jumpers. USB stays at the bottom; **VIN** is **bottom-left**.

Active firmware profile: `POC_LEFT_HEADER` in [`elegoo_esp32_wroom32.rs`](../../Firmware/esp-drone-rs/src/board/elegoo_esp32_wroom32.rs). GPIO is claimed via [`BoardProfile::take`](../../Firmware/esp-drone-rs/src/board/profile.rs) and split with [`PeripheralPins`](../../Firmware/esp-drone-rs/src/board/profile.rs).

## Left header pin map (POC)

This list is **only** the order of pins down the **board’s left header**. It is **not** a top-down view of the drone.

```
    LEFT HEADER (top → bottom)
    ─────────────────────────
    EN
    VP, VN, D34, D35     ← unused (input / ADC)
    D32  ── M1  (motor 1 PWM)
    D33  ── M2  (motor 2 PWM)
    D25  ── M3  (motor 3 PWM)
    D26  ── M4  (motor 4 PWM)
    D27  ── LED (status, external)
    D14  ── I2C SDA (MPU-6050)
    D13  ── I2C SCL (MPU-6050)
    D12                  ← unused (boot strap — do not drive low at reset)
    GND
    VIN                  ← bottom-left
```

All four motor wires leave the **same board header** (left). That has nothing to do with front/back/left/right on the airframe.

## Signal summary

| Signal | GPIO | Header | Frame corner (Crazyflie X) |
|--------|------|--------|----------------------------|
| **M1** | 32 | D32 | **Front-right** |
| **M2** | 33 | D33 | **Back-right** |
| **M3** | 25 | D25 | **Back-left** |
| **M4** | 26 | D26 | **Front-left** |
| **SDA** | 14 | D14 | MPU-6050 SDA |
| **SCL** | 13 | D13 | MPU-6050 SCL |
| **LED** | 27 | D27 | LED anode; cathode → GND |
| **3V3** | — | right header 3V3 | MPU-6050 VCC |
| **GND** | — | left or right GND | Common ground |

## Why the old diagram was confusing

An earlier version showed a 2×2 grid like this:

```
    D32 (M1) front-left     |  D25 (M3) front-right
    D33 (M2) back-right     |  D26 (M4) back-left
```

That caused two problems:

1. **Wrong corner names** — Crazyflie / ESP-Drone X-mixer uses **M1 = front-right**, **M4 = front-left**, **M3 = back-left**, **M2 = back-right** (see [elegoo-esp32-wroom32.md](./elegoo-esp32-wroom32.md)).
2. **Mixed up two different layouts** — The left/right **columns in that grid were not “left/right side of the drone.”** They were an arbitrary way to fit four pins on paper. **Every motor pin is on the board’s left header**, so seeing “front-left” and “back-right” in the same column did **not** mean those motors belong on the same side of the frame.

On the **actual frame** (top view):

- **Left side** of the drone: **M4 (front-left)** and **M3 (back-left)**
- **Right side** of the drone: **M1 (front-right)** and **M2 (back-right)**

“Front-left” and “back-right” are on **opposite corners** (a diagonal), never the same side.

## Frame mounting (top view)

Pick a **front** direction on your frame and mount each motor at the matching corner. Wire each arm back to the GPIO in the table above.

```
              FRONT ↑
         (your chosen nose)

      M4  FL              FR  M1
     D26                   D32

            [ flight controller ]

      M3  BL              BR  M2
     D25                   D33

              BACK ↓
```

Motor index order in firmware is **M1 → M2 → M3 → M4** (front-right, back-right, back-left, front-left), not necessarily clockwise around the frame. After boot, use the serial REPL to command motors; MPU-6050 data streams only after `imu on` — see [serial-commands.md](./serial-commands.md).

## Motor driver (per channel)

Each GPIO drives the **gate of an SI2300** (Si2300DS) directly. There is no series resistor and no gate pull-down. The four motors sit in parallel on the supply: every motor positive shares battery positive, and each drain switches one motor negative to ground. A **1N5819 Schottky** across each motor clamps inductive kick when the MOSFET turns off.

SOT-23, marking **P2** facing you, pins down: **1 gate, 2 source, 3 drain**.

```
  motor supply (+) ────────► motor +
                         ┌──►|──  1N5819 (band / cathode → +)
  GPIO ────────────►|G    │    anode ──► motor −
            SI2300  D|────┘
                    |S
                    └──► GND (common with ESP32)
```

| Part | Value / type | Notes |
|------|----------------|-------|
| MOSFET | SI2300 N-channel | Low-side switch, one per motor |
| Gate | GPIO direct | D32 / D33 / D25 / D26 to pin 1. No resistor |
| Source | GND | Pin 2. Common with ESP32 and battery negative |
| Drain | Motor − | Pin 3. One drain per motor |
| Flyback diode | 1N5819 Schottky | Band (**C**) → motor **+**, anode → drain |

**POC bench:** motor **+** from **1S LiPo (~3.8 V)** for meaningful 8520 thrust when using serial commands or bench tests. LED-only GPIO checks can use **3.3 V** with no props — see [motor-led-flash-test.md](./motor-led-flash-test.md). **Flight build:** same LiPo supply; ESP32 **GND** common only — do not power motors from the ESP **3V3** pin on a real quad.

## Breadboard wiring diagram

```
         Elegoo LEFT HEADER                    Motor driver (×4, one shown)
         ─────────────────                    ───────────────────────────

         D32 (M1, FR) ───────────────────────► SI2300 gate → drain → M1 motor −
         D33 (M2, BR) ───────────────────────► (same for M2–M4)
         D25 (M3, BL)
         D26 (M4, FL)

         motor + (×4, in parallel) ◄── 1S LiPo + (spin test) or 3V3 (LED-only check)
                              each: 1N5819 band → +, anode → drain

         D14 (SDA) ──────────────────────────► MPU-6050 SDA
         D13 (SCL) ──────────────────────────► MPU-6050 SCL

         D27 (LED) ──── LED anode; cathode ──► GND

         GND ────────────────────────────────► GND rail (ESP + SI2300 source + LiPo −)
         (3V3 from RIGHT header) ───────────► MPU-6050 VCC
```

For GPIO-only checks without motors, use an **LED** on each motor pin (cathode to GND) instead of the SI2300 — see [motor-led-flash-test.md](./motor-led-flash-test.md).

## Notes

- The **onboard LED (D2 / GPIO 2)** is on the right header and is **not used** in this POC profile. Phase 0 blink uses an **external LED on D27**.
- **Do not** route motor or I2C signals to the right header in POC — keeps wiring on one side of the **board**, not the drone.
- **1N5819** flyback diodes (one per motor) sit across each motor, band toward motor positive. The gate wire has no resistor. See [wokwi-simulation.md](./wokwi-simulation.md).
- For the production drone PCB, see the ESPLANE_V1 section in [elegoo-esp32-wroom32.md](./elegoo-esp32-wroom32.md) and [`esplane_v1.rs`](../../Firmware/esp-drone-rs/src/board/esplane_v1.rs).
