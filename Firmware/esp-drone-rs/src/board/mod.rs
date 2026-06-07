//! Board-specific GPIO and peripheral pin assignments.
//!
//! Select exactly one profile via Cargo features:
//!
//! - `board-elegoo-poc` (default) — Elegoo ESP32-WROOM-32 devkit, left-header POC
//! - `board-esplane-v1` — production ESPLANE PCB (not yet implemented)
//!
//! Each board file implements [`BoardProfile`] with typed pin bundles and a single
//! claim entry point ([`BoardProfile::take`]).

mod profile;

#[cfg(feature = "board-elegoo-poc")]
mod elegoo_esp32_wroom32;

#[cfg(feature = "board-esplane-v1")]
mod esplane_v1;

#[cfg(all(feature = "board-elegoo-poc", feature = "board-esplane-v1"))]
compile_error!("Enable only one board-* feature at a time");

#[cfg(not(any(feature = "board-elegoo-poc", feature = "board-esplane-v1")))]
compile_error!("Select a board profile: board-elegoo-poc (default) or board-esplane-v1");

#[cfg(feature = "board-elegoo-poc")]
pub use elegoo_esp32_wroom32::ElegooPoc as ActiveBoard;

#[cfg(feature = "board-esplane-v1")]
pub use esplane_v1::EsplaneV1 as ActiveBoard;

pub use profile::{BoardProfile, HasMotorPins};
