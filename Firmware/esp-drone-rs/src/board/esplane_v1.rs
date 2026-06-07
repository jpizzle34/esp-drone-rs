//! ESPLANE_V1 production PCB pin profile — not yet implemented.
//!
//! Mixed-header layout (signals on left and right devkit headers).

/// Onboard status LED (GPIO2).
pub const STATUS_LED: u8 = 2;
/// MPU-6050 I2C data.
pub const I2C0_SDA: u8 = 21;
/// MPU-6050 I2C clock.
pub const I2C0_SCL: u8 = 22;
/// Motor M1 — LEDC ch0.
pub const MOTOR_M1: u8 = 4;
/// Motor M2 — LEDC ch1.
pub const MOTOR_M2: u8 = 33;
/// Motor M3 — LEDC ch2.
pub const MOTOR_M3: u8 = 32;
/// Motor M4 — LEDC ch3.
pub const MOTOR_M4: u8 = 25;

compile_error!(
    "board-esplane-v1 is not implemented yet — build with the default board-elegoo-poc feature"
);
