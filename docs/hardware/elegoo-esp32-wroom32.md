# Elegoo ESP32-WROOM-32

Development board used for the Rust firmware POC: dual-core Xtensa ESP32, CP2102 USB-UART, 30-pin headers.

**Orientation in all diagrams:** USB port at the **bottom**, **VIN** at **bottom-left** of the left header.

## Header layout

```
                    ┌─────────────────────────┐
                    │   ELEGOO ESP32-WROOM    │
    LEFT HEADER     │                         │     RIGHT HEADER
    (top → bottom)  │                         │     (top → bottom)
    ─────────────   │                         │   ─────────────
    EN              │                         │              D23
    VP  (GPIO36)    │                         │   D22  (SCL)  ← default I2C, not used in POC
    VN  (GPIO39)    │                         │   TX0 (GPIO1)  ⚠ USB serial
    D34             │                         │   RX0 (GPIO3)  ⚠ USB serial
    D35             │                         │   D21  (SDA)  ← default I2C, not used in POC
    D32  (M1)       │                         │              D19
    D33  (M2)       │                         │              D18
    D25  (M3)       │                         │              D5
    D26  (M4)       │                         │              TX2
    D27  (LED)      │                         │              RX2
    D14  (SDA)      │                         │   D4
    D12  ⚠ boot     │                         │   D2  ← onboard LED (right header only)
    D13  (SCL)      │                         │   D15  ⚠ boot
    GND             │                         │              GND
    VIN  (5V)  ◄──── bottom-left             │              3V3
                    └─────────────────────────┘
                           [USB]  RST  BOOT
```

⚠ **Do not use for motors:** GPIO 0, 1, 3, 12, 15 (boot / USB). GPIO 34–39 are input-only.

## Pin profiles

| Profile | Status | Use case |
|---------|--------|----------|
| [`POC_LEFT_HEADER`](./poc-left-header-wiring.md) | **Active** | Breadboard POC — all drone wires on left header |
| [`ESPLANE_V1`](../../Firmware/esp-drone-rs/src/board/esplane_v1.rs) | Planned | ESP-Drone classic ESP32 drone PCB (mixed headers) |

### POC_LEFT_HEADER (active)

| Function | GPIO | Header | Frame corner |
|----------|------|--------|--------------|
| Status LED | 27 | D27 | External LED + 330 Ω to GND |
| I2C SDA | 14 | D14 | MPU-6050 |
| I2C SCL | 13 | D13 | MPU-6050 |
| Motor M1 | 32 | D32 | LEDC PWM — **front-right** |
| Motor M2 | 33 | D33 | LEDC PWM — **back-right** |
| Motor M3 | 25 | D25 | LEDC PWM — **back-left** |
| Motor M4 | 26 | D26 | LEDC PWM — **front-left** |

In Rust firmware, GPIO claiming for this profile is centralized in [`DronePins::take`](../../Firmware/esp-drone-rs/src/board/elegoo_esp32_wroom32.rs) (`MotorPins`, `ImuPins`, and status LED are grouped there).

### ESPLANE_V1 (planned — `board-esplane-v1` feature)

Production PCB profile; firmware not implemented yet. Pin constants live in [`esplane_v1.rs`](../../Firmware/esp-drone-rs/src/board/esplane_v1.rs) (Cargo feature `board-esplane-v1`).

| Function | GPIO | Header | Frame corner |
|----------|------|--------|--------------|
| Status LED | 2 | D2 (onboard) | — |
| I2C SDA | 21 | D21 | — |
| I2C SCL | 22 | D22 | — |
| Motor M1 | 4 | D4 | front-right |
| Motor M2 | 33 | D33 | back-right |
| Motor M3 | 32 | D32 | back-left |
| Motor M4 | 25 | D25 | front-left |

## Quadcopter motor layout (X formation)

Top view of the **airframe** (not the devkit header). Matches Crazyflie / ESP-Drone `QUAD_FORMATION_X` mixer.

```
              FRONT ↑

      M4 (FL)           M1 (FR)
        ╲                 ╱
         ╲               ╱
          ╲             ╱
           [ FC board ]
          ╱             ╲
         ╱               ╲
        ╱                 ╲
      M3 (BL)           M2 (BR)

              BACK ↓
```

POC GPIO mapping (same motor indices):

| Corner | Motor | POC pin |
|--------|-------|---------|
| Front-left | M4 | D26 |
| Front-right | M1 | D32 |
| Back-left | M3 | D25 |
| Back-right | M2 | D33 |

Mixer equations (from ESP-Drone / Crazyflie):

- M1 = thrust − roll/2 + pitch/2 + yaw  
- M2 = thrust − roll/2 − pitch/2 − yaw  
- M3 = thrust + roll/2 − pitch/2 + yaw  
- M4 = thrust + roll/2 + pitch/2 − yaw  

See [poc-left-header-wiring.md](./poc-left-header-wiring.md) for why **board header side** and **frame left/right** must not be mixed in one diagram.
