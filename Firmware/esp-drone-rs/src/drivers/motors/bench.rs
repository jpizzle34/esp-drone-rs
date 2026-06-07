//! Sequential motor bench tests — wiring check (LED) and per-motor spin.

use esp_idf_hal::delay::FreeRtos;

use super::meta::MotorMeta;
use super::Motors;

/// Bench test mode — select in `main` before flashing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub enum BenchMode {
    /// MOSFET indicator LEDs only; safe without propellers.
    Led,
    /// Spins 8520 motors one at a time; props attached, frame secured.
    Spin,
}

// ---------------------------------------------------------------------------
// LED bench test — no props; ~30 % duty lights the MOSFET indicator LEDs.
// ---------------------------------------------------------------------------
const LED_TEST_DUTY_NUMERATOR: u32 = 3;
const LED_TEST_DUTY_DENOMINATOR: u32 = 10;
const LED_TEST_ON_MS: u32 = 800;
const LED_TEST_GAP_MS: u32 = 400;

// ---------------------------------------------------------------------------
// Motor spin test — 8520 coreless, 3.8 V supply, 3.2 V target, 55 mm props.
//
// Goal: per-motor spin on the bench, not lift. One motor at a time.
// Duty = 3.2 V / 3.8 V (~84 %); average motor voltage ≈ supply × duty.
// ---------------------------------------------------------------------------
const SPIN_DUTY_NUMERATOR: u32 = 32;
const SPIN_DUTY_DENOMINATOR: u32 = 38;
const SPIN_ON_MS: u32 = 1000;
const SPIN_GAP_MS: u32 = 1000;

struct SequentialConfig {
    step_label: &'static str,
    duty_num: u32,
    duty_den: u32,
    on_ms: u32,
    gap_ms: u32,
    prelude_delay_ms: u32,
}

const LED_CONFIG: SequentialConfig = SequentialConfig {
    step_label: "Step",
    duty_num: LED_TEST_DUTY_NUMERATOR,
    duty_den: LED_TEST_DUTY_DENOMINATOR,
    on_ms: LED_TEST_ON_MS,
    gap_ms: LED_TEST_GAP_MS,
    prelude_delay_ms: 0,
};

const SPIN_CONFIG: SequentialConfig = SequentialConfig {
    step_label: "Spin",
    duty_num: SPIN_DUTY_NUMERATOR,
    duty_den: SPIN_DUTY_DENOMINATOR,
    on_ms: SPIN_ON_MS,
    gap_ms: SPIN_GAP_MS,
    prelude_delay_ms: 500,
};

/// Run the selected bench test sequence against a board's motor table.
pub fn run_bench_test(
    motors: &mut Motors,
    table: &[MotorMeta],
    mode: BenchMode,
) -> anyhow::Result<()> {
    match mode {
        BenchMode::Led => {
            log_led_preamble();
            run_sequential(motors, table, &LED_CONFIG)?;
            log::info!("=== Motor LED test complete ===");
            log::info!("Pass: four steps, one LED each, order M1 → M2 → M3 → M4");
        }
        BenchMode::Spin => {
            log_spin_preamble();
            run_sequential(motors, table, &SPIN_CONFIG)?;
            log::info!("=== Motor spin test complete ===");
            log::info!("Pass: M1 → M2 → M3 → M4 each twitched briefly with props");
        }
    }
    Ok(())
}

fn log_led_preamble() {
    log::info!("=== Motor LED test begin ===");
    log::info!(
        "Each step: {} ms ON at ~{}% PWM, {} ms gap. Expect ONE LED lit per step.",
        LED_TEST_ON_MS,
        LED_TEST_DUTY_NUMERATOR * 100 / LED_TEST_DUTY_DENOMINATOR,
        LED_TEST_GAP_MS
    );
}

fn log_spin_preamble() {
    let duty_pct = SPIN_DUTY_NUMERATOR * 100 / SPIN_DUTY_DENOMINATOR;

    log::warn!("=== Motor spin test begin ===");
    log::warn!(
        "8520 / 3.8 V / 55 mm props — {} ms pulse at ~{}% PWM (~{:.2} V avg), {} ms gap",
        SPIN_ON_MS,
        duty_pct,
        3.8 * SPIN_DUTY_NUMERATOR as f32 / SPIN_DUTY_DENOMINATOR as f32,
        SPIN_GAP_MS
    );
    log::warn!("Hold frame down; one motor spins at a time. No collective throttle.");
}

fn run_sequential(
    motors: &mut Motors,
    table: &[MotorMeta],
    config: &SequentialConfig,
) -> anyhow::Result<()> {
    let test_duty = motors.duty_percent(config.duty_num, config.duty_den);

    motors.all_off()?;
    if config.prelude_delay_ms > 0 {
        FreeRtos::delay_ms(config.prelude_delay_ms);
    }

    for (step, meta) in table.iter().enumerate() {
        log::info!(
            "{} {}/4: {} GPIO{} ({}) — {}",
            config.step_label,
            step + 1,
            meta.motor.name(),
            meta.gpio,
            meta.header,
            meta.corner
        );
        motors.set_duty(meta.motor, test_duty)?;
        FreeRtos::delay_ms(config.on_ms);
        motors.set_duty(meta.motor, 0)?;
        FreeRtos::delay_ms(config.gap_ms);
    }

    Ok(())
}
