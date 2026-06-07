//! Brushed motor PWM via LEDC (matches ESP-Drone: 15 kHz, 8-bit duty).
//!
//! GPIO → LEDC wiring is implemented per board via [`MotorWiring`].

mod bench;
mod meta;
mod slot;
mod wiring;

use esp_idf_hal::ledc::{LedcDriver, LedcTimerDriver, LowSpeed};
use esp_idf_hal::units::Hertz;

pub use bench::{run_bench_test, BenchMode};
pub use meta::{
    assert_gpio_order, assert_slot_order, assert_table_gpio_order, build_table, MotorMeta,
    MotorPinMeta,
};
pub use slot::Motor;
pub use wiring::MotorWiring;

/// PWM frequency used by the C ESP-Drone `motors.c`.
const PWM_FREQUENCY: Hertz = Hertz(15_000);

pub struct Motors {
    _timer: LedcTimerDriver<'static, LowSpeed>,
    max_duty: u32,
    drivers: [LedcDriver<'static>; 4],
}

impl Motors {
    pub const fn pwm_frequency() -> Hertz {
        PWM_FREQUENCY
    }

    /// Construct from hardware already wired by a board's [`MotorWiring`] impl.
    pub fn from_drivers(
        timer: LedcTimerDriver<'static, LowSpeed>,
        drivers: [LedcDriver<'static>; 4],
    ) -> anyhow::Result<Self> {
        let max_duty = drivers[0].get_max_duty();
        debug_assert!(
            drivers
                .iter()
                .all(|driver| driver.get_max_duty() == max_duty),
            "all LEDC channels must share the same duty resolution"
        );

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
}
