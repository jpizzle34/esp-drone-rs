//! External command and telemetry — Phase 2 (serial) and Phase 4 (WiFi/CRTP).
//!
//! ```text
//! comm/
//!   mod.rs     — this file
//!   serial.rs  — UART bench commands (motor duty, arm/disarm)
//!   crtp.rs    — CRTP framing + RPYT commander over WiFi/UDP
//! ```
//!
//! Comms tasks run on PRO CPU when WiFi is added; setpoints feed the stabilizer via a channel.

mod crtp;
mod serial;
