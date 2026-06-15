//! IMU sample task — optional serial stream (off by default).

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::i2c::I2c;

use crate::board::{ActiveBoard, BoardProfile};
use crate::drivers::imu::{ImuWiring, Mpu6050};

/// Default IMU log rate when `imu on` is used without an explicit hz (`10 Hz`).
pub const DEFAULT_STREAM_HZ: u32 = 10;

const MIN_STREAM_HZ: u32 = 1;
const MAX_STREAM_HZ: u32 = 100;
const IDLE_POLL_MS: u32 = 100;

/// Shared IMU stream controls (stream off by default).
#[derive(Clone)]
pub struct ImuService {
    stream_enabled: Arc<AtomicBool>,
    stream_period_ms: Arc<AtomicU32>,
}

impl ImuService {
    pub fn set_stream(&self, enabled: bool) {
        self.stream_enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn stream_enabled(&self) -> bool {
        self.stream_enabled.load(Ordering::Relaxed)
    }

    pub fn set_stream_hz(&self, hz: u32) {
        let hz = hz.clamp(MIN_STREAM_HZ, MAX_STREAM_HZ);
        let period_ms = (1000 / hz).max(1);
        self.stream_period_ms.store(period_ms, Ordering::Relaxed);
    }

    pub fn stream_hz(&self) -> u32 {
        let period_ms = self.stream_period_ms.load(Ordering::Relaxed).max(1);
        1000 / period_ms
    }
}

/// Wire and init the MPU-6050. Stream is **off** until `imu on` on serial.
pub fn init<P, I2C>(imu_pins: P, i2c: I2C) -> anyhow::Result<Option<ImuService>>
where
    P: ImuWiring,
    I2C: I2c + 'static,
{
    let (imu_sda, imu_scl) = ActiveBoard::imu_i2c();
    log::info!(
        "IMU I2C: SDA=GPIO{} ({}), SCL=GPIO{} ({})",
        imu_sda.gpio,
        imu_sda.header,
        imu_scl.gpio,
        imu_scl.header,
    );

    let mut mpu = imu_pins.wire(i2c)?;
    match mpu.init() {
        Ok(()) => {
            let service = ImuService {
                stream_enabled: Arc::new(AtomicBool::new(false)),
                stream_period_ms: Arc::new(AtomicU32::new(1000 / DEFAULT_STREAM_HZ)),
            };
            let _imu_task = spawn_sample_task(mpu, service.clone())?;
            log::info!("IMU ready — serial: 'imu on' to stream, 'imu off' to stop");
            Ok(Some(service))
        }
        Err(err) => {
            log::warn!("IMU init failed — continuing without sensor: {err:#}");
            Ok(None)
        }
    }
}

fn spawn_sample_task(
    mpu: Mpu6050<'static>,
    service: ImuService,
) -> anyhow::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("imu_sample".into())
        .stack_size(4096)
        .spawn(move || {
            let mut mpu = mpu;
            let stream_enabled = service.stream_enabled;
            let stream_period_ms = service.stream_period_ms;

            loop {
                if stream_enabled.load(Ordering::Relaxed) {
                    let period = stream_period_ms.load(Ordering::Relaxed).max(1);
                    match mpu.read_sample() {
                        Ok(sample) => {
                            let (roll, pitch) = sample.tilt_deg();
                            let [ax, ay, az] = sample.accel_g;
                            let [gx, gy, gz] = sample.gyro_dps;
                            println!(
                                "IMU accel=({ax:+.2},{ay:+.2},{az:+.2})g \
                                 gyro=({gx:+.0},{gy:+.0},{gz:+.0})°/s \
                                 tilt roll={roll:+.1}° pitch={pitch:+.1}° \
                                 temp={:.1}°C",
                                sample.temp_c
                            );
                        }
                        Err(err) => log::error!("IMU read failed: {err:#}"),
                    }
                    FreeRtos::delay_ms(period);
                } else {
                    FreeRtos::delay_ms(IDLE_POLL_MS);
                }
            }
        })
        .map_err(|err| anyhow::anyhow!("failed to spawn IMU sample task: {err}"))
}
