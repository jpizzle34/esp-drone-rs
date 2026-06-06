//! Board-specific GPIO and peripheral pin assignments.

mod elegoo_esp32_wroom32;
mod motor;

pub use elegoo_esp32_wroom32::*;
pub use motor::{Motor, MotorMeta};
