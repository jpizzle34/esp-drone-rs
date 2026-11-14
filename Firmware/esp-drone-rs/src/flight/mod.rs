//! Closed-loop attitude — angle/rate PID, X-quad mix, 250 Hz stabilizer.
//!
//! ```text
//! flight/
//!   mod.rs         — this file
//!   pid.rs         — PID block
//!   attitude.rs    — angle → rate cascade
//!   mixer.rs       — X-quad motor mixing
//!   stabilizer.rs  — control task (APP-side; rate matches the IMU)
//! ```
//!
//! Reference C: `stabilizer.c`, `controller_pid.c`, `attitude_pid_controller.c`,
//! `power_distribution_stock.c`.

mod attitude;
mod mixer;
mod pid;
mod stabilizer;

pub use attitude::PID_GAIN_MAX;
pub use stabilizer::{spawn, AttitudeService};
