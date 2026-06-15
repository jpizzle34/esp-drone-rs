//! MPU-6050 register access and scaled sample reads.
//!
//! Reference C: `Firmware/esp-drone/components/drivers/i2c_devices/mpu6050/`

use anyhow::Context;
use esp_idf_hal::delay::{FreeRtos, BLOCK};
use esp_idf_hal::i2c::I2cDriver;

const REG_SMPLRT_DIV: u8 = 0x19;
const REG_CONFIG: u8 = 0x1A;
const REG_GYRO_CONFIG: u8 = 0x1B;
const REG_ACCEL_CONFIG: u8 = 0x1C;
const REG_ACCEL_XOUT_H: u8 = 0x3B;
const REG_PWR_MGMT_1: u8 = 0x6B;
const REG_WHO_AM_I: u8 = 0x75;

const WHO_AM_I_EXPECTED: u8 = 0x68;

/// Default 7-bit I2C address (AD0 low).
pub const DEFAULT_ADDR: u8 = 0x68;

/// ±2000 °/s — matches ESP-Drone `SENSORS_GYRO_FS_CFG`.
const GYRO_FS_2000: u8 = 0x03;
/// ±16 g — matches ESP-Drone `SENSORS_ACCEL_FS_CFG`.
const ACCEL_FS_16: u8 = 0x03;
/// 42 Hz DLPF — matches ESP-Drone default (not the optional 256 Hz path).
const DLPF_BW_42: u8 = 0x03;

const GYRO_LSB_PER_DEG_S: f32 = (2.0 * 2000.0) / 65536.0;
const ACCEL_LSB_PER_G: f32 = (2.0 * 16.0) / 65536.0;

/// One IMU sample in physical units.
#[derive(Debug, Clone, Copy)]
pub struct ImuSample {
    pub accel_g: [f32; 3],
    pub gyro_dps: [f32; 3],
    pub temp_c: f32,
}

impl ImuSample {
    /// Roll and pitch from gravity vector (degrees). Yaw is not observable from accel alone.
    pub fn tilt_deg(&self) -> (f32, f32) {
        let [ax, ay, az] = self.accel_g;
        let roll = ay.atan2(az).to_degrees();
        let pitch = (-ax).atan2((ay * ay + az * az).sqrt()).to_degrees();
        (roll, pitch)
    }
}

pub struct Mpu6050<'d> {
    i2c: I2cDriver<'d>,
    addr: u8,
}

impl<'d> Mpu6050<'d> {
    pub fn new(i2c: I2cDriver<'d>, addr: u8) -> Self {
        Self { i2c, addr }
    }

    /// Reset, wake, and configure ranges/filter to match ESP-Drone defaults.
    pub fn init(&mut self) -> anyhow::Result<()> {
        let id = self.who_am_i()?;
        if id != WHO_AM_I_EXPECTED {
            anyhow::bail!(
                "MPU6050 WHO_AM_I mismatch: got 0x{id:02X}, expected 0x{WHO_AM_I_EXPECTED:02X}"
            );
        }

        self.write_reg(REG_PWR_MGMT_1, 0x80)?;
        FreeRtos::delay_ms(50);
        self.write_reg(REG_PWR_MGMT_1, 0x01)?; // wake, PLL with X gyro reference
        self.write_reg(REG_SMPLRT_DIV, 0)?;
        self.write_reg(REG_CONFIG, DLPF_BW_42)?;
        self.write_reg(REG_GYRO_CONFIG, GYRO_FS_2000 << 3)?;
        self.write_reg(REG_ACCEL_CONFIG, ACCEL_FS_16 << 3)?;
        FreeRtos::delay_ms(100);

        log::info!("MPU6050 init OK (addr=0x{:02X}, WHO_AM_I=0x{id:02X})", self.addr);
        Ok(())
    }

    pub fn who_am_i(&mut self) -> anyhow::Result<u8> {
        self.read_reg(REG_WHO_AM_I)
    }

    pub fn read_sample(&mut self) -> anyhow::Result<ImuSample> {
        let mut raw = [0u8; 14];
        self.i2c
            .write_read(self.addr, &[REG_ACCEL_XOUT_H], &mut raw, BLOCK)
            .context("MPU6050 burst read failed")?;

        let ax = i16::from_be_bytes([raw[0], raw[1]]);
        let ay = i16::from_be_bytes([raw[2], raw[3]]);
        let az = i16::from_be_bytes([raw[4], raw[5]]);
        let temp = i16::from_be_bytes([raw[6], raw[7]]);
        let gx = i16::from_be_bytes([raw[8], raw[9]]);
        let gy = i16::from_be_bytes([raw[10], raw[11]]);
        let gz = i16::from_be_bytes([raw[12], raw[13]]);

        Ok(ImuSample {
            accel_g: [
                ax as f32 * ACCEL_LSB_PER_G,
                ay as f32 * ACCEL_LSB_PER_G,
                az as f32 * ACCEL_LSB_PER_G,
            ],
            gyro_dps: [
                gx as f32 * GYRO_LSB_PER_DEG_S,
                gy as f32 * GYRO_LSB_PER_DEG_S,
                gz as f32 * GYRO_LSB_PER_DEG_S,
            ],
            temp_c: temp as f32 / 340.0 + 36.53,
        })
    }

    fn read_reg(&mut self, reg: u8) -> anyhow::Result<u8> {
        let mut buf = [0u8];
        self.i2c
            .write_read(self.addr, &[reg], &mut buf, BLOCK)
            .with_context(|| format!("MPU6050 read reg 0x{reg:02X} failed"))?;
        Ok(buf[0])
    }

    fn write_reg(&mut self, reg: u8, value: u8) -> anyhow::Result<()> {
        self.i2c
            .write(self.addr, &[reg, value], BLOCK)
            .with_context(|| format!("MPU6050 write reg 0x{reg:02X}=0x{value:02X} failed"))?;
        Ok(())
    }
}
