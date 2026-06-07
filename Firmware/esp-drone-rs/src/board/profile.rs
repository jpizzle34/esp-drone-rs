//! Board profile trait — one implementation per hardware pin map.
//!
//! Select the active profile with a Cargo feature (`board-elegoo-poc` default,
//! `board-esplane-v1` when implemented). Exactly one `board-*` feature must be enabled.

use esp_idf_hal::gpio::Pins;

use crate::drivers::motors::{MotorMeta, MotorPinMeta, MotorWiring};

/// GPIO number + devkit header label for logs and bench wiring.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct BoardPin {
    pub gpio: u8,
    pub header: &'static str,
}

/// Typed pin bundle returned by a board profile's claim entry point.
pub trait HasMotorPins {
    type MotorPins: MotorWiring;
    type StatusLedPin;

    fn split(self) -> (Self::StatusLedPin, Self::MotorPins);
}

/// Hardware pin map for one board + wiring profile.
pub trait BoardProfile {
    type DronePins: HasMotorPins;

    const BOARD_NAME: &'static str;
    const PIN_PROFILE: &'static str;

    /// GPIO numbers for M1→M4 in LEDC ch0→ch3 wiring order.
    const MOTOR_GPIO_ORDER: [u8; 4];

    fn take(pins: Pins) -> Self::DronePins;
    fn status_led() -> BoardPin;
    fn imu_i2c() -> (BoardPin, BoardPin);
    #[allow(dead_code)] // part of the board contract; used when adding new profiles
    fn motor_pin_meta() -> &'static [MotorPinMeta; 4];
    fn motor_table() -> &'static [MotorMeta; 4];
    fn log_pinout();
}
