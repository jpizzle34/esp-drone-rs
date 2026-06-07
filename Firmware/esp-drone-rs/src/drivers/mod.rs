//! Hardware drivers — peripheral control decoupled from board pin maps.
//!
//! ```text
//! drivers/
//!   mod.rs          — this file; re-exports active drivers
//!   motors/         — brushed motor PWM (LEDC), bench tests  [active]
//!   imu/            — MPU6050 I2C                            [stub]
//!   power/          — battery voltage / current                [stub]
//!
//! Higher layers (not drivers):
//!   sensors/        — sample tasks, calibration
//!   estimation/     — attitude fusion
//!   flight/         — stabilizer, PID, mixer
//!   comm/           — serial + CRTP / WiFi
//!   safety/         — arming gate, failsafe
//! ```

pub mod imu;
pub mod motors;
pub mod power;
