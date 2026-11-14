//! IMU sample task — 250 Hz read, gyro bias, Mahony attitude.
//!
//! The serial stream is off by default. Sampling and fusion run either way so
//! `imu` can report the latest attitude.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::i2c::I2c;

use crate::board::{ActiveBoard, BoardProfile};
use crate::drivers::imu::{ImuSample, ImuWiring, Mpu6050};
use crate::estimation::{AttitudeDeg, Mahony};

/// Default IMU log rate when `imu on` is used without an explicit hz (`10 Hz`).
pub const DEFAULT_STREAM_HZ: u32 = 10;

/// Continuous sample rate. The DLPF is 42 Hz; 250 Hz keeps the latest sample fresh.
pub const SAMPLE_HZ: u32 = 250;

const MIN_STREAM_HZ: u32 = 1;
const MAX_STREAM_HZ: u32 = 100;
const SAMPLE_PERIOD_MS: u32 = 1000 / SAMPLE_HZ;

/// Thrown away after init — the first conversions are often zero.
const CAL_DISCARD: u32 = 16;
/// Still-board average. At 250 Hz this is about one second.
const CAL_SAMPLES: u32 = 256;
/// Bias above this usually means the board moved during calibration.
const BIAS_WARN_DPS: f32 = 20.0;

/// Consecutive bus failures before resetting the MPU and the I2C FIFOs.
const BUS_RECOVER_AFTER: u32 = 8;
/// Past this streak, say so on the dashboard — a reset is not bringing it back.
const BUS_WIRING_HINT_AFTER: u32 = 200;

/// Latest bias-corrected sample and fused attitude.
#[derive(Debug, Clone, Copy)]
pub struct ImuSnapshot {
    pub accel_g: [f32; 3],
    pub gyro_dps: [f32; 3],
    pub temp_c: f32,
    pub attitude: AttitudeDeg,
    /// When this sample was stored. The stabilizer disarms if it goes stale.
    pub at: Instant,
}

/// Gyro bias plus the mean accel used to seed attitude.
struct GyroCal {
    bias_dps: [f32; 3],
    accel_g: [f32; 3],
}

/// Shared IMU controls. Stream is off until `imu on`.
#[derive(Clone)]
pub struct ImuService {
    stream_enabled: Arc<AtomicBool>,
    stream_period_ms: Arc<AtomicU32>,
    cal_request: Arc<AtomicBool>,
    latest: Arc<Mutex<Option<ImuSnapshot>>>,
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

    /// Ask the sample task to re-average the gyro and re-seed roll/pitch from gravity.
    pub fn request_cal(&self) {
        self.cal_request.store(true, Ordering::Relaxed);
    }

    pub fn latest(&self) -> Option<ImuSnapshot> {
        self.latest.lock().ok().and_then(|slot| *slot)
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
    if let Err(err) = mpu.init() {
        log::warn!("IMU init failed — continuing without sensor: {err:#}");
        return Ok(None);
    }

    log::info!("IMU calibrating gyro — keep the board still");
    let cal = match calibrate(&mut mpu) {
        Ok(cal) => cal,
        Err(err) => {
            log::warn!("IMU gyro cal failed — continuing without sensor: {err:#}");
            return Ok(None);
        }
    };
    log_bias(&cal);

    let mut fusion = Mahony::new();
    fusion.seed_from_accel(cal.accel_g);

    let service = ImuService {
        stream_enabled: Arc::new(AtomicBool::new(false)),
        stream_period_ms: Arc::new(AtomicU32::new(1000 / DEFAULT_STREAM_HZ)),
        cal_request: Arc::new(AtomicBool::new(false)),
        latest: Arc::new(Mutex::new(None)),
    };
    let _imu_task = spawn_sample_task(mpu, fusion, cal.bias_dps, service.clone())?;
    log::info!(
        "IMU ready ({SAMPLE_HZ} Hz Mahony) — serial: 'imu on' to stream, 'imu cal' to re-zero"
    );
    Ok(Some(service))
}

fn calibrate(mpu: &mut Mpu6050<'static>) -> anyhow::Result<GyroCal> {
    for _ in 0..CAL_DISCARD {
        let _ = read_valid_sample(mpu)?;
        FreeRtos::delay_ms(SAMPLE_PERIOD_MS);
    }

    let mut gyro = [0.0f32; 3];
    let mut accel = [0.0f32; 3];
    for _ in 0..CAL_SAMPLES {
        let sample = read_valid_sample(mpu)?;
        for axis in 0..3 {
            gyro[axis] += sample.gyro_dps[axis];
            accel[axis] += sample.accel_g[axis];
        }
        FreeRtos::delay_ms(SAMPLE_PERIOD_MS);
    }

    let n = CAL_SAMPLES as f32;
    Ok(GyroCal {
        bias_dps: gyro.map(|v| v / n),
        accel_g: accel.map(|v| v / n),
    })
}

fn read_valid_sample(mpu: &mut Mpu6050<'static>) -> anyhow::Result<ImuSample> {
    mpu.read_sample()
}

fn log_bias(cal: &GyroCal) {
    let [bx, by, bz] = cal.bias_dps;
    log::info!("IMU gyro bias ({bx:+.2},{by:+.2},{bz:+.2}) °/s");
    let mag = (bx * bx + by * by + bz * bz).sqrt();
    if mag > BIAS_WARN_DPS {
        log::warn!(
            "IMU gyro bias {mag:.1} °/s is large — reset or 'imu cal' while the board is still"
        );
    }
}

fn spawn_sample_task(
    mpu: Mpu6050<'static>,
    fusion: Mahony,
    bias: [f32; 3],
    service: ImuService,
) -> anyhow::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("imu_sample".into())
        .stack_size(8192)
        .spawn(move || {
            let mut mpu = mpu;
            let mut fusion = fusion;
            let mut bias = bias;
            let mut last = Instant::now();
            let mut samples_since_print: u32 = 0;
            let mut read_failures: u32 = 0;

            loop {
                if service.cal_request.swap(false, Ordering::Relaxed) {
                    match calibrate(&mut mpu) {
                        Ok(cal) => {
                            bias = cal.bias_dps;
                            fusion.seed_from_accel(cal.accel_g);
                            read_failures = 0;
                            let [bx, by, bz] = bias;
                            println!(
                                "IMU gyro bias ({bx:+.2},{by:+.2},{bz:+.2}) °/s — attitude re-aligned, yaw reset"
                            );
                        }
                        Err(err) => {
                            log::error!("IMU gyro cal failed: {err:#}");
                            if let Err(reset_err) = mpu.recover() {
                                log::error!("IMU sensor reset failed: {reset_err:#}");
                            }
                        }
                    }
                    // Drop the fault interval so the next good sample does not
                    // integrate a multi-second dt into the quaternion.
                    last = Instant::now();
                    samples_since_print = 0;
                    continue;
                }

                let started = Instant::now();
                match read_valid_sample(&mut mpu) {
                    Ok(sample) => {
                        if read_failures >= BUS_RECOVER_AFTER {
                            log::info!(
                                "IMU recovered after {read_failures} bad sample(s) — holding fusion across the gap"
                            );
                            if service.stream_enabled.load(Ordering::Relaxed) {
                                println!("IMU recovered — stream resumes");
                            }
                        }
                        read_failures = 0;

                        let now = Instant::now();
                        let dt = now.duration_since(last).as_secs_f32().clamp(0.0002, 0.05);
                        last = now;

                        let gyro = correct_gyro(&sample, bias);
                        let attitude = fusion.update(gyro, sample.accel_g, dt);
                        let snapshot = ImuSnapshot {
                            accel_g: sample.accel_g,
                            gyro_dps: gyro,
                            temp_c: sample.temp_c,
                            attitude,
                            at: now,
                        };
                        if let Ok(mut slot) = service.latest.lock() {
                            *slot = Some(snapshot);
                        }

                        samples_since_print = samples_since_print.saturating_add(1);
                        if service.stream_enabled.load(Ordering::Relaxed) {
                            let every =
                                print_every(service.stream_period_ms.load(Ordering::Relaxed));
                            if samples_since_print >= every {
                                samples_since_print = 0;
                                print_sample(&snapshot);
                            }
                        } else {
                            samples_since_print = 0;
                        }
                    }
                    Err(err) => {
                        // Bus error or an all-zero burst — not a fast move.
                        // Hold the last attitude and reset `last` so the outage
                        // does not dump a huge dt into Mahony. A latched MPU
                        // keeps failing until the chip and the I2C FIFOs are reset.
                        read_failures = read_failures.saturating_add(1);
                        last = Instant::now();
                        if read_failures == 1 {
                            log::warn!("IMU read failed — holding last attitude: {err:#}");
                        }
                        let persist = read_failures == BUS_RECOVER_AFTER
                            || (read_failures > BUS_RECOVER_AFTER && read_failures % 50 == 0);
                        if persist {
                            log::error!(
                                "IMU fault ({read_failures}) — holding last attitude: {err:#}"
                            );
                            if service.stream_enabled.load(Ordering::Relaxed) {
                                if read_failures >= BUS_WIRING_HINT_AFTER {
                                    println!(
                                        "IMU fault ({read_failures}) — holding last attitude (check SDA/SCL/3V3/GND)"
                                    );
                                } else {
                                    println!(
                                        "IMU fault ({read_failures}) — holding last attitude, resetting sensor"
                                    );
                                }
                            }
                            match mpu.recover() {
                                Ok(()) => {
                                    log::warn!("IMU sensor reset after {read_failures} bad reads")
                                }
                                Err(reset_err) => {
                                    log::error!("IMU sensor reset failed: {reset_err:#}")
                                }
                            }
                            last = Instant::now();
                        }
                    }
                }

                let elapsed = started.elapsed().as_millis() as u32;
                if elapsed < SAMPLE_PERIOD_MS {
                    FreeRtos::delay_ms(SAMPLE_PERIOD_MS - elapsed);
                }
            }
        })
        .map_err(|err| anyhow::anyhow!("failed to spawn IMU sample task: {err}"))
}

fn correct_gyro(sample: &ImuSample, bias: [f32; 3]) -> [f32; 3] {
    [
        sample.gyro_dps[0] - bias[0],
        sample.gyro_dps[1] - bias[1],
        sample.gyro_dps[2] - bias[2],
    ]
}

fn print_every(period_ms: u32) -> u32 {
    (period_ms.max(1) / SAMPLE_PERIOD_MS).max(1)
}

fn print_sample(snapshot: &ImuSnapshot) {
    let [ax, ay, az] = snapshot.accel_g;
    let [gx, gy, gz] = snapshot.gyro_dps;
    let AttitudeDeg { roll, pitch, yaw } = snapshot.attitude;
    println!(
        "IMU accel=({ax:+.2},{ay:+.2},{az:+.2})g \
         gyro=({gx:+.0},{gy:+.0},{gz:+.0})°/s \
         roll={roll:+.1}° pitch={pitch:+.1}° yaw={yaw:+.1}° \
         temp={:.1}°C",
        snapshot.temp_c
    );
}
