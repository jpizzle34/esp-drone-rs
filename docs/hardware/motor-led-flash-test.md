# Motor LED flash test (optional)

GPIO verification with **LEDs on the four motor PWM pins** — no props or motors required.

> **Not the default boot test.** After flashing, [`esp-drone-rs`](../../Firmware/esp-drone-rs/) runs the **motor spin bench test** (`BenchMode::Spin`). Use this LED procedure only when you want to confirm wiring before attaching MOSFETs and motors. See the [README expected output](../../README.md#expected-output) and [wokwi-simulation.md](./wokwi-simulation.md) for the default behaviour.

## Enable the LED test

In [`main.rs`](../../Firmware/esp-drone-rs/src/main.rs), change the bench mode constant:

```rust
const BENCH_MODE: BenchMode = BenchMode::Led;  // default is BenchMode::Spin
```

Rebuild and flash. Switch back to `BenchMode::Spin` when you are ready for the motor bench test.

## Wiring

Connect **one LED + 330 Ω resistor** per motor pin (cathode to GND). See [poc-left-header-wiring.md](./poc-left-header-wiring.md).

| Step | Motor | GPIO | Header | Frame corner | LED should glow |
|------|-------|------|--------|--------------|-----------------|
| 1 | M1 | 32 | D32 | Front-right | alone |
| 2 | M2 | 33 | D33 | Back-right | alone |
| 3 | M3 | 25 | D25 | Back-left | alone |
| 4 | M4 | 26 | D26 | Front-left | alone |

Optional: **D27** status LED — blinks once before the test and twice after (same as the spin test).

## Flash and monitor

You can also run the default spin test in the [Wokwi simulator](./wokwi-simulation.md) first (no hardware).

From `Firmware/esp-drone-rs/` (scripts load the ESP environment automatically):

```bash
cd Firmware/esp-drone-rs
./scripts/flash.sh debug      # debug build + flash + serial monitor
./scripts/flash.sh            # release (default)
```

Or with `cargo` directly (requires `source ~/export-esp.sh` first):

```bash
cargo run              # debug
cargo run --release    # release
```

## Expected behaviour (LED test mode)

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
| Reboot loop | Check serial for panic; reduce duty in `drivers/motors/bench.rs` if needed |
| Motors spin instead of LEDs | Still on `BenchMode::Spin` — change `BENCH_MODE` in `main.rs` |

## Implementation

- Test logic: [`drivers/motors/bench.rs`](../../Firmware/esp-drone-rs/src/drivers/motors/bench.rs) — `run_bench_test` with `BenchMode::Led`
- Based on C [`motorsTest()`](../../Firmware/esp-drone/components/drivers/general/motors/motors.c) (longer on-time for visible LEDs).
- PWM: **15 kHz**, 8-bit LEDC (same as ESP-Drone).

## Re-run without reflash

Power-cycle or press **RST** on the board — the test runs once every boot.
