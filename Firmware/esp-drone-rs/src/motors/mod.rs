//! Brushed motor PWM via LEDC (matches ESP-Drone: 15 kHz, 8-bit duty).

use esp_idf_hal::gpio::{Gpio25, Gpio26, Gpio32, Gpio33};
use esp_idf_hal::ledc::{config::TimerConfig, LedcDriver, LedcTimerDriver, LowSpeed, LEDC};
use esp_idf_hal::units::Hertz;

use crate::board;

/// PWM frequency used by the C ESP-Drone `motors.c`.
const PWM_FREQUENCY: Hertz = Hertz(15_000);

/// Bench-test duty (~30 %). C firmware uses 20 % for `motorsTest()`.
const TEST_DUTY_NUMERATOR: u32 = 3;
const TEST_DUTY_DENOMINATOR: u32 = 10;

const TEST_ON_MS: u32 = 800;
const TEST_GAP_MS: u32 = 400;

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

    pub fn test_duty(&self) -> u32 {
        self.max_duty * TEST_DUTY_NUMERATOR / TEST_DUTY_DENOMINATOR
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

    /// Sequential LED/motor test: M1 → M2 → M3 → M4, one channel at a time.
    pub fn run_sequential_led_test(&mut self) -> anyhow::Result<()> {
        let test_duty = self.test_duty();

        log::info!("=== Motor LED test begin ===");
        log::info!(
            "Each step: {} ms ON at ~{}% PWM, {} ms gap. Expect ONE LED lit per step.",
            TEST_ON_MS,
            TEST_DUTY_NUMERATOR * 100 / TEST_DUTY_DENOMINATOR,
            TEST_GAP_MS
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
            esp_idf_hal::delay::FreeRtos::delay_ms(TEST_ON_MS);
            self.set_duty(index, 0)?;
            esp_idf_hal::delay::FreeRtos::delay_ms(TEST_GAP_MS);
        }

        log::info!("=== Motor LED test complete ===");
        log::info!("Pass: four steps, one LED each, order M1 → M2 → M3 → M4");
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
