//! Attitude estimation.
//!
//! ```text
//! estimation/
//!   mod.rs    — this file
//!   fusion.rs — Mahony AHRS (gyro + accel)
//! ```
//!
//! Reference C: `sensfusion6.c` (Mahony path). Kalman stays deferred.

mod fusion;

pub use fusion::{AttitudeDeg, Mahony};
