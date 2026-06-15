//! Board-specific GPIO → I2C wiring.
//!
//! Each board implements [`ImuWiring`] for its typed [`ImuPins`] bundle.
//! The driver crate defines the trait; the board crate provides the impl,
//! keeping GPIO types out of generic driver code.

use esp_idf_hal::i2c::I2c;

use super::Mpu6050;

/// Wire a board's IMU I2C pin bundle to an I2C controller.
pub trait ImuWiring {
    fn wire<I2C: I2c + 'static>(self, i2c: I2C) -> anyhow::Result<Mpu6050<'static>>;
}
