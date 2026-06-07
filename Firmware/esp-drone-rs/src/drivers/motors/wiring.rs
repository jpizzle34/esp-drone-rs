//! Board-specific GPIO → LEDC wiring.
//!
//! Each board implements [`MotorWiring`] for its typed [`MotorPins`] bundle.
//! The driver crate defines the trait; the board crate provides the impl,
//! keeping GPIO types out of generic driver code.

use esp_idf_hal::ledc::LEDC;

use super::Motors;

/// Wire a board's motor GPIO bundle to LEDC channels (M1→ch0 … M4→ch3).
pub trait MotorWiring {
    /// GPIO numbers for M1→M4 in the same order as LEDC ch0→ch3 wiring.
    const GPIO_ORDER: [u8; 4];

    fn wire(self, ledc: LEDC) -> anyhow::Result<Motors>;
}
