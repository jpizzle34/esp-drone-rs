//! Hardware drivers — peripheral control decoupled from board pin maps.
//!
//! ```text
//! drivers/
//!   mod.rs          — this file; re-exports active drivers
//!   motors/         — brushed motor PWM (LEDC), bench tests  [active]
//!   imu/            — MPU6050 I2C + [`ImuWiring`]                 [active]
//!   power/          — battery voltage / current                [stub]
//!
//! Higher layers (not drivers):
//!   sensors/        — sample tasks, calibration [active: IMU 250 Hz + gyro bias]
//!   estimation/     — Mahony attitude [active]
//!   flight/         — attitude PID, X mixer, 250 Hz stabilizer [active]
//!   comm/           — serial bench REPL [active]; CRTP / WiFi [stub]
//!   safety/         — arming gate, failsafe
//! ```

pub mod imu;
pub mod motors;
pub mod power;
