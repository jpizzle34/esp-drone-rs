# Motor LED flash test (optional)

GPIO verification with **LEDs on the four motor PWM pins** — no props or motors required.

> **Not the default boot behaviour.** After flashing, [`esp-drone-rs`](../../Firmware/esp-drone-rs/) boots to the **serial REPL** with motors **off**. Use serial commands or the optional bench test below to exercise GPIO.

## Quick check via serial (no code change)

From the `>` prompt ([serial-commands.md](./serial-commands.md)):

```text
> motor 1 30
> motor 2 30
...
> stop
```

Each command lights one motor channel (or LED substitute) at ~30% PWM.

## Sequential LED bench test (optional code change)

[`drivers/motors/bench.rs`](../../Firmware/esp-drone-rs/src/drivers/motors/bench.rs) provides `run_bench_test(..., BenchMode::Led)` — M1 → M4, ~30% PWM, ~800 ms per step. It is **not** called from [`main.rs`](../../Firmware/esp-drone-rs/src/main.rs) by default.

To run it on boot, temporarily add after motor init in `main.rs`:

```rust
use drivers::motors::{run_bench_test, BenchMode};
// ...
run_bench_test(&mut motors, ActiveBoard::motor_table(), BenchMode::Led)?;
motors.all_off()?;
```

Rebuild and flash. Remove the call when done.

For per-motor **spin** tests (props attached), use `BenchMode::Spin` instead — see timing constants in `bench.rs`.

## Wiring

Connect **one LED** per motor pin, cathode straight to GND. See [poc-left-header-wiring.md](./poc-left-header-wiring.md).

| Step | Motor | GPIO | Header | Frame corner | LED should glow |
|------|-------|------|--------|--------------|-----------------|
| 1 | M1 | 32 | D32 | Front-right | alone |
| 2 | M2 | 33 | D33 | Back-right | alone |
| 3 | M3 | 25 | D25 | Back-left | alone |
| 4 | M4 | 26 | D26 | Front-left | alone |

Optional: **D27** status LED — blinks during boot, then 500 ms heartbeat.

## Flash and monitor

You can also test in the [Wokwi simulator](./wokwi-simulation.md) first (no hardware).

From `Firmware/esp-drone-rs/` (scripts load the ESP environment automatically):

```bash
cd Firmware/esp-drone-rs
./scripts/flash.sh debug      # debug build + flash + serial monitor
./scripts/flash.sh            # release (default)
```

## Expected behaviour (LED bench mode)

When `run_bench_test(..., BenchMode::Led)` is enabled on boot:

1. Serial banner with pin map.
2. **D27** — one short blink (test starting).
3. **Motor test** — each motor pin ~**800 ms ON** at ~**30% PWM**, **400 ms** off before next step.
4. Serial log for each step, e.g.  
   `Step 1/4: M1 GPIO32 (D32) — front-right`
5. **D27** — two short blinks (test done).
6. Serial REPL or heartbeat depending on `main.rs` wiring.

## Pass criteria

- Exactly **one** motor LED lit per step, in order **M1 → M2 → M3 → M4**.
- No other motor LEDs glow during a step (no crosstalk).
- Serial shows all four steps without errors.

## Failures

| Symptom | Likely cause |
|---------|----------------|
| No LEDs | Wrong GPIO, LED polarity, or not on left header pins |
| Wrong order | Swap labels on breadboard — follow table above |
| Multiple LEDs on | Shared wiring fault or short |
| Reboot loop | Check serial for panic |
| Motors spin instead of LEDs | Duty too high for LEDs, or using spin bench config |

## Implementation

- Test logic: [`drivers/motors/bench.rs`](../../Firmware/esp-drone-rs/src/drivers/motors/bench.rs) — `run_bench_test` with `BenchMode::Led` or `BenchMode::Spin`
- Based on C [`motorsTest()`](../../Firmware/esp-drone/components/drivers/general/motors/motors.c) (longer on-time for visible LEDs).
- PWM: **15 kHz**, 8-bit LEDC (same as ESP-Drone).

## Re-run without reflash

Power-cycle or press **RST** — default firmware returns to the serial REPL (motors off).
