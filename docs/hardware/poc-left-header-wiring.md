# POC wiring — left header only

All POC signals are on the **left header** so a breadboard can sit on one side of the board with short jumpers. USB stays at the bottom; **VIN** is **bottom-left**.

Active firmware profile: `POC_LEFT_HEADER` in [`elegoo_esp32_wroom32.rs`](../../Firmware/esp-drone-rs/src/board/elegoo_esp32_wroom32.rs).

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
| **LED** | 27 | D27 | LED anode (+ 330 Ω); cathode → GND |
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

Motor test order in firmware (`M1 → M2 → M3 → M4`) follows **motor index**, not position around the frame clockwise.

## Breadboard wiring diagram

```
         Elegoo LEFT HEADER                    Breadboard / LEDs
         ─────────────────                    ─────────────────

         D32 (M1, FR) ───────────────────────► Motor / LED 1
         D33 (M2, BR) ───────────────────────► Motor / LED 2
         D25 (M3, BL) ───────────────────────► Motor / LED 3
         D26 (M4, FL) ───────────────────────► Motor / LED 4

         D14 (SDA) ──────────────────────────► MPU-6050 SDA
         D13 (SCL) ──────────────────────────► MPU-6050 SCL

         D27 (LED) ────[ 330 Ω ]─── LED ─────► GND

         GND ────────────────────────────────► GND rail
         (3V3 from RIGHT header) ───────────► MPU-6050 VCC
```

## Notes

- The **onboard LED (D2 / GPIO 2)** is on the right header and is **not used** in this POC profile. Phase 0 blink uses an **external LED on D27**.
- **Do not** route motor or I2C signals to the right header in POC — keeps wiring on one side of the **board**, not the drone.
- For the production drone PCB, see `ESPLANE_V1` in [elegoo-esp32-wroom32.md](./elegoo-esp32-wroom32.md).
