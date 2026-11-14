//! Attitude stabilizer.
//!
//! Runs at the IMU sample rate (250 Hz). The task reads the latest Mahony
//! attitude, runs the angle/rate cascade, mixes to M1–M4, and writes duty
//! through [`MotorGate`]. Output starts only after `att on` while armed, and
//! thrust `0` keeps every motor at 0. A stale or missing sample disarms.
//!
//! The loop is not pinned to a core yet. 1 kHz waits on a faster sensor — at
//! 250 Hz the MPU-6050 already limits the new information per tick.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use esp_idf_hal::delay::FreeRtos;

use super::attitude::{
    actuator_to_percent, format_pid_line, format_pid_report, round_gain, wrap_deg,
    AttitudeController, CascadeGains, PidAxis,
};
use super::mixer::mix_x;
use super::pid::PidGains;
use crate::estimation::AttitudeDeg;
use crate::safety::{DisarmReason, MotorGate};
use crate::sensors::ImuService;

/// Control rate. Matches [`crate::sensors::imu::SAMPLE_HZ`].
pub const CONTROL_HZ: u32 = 250;

/// Commanded roll / pitch limit, degrees.
pub const MAX_TILT_DEG: f32 = 30.0;

const PERIOD_MS: u32 = 1000 / CONTROL_HZ;
const PRINT_EVERY: u32 = CONTROL_HZ / 10;
const STALE: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy)]
struct Target {
    roll_deg: f32,
    pitch_deg: f32,
    yaw_deg: f32,
    thrust_pct: u32,
}

impl Default for Target {
    fn default() -> Self {
        Self {
            roll_deg: 0.0,
            pitch_deg: 0.0,
            yaw_deg: 0.0,
            thrust_pct: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Report {
    duties: [u32; 4],
    measured: Option<AttitudeDeg>,
}

impl Default for Report {
    fn default() -> Self {
        Self {
            duties: [0; 4],
            measured: None,
        }
    }
}

/// Shared attitude-mode setpoints. Off until `att on`.
///
/// PID gains live here too. The stabilizer copies them when [`Self::gains_generation`]
/// changes, so a `pid` command applies on the next control tick.
#[derive(Clone)]
pub struct AttitudeService {
    imu_ready: bool,
    enabled: Arc<AtomicBool>,
    target: Arc<Mutex<Target>>,
    report: Arc<Mutex<Report>>,
    gains: Arc<Mutex<CascadeGains>>,
    gains_gen: Arc<AtomicU32>,
}

impl AttitudeService {
    pub fn new(imu_ready: bool) -> Self {
        Self {
            imu_ready,
            enabled: Arc::new(AtomicBool::new(false)),
            target: Arc::new(Mutex::new(Target::default())),
            report: Arc::new(Mutex::new(Report::default())),
            gains: Arc::new(Mutex::new(CascadeGains::defaults())),
            gains_gen: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn gains_generation(&self) -> u32 {
        self.gains_gen.load(Ordering::Acquire)
    }

    pub fn copy_gains(&self) -> Option<CascadeGains> {
        self.gains.lock().ok().map(|gains| *gains)
    }

    /// Six `OK pid …` lines, angle loops then rate loops.
    pub fn pid_report(&self) -> Result<String, String> {
        let gains = self.lock_gains()?;
        Ok(format_pid_report(&gains))
    }

    /// Restore the ESP32 Crazyflie defaults and report them.
    pub fn reset_pid(&self) -> Result<String, String> {
        let defaults = CascadeGains::defaults();
        {
            let mut gains = self.lock_gains()?;
            if *gains != defaults {
                *gains = defaults;
                self.gains_gen.fetch_add(1, Ordering::Release);
            }
        }
        self.pid_report()
    }

    /// Store one loop. Gains are rounded to 0.0001. A real change bumps the generation.
    pub fn set_pid_loop(&self, name: &str, kp: f32, ki: f32, kd: f32) -> Result<String, String> {
        let axis = PidAxis::parse(name).ok_or_else(|| {
            format!(
                "unknown pid loop '{name}' — use roll, pitch, yaw, roll-rate, pitch-rate, yaw-rate"
            )
        })?;
        let next = PidGains {
            kp: round_gain(kp),
            ki: round_gain(ki),
            kd: round_gain(kd),
        };
        if !gain_in_range(next.kp) || !gain_in_range(next.ki) || !gain_in_range(next.kd) {
            return Err(format!(
                "gain must be 0..={}",
                super::attitude::PID_GAIN_MAX
            ));
        }
        {
            let mut gains = self.lock_gains()?;
            if gains.get(axis) != next {
                gains.set(axis, next);
                self.gains_gen.fetch_add(1, Ordering::Release);
            }
        }
        Ok(format_pid_line(axis, next))
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    /// Hold level, capture yaw, and force thrust to 0. Rejected if already on.
    pub fn enable(&self, yaw_deg: f32) -> Result<(), String> {
        if !self.imu_ready {
            return Err("IMU not available".to_string());
        }
        if self.is_enabled() {
            return Err("already on".to_string());
        }
        self.write_target(Target {
            roll_deg: 0.0,
            pitch_deg: 0.0,
            yaw_deg: wrap_deg(yaw_deg),
            thrust_pct: 0,
        })?;
        self.enabled.store(true, Ordering::Release);
        Ok(())
    }

    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Release);
        if let Ok(mut report) = self.report.lock() {
            report.duties = [0; 4];
        }
    }

    pub fn set_thrust(&self, percent: u32) -> Result<u32, String> {
        self.require_enabled()?;
        if percent > 100 {
            return Err(format!("thrust must be 0-100, got {percent}"));
        }
        self.update_target(|target| target.thrust_pct = percent)?;
        Ok(percent)
    }

    pub fn set_roll(&self, deg: f32) -> Result<f32, String> {
        self.set_tilt(|target| &mut target.roll_deg, deg, "roll")
    }

    pub fn set_pitch(&self, deg: f32) -> Result<f32, String> {
        self.set_tilt(|target| &mut target.pitch_deg, deg, "pitch")
    }

    pub fn set_yaw(&self, deg: f32) -> Result<f32, String> {
        self.require_enabled()?;
        if !deg.is_finite() {
            return Err("invalid yaw".to_string());
        }
        let wrapped = wrap_deg(deg);
        self.update_target(|target| target.yaw_deg = wrapped)?;
        Ok(wrapped)
    }

    /// Level roll/pitch and recapture yaw. Thrust is left alone.
    pub fn level(&self, yaw_deg: f32) -> Result<f32, String> {
        self.require_enabled()?;
        let yaw = wrap_deg(yaw_deg);
        self.update_target(|target| {
            target.roll_deg = 0.0;
            target.pitch_deg = 0.0;
            target.yaw_deg = yaw;
        })?;
        Ok(yaw)
    }

    pub fn status_line(&self) -> String {
        let enabled = self.is_enabled();
        let target = self.target.lock().ok().map(|target| *target);
        let report = self.report.lock().ok().map(|report| *report);
        let Some(target) = target else {
            return "attitude unavailable".to_string();
        };
        let mode = if enabled { "on" } else { "off" };
        let measured = report.and_then(|report| report.measured);
        let duties = report.map(|report| report.duties).unwrap_or([0; 4]);
        match measured {
            Some(measured) => format!(
                "attitude {mode} — thrust {}% roll {:+.1}° pitch {:+.1}° yaw {:+.1}° | \
                 measured roll {:+.1}° pitch {:+.1}° yaw {:+.1}° | \
                 M1={} M2={} M3={} M4={}",
                target.thrust_pct,
                target.roll_deg,
                target.pitch_deg,
                target.yaw_deg,
                measured.roll,
                measured.pitch,
                measured.yaw,
                duties[0],
                duties[1],
                duties[2],
                duties[3],
            ),
            None => format!(
                "attitude {mode} — thrust {}% roll {:+.1}° pitch {:+.1}° yaw {:+.1}° | no sample yet",
                target.thrust_pct, target.roll_deg, target.pitch_deg, target.yaw_deg,
            ),
        }
    }

    fn set_tilt(
        &self,
        slot: impl FnOnce(&mut Target) -> &mut f32,
        deg: f32,
        name: &str,
    ) -> Result<f32, String> {
        self.require_enabled()?;
        if !deg.is_finite() || !(-MAX_TILT_DEG..=MAX_TILT_DEG).contains(&deg) {
            return Err(format!(
                "{name} must be {:+.0}..{:+.0} deg",
                -MAX_TILT_DEG, MAX_TILT_DEG
            ));
        }
        self.update_target(|target| *slot(target) = deg)?;
        Ok(deg)
    }

    fn require_enabled(&self) -> Result<(), String> {
        if !self.imu_ready {
            return Err("IMU not available".to_string());
        }
        if !self.is_enabled() {
            return Err("attitude mode is off — type 'att on' first".to_string());
        }
        Ok(())
    }

    fn write_target(&self, target: Target) -> Result<(), String> {
        let mut slot = self
            .target
            .lock()
            .map_err(|_| "attitude mutex poisoned".to_string())?;
        *slot = target;
        Ok(())
    }

    fn update_target(&self, update: impl FnOnce(&mut Target)) -> Result<(), String> {
        let mut slot = self
            .target
            .lock()
            .map_err(|_| "attitude mutex poisoned".to_string())?;
        update(&mut slot);
        Ok(())
    }

    fn target(&self) -> Option<Target> {
        self.target.lock().ok().map(|target| *target)
    }

    fn lock_gains(&self) -> Result<std::sync::MutexGuard<'_, CascadeGains>, String> {
        self.gains
            .lock()
            .map_err(|_| "attitude mutex poisoned".to_string())
    }

    fn publish(&self, duties: [u32; 4], measured: AttitudeDeg) {
        if let Ok(mut report) = self.report.lock() {
            report.duties = duties;
            report.measured = Some(measured);
        }
    }
}

/// Spawn the stabilizer. With no IMU the task stays idle and `att on` fails.
pub fn spawn(
    gate: Arc<MotorGate>,
    imu: Option<ImuService>,
    service: AttitudeService,
) -> anyhow::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("stabilizer".into())
        .stack_size(8192)
        .spawn(move || {
            let Some(imu) = imu else {
                log::info!("stabilizer idle — no IMU");
                loop {
                    FreeRtos::delay_ms(1000);
                }
            };
            run_loop(gate, imu, service);
        })
        .map_err(|err| anyhow::anyhow!("failed to spawn stabilizer task: {err}"))
}

fn run_loop(gate: Arc<MotorGate>, imu: ImuService, service: AttitudeService) {
    let mut controller = AttitudeController::new();
    let mut applied_gains = 0u32;
    let mut last = Instant::now();
    let mut tick: u32 = 0;
    log::info!("stabilizer ready ({CONTROL_HZ} Hz) — serial: 'att on' when armed");

    loop {
        let started = Instant::now();
        applied_gains = sync_gains(&service, &mut controller, applied_gains);
        let enabled = service.is_enabled();
        let armed = gate.is_armed();

        if !enabled || !armed {
            if enabled && !armed {
                service.disable();
            }
            controller.reset();
            last = started;
            delay_rest(started);
            continue;
        }

        let sample = imu.latest().filter(|sample| sample_ok(sample, started));
        let Some(sample) = sample else {
            fail_closed(&gate, &service, "IMU sample lost");
            controller.reset();
            last = started;
            delay_rest(started);
            continue;
        };

        let Some(target) = service.target() else {
            delay_rest(started);
            continue;
        };

        let dt = started
            .saturating_duration_since(last)
            .as_secs_f32()
            .clamp(0.001, 0.02);
        last = started;

        let duties = if target.thrust_pct == 0 {
            controller.reset();
            [0; 4]
        } else {
            let desired = AttitudeDeg {
                roll: target.roll_deg,
                pitch: target.pitch_deg,
                yaw: target.yaw_deg,
            };
            let actuators = controller.update(desired, sample.attitude, sample.gyro_dps, dt);
            mix_x(
                target.thrust_pct as f32,
                actuator_to_percent(actuators.roll, true),
                actuator_to_percent(actuators.pitch, true),
                actuator_to_percent(actuators.yaw, false),
            )
        };

        if !service.is_enabled() || !gate.is_armed() {
            delay_rest(started);
            continue;
        }

        if let Err(err) = gate.apply_duty_percents(duties) {
            if gate.is_armed() {
                log::error!("stabilizer output failed: {err}");
                fail_closed(&gate, &service, "attitude output failed");
            } else {
                service.disable();
            }
            controller.reset();
            delay_rest(started);
            continue;
        }

        service.publish(duties, sample.attitude);
        tick = tick.wrapping_add(1);
        if tick % PRINT_EVERY == 0 {
            print_att(&target, sample.attitude, duties);
        }
        delay_rest(started);
    }
}

/// Apply a new gain set once. If another `pid` write landed mid-copy, leave the
/// generation stale so the next tick picks up the latest values.
fn sync_gains(service: &AttitudeService, controller: &mut AttitudeController, applied: u32) -> u32 {
    let generation = service.gains_generation();
    if generation == applied {
        return applied;
    }
    let Some(gains) = service.copy_gains() else {
        return applied;
    };
    if service.gains_generation() != generation {
        return applied;
    }
    controller.set_cascade(gains);
    generation
}

fn gain_in_range(value: f32) -> bool {
    value.is_finite() && (0.0..=super::attitude::PID_GAIN_MAX).contains(&value)
}

fn sample_ok(sample: &crate::sensors::ImuSnapshot, now: Instant) -> bool {
    let [gx, gy, gz] = sample.gyro_dps;
    let AttitudeDeg { roll, pitch, yaw } = sample.attitude;
    roll.is_finite()
        && pitch.is_finite()
        && yaw.is_finite()
        && gx.is_finite()
        && gy.is_finite()
        && gz.is_finite()
        && now.saturating_duration_since(sample.at) <= STALE
}

fn fail_closed(gate: &MotorGate, service: &AttitudeService, reason: &str) {
    service.disable();
    let _ = gate.disarm(DisarmReason::AttitudeFault);
    println!("OK disarmed — {reason}, attitude mode off");
}

fn print_att(target: &Target, measured: AttitudeDeg, duties: [u32; 4]) {
    println!(
        "att thr {}% sp={:+.1}/{:+.1}/{:+.1} meas={:+.1}/{:+.1}/{:+.1} | M1={} M2={} M3={} M4={}",
        target.thrust_pct,
        target.roll_deg,
        target.pitch_deg,
        target.yaw_deg,
        measured.roll,
        measured.pitch,
        measured.yaw,
        duties[0],
        duties[1],
        duties[2],
        duties[3],
    );
}

fn delay_rest(started: Instant) {
    let elapsed = started.elapsed().as_millis() as u32;
    if elapsed < PERIOD_MS {
        FreeRtos::delay_ms(PERIOD_MS - elapsed);
    }
}
