//! Brushed motor PWM via LEDC (matches ESP-Drone: 15 kHz, 8-bit duty).
//!
//! Hardware wiring (GPIO → LEDC channel) lives in [`crate::board::init_motors`].

use esp_idf_hal::ledc::{LedcDriver, LedcTimerDriver, LowSpeed};
use esp_idf_hal::units::Hertz;

use crate::board::{self, Motor};

/// PWM frequency used by the C ESP-Drone `motors.c`.
const PWM_FREQUENCY: Hertz = Hertz(15_000);

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

pub struct Motors {
    _timer: LedcTimerDriver<'static, LowSpeed>,
    max_duty: u32,
    drivers: [LedcDriver<'static>; 4],
}

impl Motors {
    /// PWM frequency — used by [`crate::board::init_motors`] when creating the shared timer.
    pub const fn pwm_frequency() -> Hertz {
        PWM_FREQUENCY
    }

    /// Construct from hardware already wired by the board layer.
    pub fn from_drivers(
        timer: LedcTimerDriver<'static, LowSpeed>,
        drivers: [LedcDriver<'static>; 4],
    ) -> anyhow::Result<Self> {
        let max_duty = drivers[0].get_max_duty();
        let mut motors = Self {
            _timer: timer,
            max_duty,
            drivers,
        };
        motors.all_off()?;
        Ok(motors)
    }

    pub fn duty_percent(&self, numerator: u32, denominator: u32) -> u32 {
        self.max_duty * numerator / denominator
    }

    pub fn led_test_duty(&self) -> u32 {
        self.duty_percent(LED_TEST_DUTY_NUMERATOR, LED_TEST_DUTY_DENOMINATOR)
    }

    pub fn spin_test_duty(&self) -> u32 {
        self.duty_percent(SPIN_DUTY_NUMERATOR, SPIN_DUTY_DENOMINATOR)
    }

    pub fn set_duty(&mut self, motor: Motor, duty: u32) -> anyhow::Result<()> {
        self.drivers[motor.as_usize()].set_duty(duty)?;
        Ok(())
    }

    pub fn all_off(&mut self) -> anyhow::Result<()> {
        for motor in Motor::ALL {
            self.set_duty(motor, 0)?;
        }
        Ok(())
    }

    /// Run the selected bench test sequence.
    pub fn run_bench_test(&mut self, mode: BenchMode) -> anyhow::Result<()> {
        match mode {
            BenchMode::Led => self.run_sequential_led_test(),
            BenchMode::Spin => self.run_sequential_spin_test(),
        }
    }

    /// Sequential LED test: M1 → M2 → M3 → M4, one channel at a time (no props).
    pub fn run_sequential_led_test(&mut self) -> anyhow::Result<()> {
        let test_duty = self.led_test_duty();

        log::info!("=== Motor LED test begin ===");
        log::info!(
            "Each step: {} ms ON at ~{}% PWM, {} ms gap. Expect ONE LED lit per step.",
            LED_TEST_ON_MS,
            LED_TEST_DUTY_NUMERATOR * 100 / LED_TEST_DUTY_DENOMINATOR,
            LED_TEST_GAP_MS
        );

        self.all_off()?;

        for (step, meta) in board::MOTOR_TABLE.iter().enumerate() {
            log::info!(
                "Step {}/4: {} GPIO{} ({}) — {}",
                step + 1,
                meta.motor.name(),
                meta.gpio,
                meta.header,
                meta.corner
            );
            self.set_duty(meta.motor, test_duty)?;
            esp_idf_hal::delay::FreeRtos::delay_ms(LED_TEST_ON_MS);
            self.set_duty(meta.motor, 0)?;
            esp_idf_hal::delay::FreeRtos::delay_ms(LED_TEST_GAP_MS);
        }

        log::info!("=== Motor LED test complete ===");
        log::info!("Pass: four steps, one LED each, order M1 → M2 → M3 → M4");
        Ok(())
    }

    /// Sequential motor spin test: M1 → M2 → M3 → M4, one motor at a time.
    ///
    /// Tuned for 8520 coreless @ 3.8 V with 55 mm props — ~3.2 V avg, no lift.
    /// Secure the frame on the bench; props must spin freely.
    pub fn run_sequential_spin_test(&mut self) -> anyhow::Result<()> {
        let spin_duty = self.spin_test_duty();
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

        self.all_off()?;
        esp_idf_hal::delay::FreeRtos::delay_ms(500);

        for (step, meta) in board::MOTOR_TABLE.iter().enumerate() {
            log::info!(
                "Spin {}/4: {} GPIO{} ({}) — {}",
                step + 1,
                meta.motor.name(),
                meta.gpio,
                meta.header,
                meta.corner
            );
            self.set_duty(meta.motor, spin_duty)?;
            esp_idf_hal::delay::FreeRtos::delay_ms(SPIN_ON_MS);
            self.set_duty(meta.motor, 0)?;
            esp_idf_hal::delay::FreeRtos::delay_ms(SPIN_GAP_MS);
        }

        log::info!("=== Motor spin test complete ===");
        log::info!("Pass: M1 → M2 → M3 → M4 each twitched briefly with props");
        Ok(())
    }
}
