//! MPU-6050 I2C driver — Phase 1a.
//!
//! Low-level register access only; sample scheduling lives in [`crate::sensors`].
//!
//! Reference C: `Firmware/esp-drone/components/drivers/i2c_devices/mpu6050/`

mod mpu6050;
mod wiring;

use esp_idf_hal::gpio::{InputPin, OutputPin};
use esp_idf_hal::i2c::{I2c, I2cConfig, I2cDriver};
use esp_idf_hal::units::Hertz;

pub use mpu6050::{Mpu6050, DEFAULT_ADDR};
pub use wiring::ImuWiring;

/// Wire I2C at 400 kHz with internal pull-ups (breakout boards usually have external ones too).
pub(crate) fn wire_i2c_bus<'d, I2C, SDA, SCL>(
    i2c: I2C,
    sda: SDA,
    scl: SCL,
) -> anyhow::Result<I2cDriver<'d>>
where
    I2C: I2c + 'd,
    SDA: InputPin + OutputPin + 'd,
    SCL: InputPin + OutputPin + 'd,
{
    let config = I2cConfig::new().baudrate(Hertz(400_000));
    Ok(I2cDriver::new(i2c, sda, scl, &config)?)
}
