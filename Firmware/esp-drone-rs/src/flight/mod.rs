//! Closed-loop flight control — Phase 3+.
//!
//! ```text
//! flight/
//!   mod.rs         — this file
//!   stabilizer.rs  — 1 kHz control loop (APP CPU)
//!   pid.rs         — attitude PID blocks
//!   mixer.rs       — X-quad motor mixing
//! ```
//!
//! Reference C: `Firmware/esp-drone/components/core/crazyflie/modules/src/stabilizer.c`

mod mixer;
mod pid;
mod stabilizer;
