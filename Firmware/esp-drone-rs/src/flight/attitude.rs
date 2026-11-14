//! Angle → rate cascade.
//!
//! Gains are the ESP32 (non-S2) defaults from Crazyflie `pid.h` (GPL-3).
//! Angle error becomes a rate command; rate error against the gyro becomes an
//! actuator on the Crazyflie uint16 thrust scale. The stabilizer converts that
//! to percent. Axes stay in the MPU-6050 chip frame, same as Mahony — there is
//! no airframe rotation yet. Gyro pitch is not negated, because the euler
//! angles are in that same frame (`controller_pid.c` negates pitch only after
//! its own sensor alignment).

use crate::estimation::AttitudeDeg;

use super::pid::{Pid, PidGains};

/// Largest Kp, Ki, or Kd the serial `pid` command will store.
pub const PID_GAIN_MAX: f32 = 5000.0;

/// Crazyflie full-scale thrust. One count is `100 / 65535` percent.
const THRUST_FULL_SCALE: f32 = 65535.0;

/// ESP-Drone / Crazyflie defaults for the ESP32 target (`pid.h`, non-S2).
const ROLL_KP: f32 = 5.9;
const ROLL_KI: f32 = 2.9;
const ROLL_KD: f32 = 0.0;
const ROLL_I_LIMIT: f32 = 20.0;

const PITCH_KP: f32 = 5.9;
const PITCH_KI: f32 = 2.9;
const PITCH_KD: f32 = 0.0;
const PITCH_I_LIMIT: f32 = 20.0;

const YAW_KP: f32 = 6.0;
const YAW_KI: f32 = 1.0;
const YAW_KD: f32 = 0.35;
const YAW_I_LIMIT: f32 = 360.0;

const ROLL_RATE_KP: f32 = 250.0;
const ROLL_RATE_KI: f32 = 500.0;
const ROLL_RATE_KD: f32 = 2.5;
const ROLL_RATE_I_LIMIT: f32 = 33.3;

const PITCH_RATE_KP: f32 = 250.0;
const PITCH_RATE_KI: f32 = 500.0;
const PITCH_RATE_KD: f32 = 2.5;
const PITCH_RATE_I_LIMIT: f32 = 33.3;

const YAW_RATE_KP: f32 = 120.0;
const YAW_RATE_KI: f32 = 16.7;
const YAW_RATE_KD: f32 = 0.0;
const YAW_RATE_I_LIMIT: f32 = 166.7;

/// One cascade loop the bench can retune: angle roll/pitch/yaw, or the matching rate loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PidAxis {
    Roll,
    Pitch,
    Yaw,
    RollRate,
    PitchRate,
    YawRate,
}

impl PidAxis {
    pub const ALL: [PidAxis; 6] = [
        PidAxis::Roll,
        PidAxis::Pitch,
        PidAxis::Yaw,
        PidAxis::RollRate,
        PidAxis::PitchRate,
        PidAxis::YawRate,
    ];

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "roll" => Some(Self::Roll),
            "pitch" => Some(Self::Pitch),
            "yaw" => Some(Self::Yaw),
            "roll-rate" => Some(Self::RollRate),
            "pitch-rate" => Some(Self::PitchRate),
            "yaw-rate" => Some(Self::YawRate),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Roll => "roll",
            Self::Pitch => "pitch",
            Self::Yaw => "yaw",
            Self::RollRate => "roll-rate",
            Self::PitchRate => "pitch-rate",
            Self::YawRate => "yaw-rate",
        }
    }
}

/// Angle and rate gains for roll, pitch, and yaw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CascadeGains {
    pub roll: PidGains,
    pub pitch: PidGains,
    pub yaw: PidGains,
    pub roll_rate: PidGains,
    pub pitch_rate: PidGains,
    pub yaw_rate: PidGains,
}

impl CascadeGains {
    /// ESP32 (non-S2) Crazyflie defaults.
    pub fn defaults() -> Self {
        Self {
            roll: PidGains {
                kp: ROLL_KP,
                ki: ROLL_KI,
                kd: ROLL_KD,
            },
            pitch: PidGains {
                kp: PITCH_KP,
                ki: PITCH_KI,
                kd: PITCH_KD,
            },
            yaw: PidGains {
                kp: YAW_KP,
                ki: YAW_KI,
                kd: YAW_KD,
            },
            roll_rate: PidGains {
                kp: ROLL_RATE_KP,
                ki: ROLL_RATE_KI,
                kd: ROLL_RATE_KD,
            },
            pitch_rate: PidGains {
                kp: PITCH_RATE_KP,
                ki: PITCH_RATE_KI,
                kd: PITCH_RATE_KD,
            },
            yaw_rate: PidGains {
                kp: YAW_RATE_KP,
                ki: YAW_RATE_KI,
                kd: YAW_RATE_KD,
            },
        }
    }

    pub fn get(self, axis: PidAxis) -> PidGains {
        match axis {
            PidAxis::Roll => self.roll,
            PidAxis::Pitch => self.pitch,
            PidAxis::Yaw => self.yaw,
            PidAxis::RollRate => self.roll_rate,
            PidAxis::PitchRate => self.pitch_rate,
            PidAxis::YawRate => self.yaw_rate,
        }
    }

    pub fn set(&mut self, axis: PidAxis, gains: PidGains) {
        match axis {
            PidAxis::Roll => self.roll = gains,
            PidAxis::Pitch => self.pitch = gains,
            PidAxis::Yaw => self.yaw = gains,
            PidAxis::RollRate => self.roll_rate = gains,
            PidAxis::PitchRate => self.pitch_rate = gains,
            PidAxis::YawRate => self.yaw_rate = gains,
        }
    }
}

/// Roll / pitch / yaw actuators on the Crazyflie thrust scale (±32767).
#[derive(Debug, Clone, Copy)]
pub struct Actuators {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
}

pub struct AttitudeController {
    roll: Pid,
    pitch: Pid,
    yaw: Pid,
    roll_rate: Pid,
    pitch_rate: Pid,
    yaw_rate: Pid,
}

impl AttitudeController {
    pub fn new() -> Self {
        let gains = CascadeGains::defaults();
        Self {
            roll: Pid::new(gains.roll.kp, gains.roll.ki, gains.roll.kd, ROLL_I_LIMIT),
            pitch: Pid::new(
                gains.pitch.kp,
                gains.pitch.ki,
                gains.pitch.kd,
                PITCH_I_LIMIT,
            ),
            yaw: Pid::new(gains.yaw.kp, gains.yaw.ki, gains.yaw.kd, YAW_I_LIMIT),
            roll_rate: Pid::new(
                gains.roll_rate.kp,
                gains.roll_rate.ki,
                gains.roll_rate.kd,
                ROLL_RATE_I_LIMIT,
            ),
            pitch_rate: Pid::new(
                gains.pitch_rate.kp,
                gains.pitch_rate.ki,
                gains.pitch_rate.kd,
                PITCH_RATE_I_LIMIT,
            ),
            yaw_rate: Pid::new(
                gains.yaw_rate.kp,
                gains.yaw_rate.ki,
                gains.yaw_rate.kd,
                YAW_RATE_I_LIMIT,
            ),
        }
    }

    /// Copy Kp/Ki/Kd into each loop. A loop whose gains changed has its integrator cleared.
    pub fn set_cascade(&mut self, gains: CascadeGains) {
        apply_loop(&mut self.roll, gains.roll);
        apply_loop(&mut self.pitch, gains.pitch);
        apply_loop(&mut self.yaw, gains.yaw);
        apply_loop(&mut self.roll_rate, gains.roll_rate);
        apply_loop(&mut self.pitch_rate, gains.pitch_rate);
        apply_loop(&mut self.yaw_rate, gains.yaw_rate);
    }

    pub fn reset(&mut self) {
        self.roll.reset();
        self.pitch.reset();
        self.yaw.reset();
        self.roll_rate.reset();
        self.pitch_rate.reset();
        self.yaw_rate.reset();
    }

    /// One cascade step. Angles and gyro are degrees and °/s in the chip frame.
    pub fn update(
        &mut self,
        desired: AttitudeDeg,
        measured: AttitudeDeg,
        gyro_dps: [f32; 3],
        dt: f32,
    ) -> Actuators {
        let roll_rate = self.roll.update(desired.roll - measured.roll, dt);
        let pitch_rate = self.pitch.update(desired.pitch - measured.pitch, dt);
        let yaw_rate = self.yaw.update(wrap_deg(desired.yaw - measured.yaw), dt);

        Actuators {
            roll: saturate_i16(self.roll_rate.update(roll_rate - gyro_dps[0], dt)),
            pitch: saturate_i16(self.pitch_rate.update(pitch_rate - gyro_dps[1], dt)),
            yaw: saturate_i16(self.yaw_rate.update(yaw_rate - gyro_dps[2], dt)),
        }
    }
}

impl Default for AttitudeController {
    fn default() -> Self {
        Self::new()
    }
}

/// Map a Crazyflie actuator count onto percent of full thrust.
///
/// `power_distribution_stock.c` divides roll and pitch by two before mixing
/// and leaves yaw at full scale.
pub fn actuator_to_percent(raw: f32, halve: bool) -> f32 {
    let scaled = if halve { raw * 0.5 } else { raw };
    scaled * (100.0 / THRUST_FULL_SCALE)
}

/// Wrap a degree error into (−180, 180].
pub fn wrap_deg(angle: f32) -> f32 {
    if !angle.is_finite() {
        return 0.0;
    }
    let mut wrapped = angle % 360.0;
    if wrapped > 180.0 {
        wrapped -= 360.0;
    } else if wrapped <= -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

fn saturate_i16(value: f32) -> f32 {
    value.clamp(-32767.0, 32767.0)
}

fn apply_loop(pid: &mut Pid, gains: PidGains) {
    if pid.gains() == gains {
        return;
    }
    pid.set_gains(gains);
    pid.reset();
}

/// Four-decimal gain, trimmed, so `16.7` stays `16.7` on the serial line.
pub fn format_gain(value: f32) -> String {
    let scaled = (f64::from(value) * 10_000.0).round() as i32;
    let whole = scaled / 10_000;
    let frac = (scaled % 10_000).unsigned_abs();
    if frac == 0 {
        return format!("{whole}");
    }
    let mut frac_text = format!("{frac:04}");
    while frac_text.ends_with('0') {
        frac_text.pop();
    }
    format!("{whole}.{frac_text}")
}

pub fn round_gain(value: f32) -> f32 {
    let scaled = (f64::from(value) * 10_000.0).round();
    (scaled / 10_000.0) as f32
}

pub fn format_pid_line(axis: PidAxis, gains: PidGains) -> String {
    format!(
        "OK pid {} {} {} {}",
        axis.name(),
        format_gain(gains.kp),
        format_gain(gains.ki),
        format_gain(gains.kd),
    )
}

pub fn format_pid_report(gains: &CascadeGains) -> String {
    PidAxis::ALL
        .into_iter()
        .map(|axis| format_pid_line(axis, gains.get(axis)))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::{
        actuator_to_percent, format_gain, format_pid_report, wrap_deg, AttitudeController,
        CascadeGains,
    };
    use crate::estimation::AttitudeDeg;
    use crate::flight::mixer::mix_x;

    #[test]
    fn wrap_shortest_yaw() {
        assert!((wrap_deg(190.0) - -170.0).abs() < 1.0e-3);
        assert!((wrap_deg(-190.0) - 170.0).abs() < 1.0e-3);
        assert!((wrap_deg(180.0) - 180.0).abs() < 1.0e-3);
        assert!((wrap_deg(-180.0) - 180.0).abs() < 1.0e-3);
    }

    #[test]
    fn right_side_down_spins_up_the_right_motors() {
        let mut controller = AttitudeController::new();
        let desired = AttitudeDeg {
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
        };
        let measured = AttitudeDeg {
            roll: 10.0,
            pitch: 0.0,
            yaw: 0.0,
        };
        let actuators = controller.update(desired, measured, [0.0; 3], 0.004);
        let duties = mix_x(
            30.0,
            actuator_to_percent(actuators.roll, true),
            actuator_to_percent(actuators.pitch, true),
            actuator_to_percent(actuators.yaw, false),
        );
        assert!(
            duties[0] > 30 && duties[1] > 30,
            "right motors should rise to pick up a right-down tilt, got {duties:?}"
        );
        assert!(
            duties[2] < 30 && duties[3] < 30,
            "left motors should fall, got {duties:?}"
        );
    }

    #[test]
    fn higher_roll_kp_commands_a_stronger_correction() {
        let desired = AttitudeDeg {
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
        };
        let measured = AttitudeDeg {
            roll: 10.0,
            pitch: 0.0,
            yaw: 0.0,
        };
        let mut stock = AttitudeController::new();
        let stock_roll = stock.update(desired, measured, [0.0; 3], 0.004).roll;

        let mut tuned = AttitudeController::new();
        let mut gains = CascadeGains::defaults();
        gains.roll.kp *= 2.0;
        tuned.set_cascade(gains);
        let tuned_roll = tuned.update(desired, measured, [0.0; 3], 0.004).roll;
        assert!(
            tuned_roll < stock_roll,
            "more angle P should correct harder, stock {stock_roll} tuned {tuned_roll}"
        );
    }

    #[test]
    fn default_report_uses_trimmed_gains() {
        let report = format_pid_report(&CascadeGains::defaults());
        assert!(report.starts_with("OK pid roll 5.9 2.9 0\n"));
        assert!(report.contains("\nOK pid yaw 6 1 0.35\n"));
        assert!(report.contains("\nOK pid yaw-rate 120 16.7 0"));
        assert_eq!(format_gain(16.7), "16.7");
        assert_eq!(format_gain(250.0), "250");
    }
}
