# Motor LED flash test

After flashing [`esp-drone-rs`](../../Firmware/esp-drone-rs/), the firmware runs a **sequential motor-channel test** on boot using LEDs wired to the four motor PWM pins.

## Wiring

Connect **one LED + 330 Ω resistor** per motor pin (cathode to GND). See [poc-left-header-wiring.md](./poc-left-header-wiring.md).

| Step | Motor | GPIO | Header | Frame corner | LED should glow |
|------|-------|------|--------|--------------|-----------------|
| 1 | M1 | 32 | D32 | Front-right | alone |
| 2 | M2 | 33 | D33 | Back-right | alone |
| 3 | M3 | 25 | D25 | Back-left | alone |
| 4 | M4 | 26 | D26 | Front-left | alone |

Optional: **D27** status LED (same as Phase 0) — blinks once before the test and twice after.

## Flash and monitor

See [wokwi-simulation.md](./wokwi-simulation.md) to run the same test in the **Wokwi simulator** first (no hardware).

```bash
source ~/export-esp.sh
cd Firmware/esp-drone-rs
cargo build
espflash flash --monitor target/xtensa-esp32-espidf/debug/esp-drone-rs
```

## Expected behaviour

1. Serial banner with pin map.
2. **D27** — one short blink (test starting).
3. **Motor test** — each motor pin ~**800 ms ON** at ~**30% PWM**, **400 ms** off before next step.
4. Serial log for each step, e.g.  
   `Step 1/4: M1 GPIO32 (D32) — front-right`
5. **D27** — two short blinks (test done).
6. **D27** — normal 500 ms heartbeat forever.

## Pass criteria

- Exactly **one** motor LED lit per step, in order **M1 → M2 → M3 → M4**.
- No other motor LEDs glow during a step (no crosstalk).
- Serial shows all four steps without errors.

## Failures

| Symptom | Likely cause |
|---------|----------------|
| No LEDs | Wrong GPIO, LED polarity, missing resistor, or not on left header pins |
| Wrong order | Swap labels on breadboard — follow table above |
| Multiple LEDs on | Shared wiring fault or short |
| Reboot loop | Check serial for panic; reduce duty in `motors/mod.rs` if needed |

## Implementation

- Test logic: [`Firmware/esp-drone-rs/src/motors/mod.rs`](../../Firmware/esp-drone-rs/src/motors/mod.rs) — `run_sequential_led_test()`
- Based on C [`motorsTest()`](../../Firmware/esp-drone/components/drivers/general/motors/motors.c) (longer on-time for visible LEDs).
- PWM: **15 kHz**, 8-bit LEDC (same as ESP-Drone).

## Re-run without reflash

Power-cycle or press **RST** on the board — the test runs once every boot.
