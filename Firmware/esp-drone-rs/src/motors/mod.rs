//! Brushed motor PWM via LEDC (matches ESP-Drone: 15 kHz, 8-bit duty).

use esp_idf_hal::gpio::{Gpio25, Gpio26, Gpio32, Gpio33};
use esp_idf_hal::ledc::{config::TimerConfig, LedcDriver, LedcTimerDriver, LowSpeed, LEDC};
use esp_idf_hal::units::Hertz;

use crate::board;

/// PWM frequency used by the C ESP-Drone `motors.c`.
const PWM_FREQUENCY: Hertz = Hertz(15_000);

// ---------------------------------------------------------------------------
// LED bench test — no props; ~30 % duty lights the MOSFET indicator LEDs.
// Kept for wiring checks without propellers attached.
// ---------------------------------------------------------------------------
#[allow(dead_code)]
const LED_TEST_DUTY_NUMERATOR: u32 = 3;
#[allow(dead_code)]
const LED_TEST_DUTY_DENOMINATOR: u32 = 10;
#[allow(dead_code)]
const LED_TEST_ON_MS: u32 = 800;
#[allow(dead_code)]
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

pub struct MotorMeta {
    pub name: &'static str,
    pub corner: &'static str,
    pub header: &'static str,
    pub gpio: u8,
}

pub const MOTOR_TABLE: [MotorMeta; 4] = [
    MotorMeta {
        name: "M1",
        corner: "front-right",
        header: "D32",
        gpio: board::MOTOR_M1,
    },
    MotorMeta {
        name: "M2",
        corner: "back-right",
        header: "D33",
        gpio: board::MOTOR_M2,
    },
    MotorMeta {
        name: "M3",
        corner: "back-left",
        header: "D25",
        gpio: board::MOTOR_M3,
    },
    MotorMeta {
        name: "M4",
        corner: "front-left",
        header: "D26",
        gpio: board::MOTOR_M4,
    },
];

pub struct Motors {
    _timer: LedcTimerDriver<'static, LowSpeed>,
    max_duty: u32,
    m1: LedcDriver<'static>,
    m2: LedcDriver<'static>,
    m3: LedcDriver<'static>,
    m4: LedcDriver<'static>,
}

impl Motors {
    pub fn new(
        ledc: LEDC,
        gpio32: Gpio32<'static>,
        gpio33: Gpio33<'static>,
        gpio25: Gpio25<'static>,
        gpio26: Gpio26<'static>,
    ) -> anyhow::Result<Self> {
        let timer_config = TimerConfig::default().frequency(PWM_FREQUENCY.into());
        let timer = LedcTimerDriver::new(ledc.timer0, &timer_config)?;

        let m1 = LedcDriver::new(ledc.channel0, &timer, gpio32)?;
        let max_duty = m1.get_max_duty();
        let m2 = LedcDriver::new(ledc.channel1, &timer, gpio33)?;
        let m3 = LedcDriver::new(ledc.channel2, &timer, gpio25)?;
        let m4 = LedcDriver::new(ledc.channel3, &timer, gpio26)?;

        let mut motors = Self {
            _timer: timer,
            max_duty,
            m1,
            m2,
            m3,
            m4,
        };
        motors.all_off()?;
        Ok(motors)
    }

    pub fn duty_percent(&self, numerator: u32, denominator: u32) -> u32 {
        self.max_duty * numerator / denominator
    }

    #[allow(dead_code)]
    pub fn led_test_duty(&self) -> u32 {
        self.duty_percent(LED_TEST_DUTY_NUMERATOR, LED_TEST_DUTY_DENOMINATOR)
    }

    pub fn spin_test_duty(&self) -> u32 {
        self.duty_percent(SPIN_DUTY_NUMERATOR, SPIN_DUTY_DENOMINATOR)
    }

    pub fn set_duty(&mut self, index: usize, duty: u32) -> anyhow::Result<()> {
        self.driver_mut(index)?.set_duty(duty)?;
        Ok(())
    }

    pub fn all_off(&mut self) -> anyhow::Result<()> {
        for i in 0..4 {
            self.set_duty(i, 0)?;
        }
        Ok(())
    }

    #[allow(dead_code)]
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

        for (index, meta) in MOTOR_TABLE.iter().enumerate() {
            log::info!(
                "Step {}/4: {} GPIO{} ({}) — {}",
                index + 1,
                meta.name,
                meta.gpio,
                meta.header,
                meta.corner
            );
            self.set_duty(index, test_duty)?;
            esp_idf_hal::delay::FreeRtos::delay_ms(LED_TEST_ON_MS);
            self.set_duty(index, 0)?;
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

        for (index, meta) in MOTOR_TABLE.iter().enumerate() {
            log::info!(
                "Spin {}/4: {} GPIO{} ({}) — {}",
                index + 1,
                meta.name,
                meta.gpio,
                meta.header,
                meta.corner
            );
            self.set_duty(index, spin_duty)?;
            esp_idf_hal::delay::FreeRtos::delay_ms(SPIN_ON_MS);
            self.set_duty(index, 0)?;
            esp_idf_hal::delay::FreeRtos::delay_ms(SPIN_GAP_MS);
        }

        log::info!("=== Motor spin test complete ===");
        log::info!("Pass: M1 → M2 → M3 → M4 each twitched briefly with props");
        Ok(())
    }

    fn driver_mut(&mut self, index: usize) -> anyhow::Result<&mut LedcDriver<'static>> {
        Ok(match index {
            0 => &mut self.m1,
            1 => &mut self.m2,
            2 => &mut self.m3,
            3 => &mut self.m4,
            _ => anyhow::bail!("invalid motor index {index}"),
        })
    }
}
