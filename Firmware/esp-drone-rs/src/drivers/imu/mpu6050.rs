//! MPU-6050 register access and scaled sample reads.
//!
//! Reference C: `Firmware/esp-drone/components/drivers/i2c_devices/mpu6050/`

use anyhow::Context;
use esp_idf_hal::delay::{FreeRtos, TickType};
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

/// Backstop if the hardware SCL timeout never fires. Long enough for a few
/// FreeRTOS ticks so it cannot expire in the same tick it started.
fn i2c_wait() -> esp_idf_hal::delay::TickType_t {
    TickType::new_millis(30).ticks()
}

/// One IMU sample in physical units (chip frame, uncalibrated).
#[derive(Debug, Clone, Copy)]
pub struct ImuSample {
    pub accel_g: [f32; 3],
    pub gyro_dps: [f32; 3],
    pub temp_c: f32,
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
        let id = self.read_who_am_i()?;

        // Same order as the C sensor bring-up: reset, wake on the internal
        // oscillator, then switch to the X-gyro PLL once the registers stick.
        self.write_reg(REG_PWR_MGMT_1, 0x80)?;
        FreeRtos::delay_ms(50);
        self.write_reg(REG_PWR_MGMT_1, 0x00)?;
        FreeRtos::delay_ms(100);
        self.write_reg(REG_PWR_MGMT_1, 0x01)?;
        FreeRtos::delay_ms(100);
        self.write_reg(REG_SMPLRT_DIV, 0)?;
        self.write_reg(REG_CONFIG, DLPF_BW_42)?;
        self.write_reg(REG_GYRO_CONFIG, GYRO_FS_2000 << 3)?;
        self.write_reg(REG_ACCEL_CONFIG, ACCEL_FS_16 << 3)?;
        FreeRtos::delay_ms(50);

        log::info!(
            "MPU6050 init OK (addr=0x{:02X}, WHO_AM_I=0x{id:02X})",
            self.addr
        );
        Ok(())
    }

    /// Clear a wedged controller and re-run the chip init.
    ///
    /// A fast move can NACK or clock-stretch the bus, or the MPU can come back
    /// from a glitch streaming zeros. Later reads then fail forever unless the
    /// FIFOs and the sensor are reset.
    pub fn recover(&mut self) -> anyhow::Result<()> {
        unsafe {
            let port = self.i2c.port();
            let _ = esp_idf_hal::sys::i2c_reset_tx_fifo(port);
            let _ = esp_idf_hal::sys::i2c_reset_rx_fifo(port);
        }
        // `init` checks WHO_AM_I before it resets the chip. Pulse the reset
        // first so a sensor that is NACKing still gets the wake-up write
        // once the controller FIFOs are clear.
        let _ = self.write_reg(REG_PWR_MGMT_1, 0x80);
        FreeRtos::delay_ms(50);
        self.init()
    }

    /// Retry WHO_AM_I so a sensor that is still powering up can answer.
    fn read_who_am_i(&mut self) -> anyhow::Result<u8> {
        let mut last = String::from("no response");
        for _ in 0..8 {
            match self.who_am_i() {
                Ok(id) if id == WHO_AM_I_EXPECTED => return Ok(id),
                Ok(id) => last = format!("WHO_AM_I=0x{id:02X}, expected 0x{WHO_AM_I_EXPECTED:02X}"),
                Err(err) => last = format!("{err:#}"),
            }
            FreeRtos::delay_ms(25);
        }
        anyhow::bail!("MPU6050 not responding at 0x{:02X} ({last})", self.addr)
    }

    pub fn who_am_i(&mut self) -> anyhow::Result<u8> {
        self.read_reg(REG_WHO_AM_I)
    }

    pub fn read_sample(&mut self) -> anyhow::Result<ImuSample> {
        let mut raw = [0u8; 14];
        self.i2c
            .write_read(self.addr, &[REG_ACCEL_XOUT_H], &mut raw, i2c_wait())
            .context("MPU6050 burst read failed")?;

        // An I2C drop often returns 14 zero bytes (`accel=0`, `temp≈36.5°C`).
        // High or low specific force is a real move — the filter gates gravity
        // correction — and must not be reported as a bus fault.
        if raw.iter().all(|&b| b == 0) {
            anyhow::bail!("MPU6050 returned all-zero sample (I2C drop / bus fault)");
        }

        let ax = i16::from_be_bytes([raw[0], raw[1]]);
        let ay = i16::from_be_bytes([raw[2], raw[3]]);
        let az = i16::from_be_bytes([raw[4], raw[5]]);
        let temp = i16::from_be_bytes([raw[6], raw[7]]);
        let gx = i16::from_be_bytes([raw[8], raw[9]]);
        let gy = i16::from_be_bytes([raw[10], raw[11]]);
        let gz = i16::from_be_bytes([raw[12], raw[13]]);

        let sample = ImuSample {
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
        };

        Ok(sample)
    }

    fn read_reg(&mut self, reg: u8) -> anyhow::Result<u8> {
        let mut buf = [0u8];
        self.i2c
            .write_read(self.addr, &[reg], &mut buf, i2c_wait())
            .with_context(|| format!("MPU6050 read reg 0x{reg:02X} failed"))?;
        Ok(buf[0])
    }

    fn write_reg(&mut self, reg: u8, value: u8) -> anyhow::Result<()> {
        self.i2c
            .write(self.addr, &[reg, value], i2c_wait())
            .with_context(|| format!("MPU6050 write reg 0x{reg:02X}=0x{value:02X} failed"))?;
        Ok(())
    }
}
