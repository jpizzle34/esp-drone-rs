//! Bench serial command parser and handlers.

use std::sync::Arc;

use esp_idf_hal::delay::FreeRtos;

use crate::board::{ActiveBoard, BoardProfile};
use crate::drivers::motors::Motor;
use crate::flight::AttitudeService;
use crate::safety::{DisarmReason, MotorGate};
use crate::sensors::{ImuService, DEFAULT_STREAM_HZ};

/// Longest allowed `motor pulse` duration (ms).
const MAX_PULSE_MS: u32 = 30_000;

const HELP: &str = "\
Commands:
  help                       Show this help
  arm                        Arm motors (M1→M4 chirp, then output enabled)
  disarm                     Disarm — all motors off immediately
  motor <0-100>              Set M1 duty percent (armed only)
  motor <1-4> <0-100>        Set motor duty percent (armed only)
  motor all <0-100>          Set all motors to same duty percent (armed only)
  motor pulse <0-100> <ms>   Run all motors at duty for ms, then stop (armed only)
  stop                       Same as disarm
  joy on                     Flight stick — E/C throttle, WASD tilt (armed, Q quit)
  joy off                    Disarm
  imu                        IMU attitude and stream status
  imu on [hz]                Stream IMU (default 10 Hz, max 100 Hz)
  imu off                    Stop IMU stream
  imu cal                    Re-zero gyro bias (keep the board still)
  att                        Attitude-mode setpoints, measured attitude, duties
  att on                     Angle mode (armed). Holds level and current yaw, thrust 0
  att off                    Leave angle mode — motors stop, stay armed
  att level                  Roll/pitch 0 and recapture yaw
  att thrust <0-100>         Collective thrust percent
  att roll <deg>             Roll setpoint, -30..+30 (right side down is +)
  att pitch <deg>            Pitch setpoint, -30..+30 (nose up is +)
  att yaw <deg>              Yaw setpoint, degrees
  pid                        Angle and rate PID gains (kp ki kd)
  pid defaults               Restore ESP32 Crazyflie defaults
  pid <loop> <kp> <ki> <kd>  Set roll, pitch, yaw, roll-rate, pitch-rate, or yaw-rate";

/// Side effect requested after a command line is handled.
#[derive(PartialEq)]
pub enum CommandAction {
    None,
    EnterJoystick,
}

pub fn print_help() {
    reply(HELP);
}

pub fn handle_line(
    line: &str,
    gate: &Arc<MotorGate>,
    imu: Option<&ImuService>,
    attitude: &AttitudeService,
) -> CommandAction {
    let line = line.trim();
    if line.is_empty() {
        return CommandAction::None;
    }

    let parts: Vec<&str> = line.split_whitespace().collect();
    let (result, action) = match parts.as_slice() {
        ["help"] | ["?"] => (Ok(HELP.to_string()), CommandAction::None),
        ["arm"] => (cmd_arm(gate), CommandAction::None),
        ["disarm"] => (
            cmd_disarm(gate, attitude, DisarmReason::Command),
            CommandAction::None,
        ),
        ["stop"] => (
            cmd_disarm(gate, attitude, DisarmReason::Stop),
            CommandAction::None,
        ),
        ["motor", pct] => (
            set_motor(gate, attitude, Motor::M1, pct),
            CommandAction::None,
        ),
        ["motor", "all", pct] => (set_all_motors(gate, attitude, pct), CommandAction::None),
        ["motor", "pulse", pct, ms] => (
            pulse_all_motors(gate, attitude, pct, ms),
            CommandAction::None,
        ),
        ["motor", idx, pct] if *idx != "all" && *idx != "pulse" => match parse_motor_index(idx) {
            Ok(motor) => (set_motor(gate, attitude, motor, pct), CommandAction::None),
            Err(msg) => (Err(msg), CommandAction::None),
        },
        ["joy", "on"] => {
            if attitude.is_enabled() {
                (
                    Err("attitude mode owns the motors — type 'att off' first".to_string()),
                    CommandAction::None,
                )
            } else if gate.is_armed() {
                (
                    Ok("entering joystick mode".to_string()),
                    CommandAction::EnterJoystick,
                )
            } else {
                (
                    Err("disarmed — type 'arm' first".to_string()),
                    CommandAction::None,
                )
            }
        }
        ["joy", "off"] => (
            cmd_disarm(gate, attitude, DisarmReason::Command),
            CommandAction::None,
        ),
        ["imu"] => (Ok(imu_status(imu)), CommandAction::None),
        ["imu", "on"] => (imu_set_stream(imu, true, None), CommandAction::None),
        ["imu", "on", hz] => match parse_hz(hz) {
            Ok(hz) => (imu_set_stream(imu, true, Some(hz)), CommandAction::None),
            Err(msg) => (Err(msg), CommandAction::None),
        },
        ["imu", "off"] => (imu_set_stream(imu, false, None), CommandAction::None),
        ["imu", "cal"] => (imu_cal(imu), CommandAction::None),
        ["att"] => (Ok(attitude.status_line()), CommandAction::None),
        ["att", "on"] => (att_on(gate, imu, attitude), CommandAction::None),
        ["att", "off"] => (att_off(gate, attitude), CommandAction::None),
        ["att", "level"] => (att_level(imu, attitude), CommandAction::None),
        ["att", "thrust", pct] => (att_thrust(attitude, pct), CommandAction::None),
        ["att", "roll", deg] => (att_axis(attitude, deg, Axis::Roll), CommandAction::None),
        ["att", "pitch", deg] => (att_axis(attitude, deg, Axis::Pitch), CommandAction::None),
        ["att", "yaw", deg] => (att_axis(attitude, deg, Axis::Yaw), CommandAction::None),
        ["pid"] => (attitude.pid_report(), CommandAction::None),
        ["pid", "defaults"] => (attitude.reset_pid(), CommandAction::None),
        ["pid", axis, kp, ki, kd] => (pid_set(attitude, axis, kp, ki, kd), CommandAction::None),
        ["pid", ..] => (
            Err("usage: pid | pid defaults | pid <loop> <kp> <ki> <kd>".to_string()),
            CommandAction::None,
        ),
        _ => (
            Err("unknown command — type help".to_string()),
            CommandAction::None,
        ),
    };

    match result {
        Ok(msg) => {
            if action != CommandAction::EnterJoystick {
                reply(&msg);
            }
        }
        Err(msg) => reply(&format!("ERR {msg}")),
    }

    action
}

fn cmd_arm(gate: &Arc<MotorGate>) -> Result<String, String> {
    gate.arm(ActiveBoard::motor_table())?;
    Ok("OK armed — motor output enabled".to_string())
}

fn cmd_disarm(
    gate: &Arc<MotorGate>,
    attitude: &AttitudeService,
    reason: DisarmReason,
) -> Result<String, String> {
    attitude.disable();
    gate.disarm(reason)?;
    Ok("OK disarmed — all motors off".to_string())
}

fn reject_if_attitude(attitude: &AttitudeService) -> Result<(), String> {
    if attitude.is_enabled() {
        Err("attitude mode owns the motors — type 'att off' first".to_string())
    } else {
        Ok(())
    }
}

fn set_motor(
    gate: &Arc<MotorGate>,
    attitude: &AttitudeService,
    motor: Motor,
    pct: &str,
) -> Result<String, String> {
    reject_if_attitude(attitude)?;
    let percent = parse_percent(pct)?;
    gate.set_duty_percent(motor, percent)?;
    Ok(format!("OK {} duty {}%", motor.name(), percent))
}

fn set_all_motors(
    gate: &Arc<MotorGate>,
    attitude: &AttitudeService,
    pct: &str,
) -> Result<String, String> {
    reject_if_attitude(attitude)?;
    let percent = parse_percent(pct)?;
    gate.set_all_duty_percent(percent)?;
    Ok(format!("OK all motors duty {percent}%"))
}

fn pulse_all_motors(
    gate: &Arc<MotorGate>,
    attitude: &AttitudeService,
    pct: &str,
    ms: &str,
) -> Result<String, String> {
    reject_if_attitude(attitude)?;
    let percent = parse_percent(pct)?;
    let duration_ms = parse_duration_ms(ms)?;
    gate.set_all_duty_percent(percent)?;
    FreeRtos::delay_ms(duration_ms);
    gate.stop_output()?;
    Ok(format!("OK all motors {percent}% for {duration_ms} ms"))
}

enum Axis {
    Roll,
    Pitch,
    Yaw,
}

fn att_on(
    gate: &Arc<MotorGate>,
    imu: Option<&ImuService>,
    attitude: &AttitudeService,
) -> Result<String, String> {
    if !gate.is_armed() {
        return Err("disarmed — type 'arm' first".to_string());
    }
    if attitude.is_enabled() {
        return Ok("OK attitude already on".to_string());
    }
    let yaw = current_yaw(imu)?;
    attitude.enable(yaw)?;
    Ok(format!("OK attitude on — holding yaw {yaw:+.1}°, thrust 0"))
}

fn att_off(gate: &Arc<MotorGate>, attitude: &AttitudeService) -> Result<String, String> {
    attitude.disable();
    if gate.is_armed() {
        gate.stop_output()?;
    }
    Ok("OK attitude off — motors stopped".to_string())
}

fn att_level(imu: Option<&ImuService>, attitude: &AttitudeService) -> Result<String, String> {
    let yaw = current_yaw(imu)?;
    let yaw = attitude.level(yaw)?;
    Ok(format!("OK attitude level — roll 0 pitch 0 yaw {yaw:+.1}°"))
}

fn att_thrust(attitude: &AttitudeService, pct: &str) -> Result<String, String> {
    let percent = parse_percent(pct)?;
    attitude.set_thrust(percent)?;
    Ok(format!("OK attitude thrust {percent}%"))
}

fn pid_set(
    attitude: &AttitudeService,
    axis: &str,
    kp: &str,
    ki: &str,
    kd: &str,
) -> Result<String, String> {
    let kp = parse_gain(kp)?;
    let ki = parse_gain(ki)?;
    let kd = parse_gain(kd)?;
    attitude.set_pid_loop(axis, kp, ki, kd)
}

fn att_axis(attitude: &AttitudeService, raw: &str, axis: Axis) -> Result<String, String> {
    let deg = parse_angle(raw)?;
    let stored = match axis {
        Axis::Roll => attitude.set_roll(deg)?,
        Axis::Pitch => attitude.set_pitch(deg)?,
        Axis::Yaw => attitude.set_yaw(deg)?,
    };
    let name = match axis {
        Axis::Roll => "roll",
        Axis::Pitch => "pitch",
        Axis::Yaw => "yaw",
    };
    Ok(format!("OK attitude {name} {stored:+.1}°"))
}

fn current_yaw(imu: Option<&ImuService>) -> Result<f32, String> {
    let Some(imu) = imu else {
        return Err("IMU not available".to_string());
    };
    let Some(sample) = imu.latest() else {
        return Err("IMU waiting for first sample".to_string());
    };
    if !sample.attitude.yaw.is_finite() {
        return Err("IMU attitude is not finite".to_string());
    }
    Ok(sample.attitude.yaw)
}

fn imu_status(imu: Option<&ImuService>) -> String {
    let Some(imu) = imu else {
        return "IMU not available".to_string();
    };
    let stream = if imu.stream_enabled() {
        format!("stream on ({} Hz)", imu.stream_hz())
    } else {
        "stream off — use: imu on".to_string()
    };
    match imu.latest() {
        Some(sample) => format!(
            "IMU {stream} — roll={:+.1}° pitch={:+.1}° yaw={:+.1}°",
            sample.attitude.roll, sample.attitude.pitch, sample.attitude.yaw
        ),
        None => format!("IMU {stream} — waiting for first sample"),
    }
}

fn imu_cal(imu: Option<&ImuService>) -> Result<String, String> {
    let Some(imu) = imu else {
        return Err("IMU not available".to_string());
    };
    imu.request_cal();
    Ok("OK calibrating gyro — keep the board still".to_string())
}

fn imu_set_stream(imu: Option<&ImuService>, on: bool, hz: Option<u32>) -> Result<String, String> {
    let Some(imu) = imu else {
        return Err("IMU not available".to_string());
    };

    if on {
        let hz = hz.unwrap_or(DEFAULT_STREAM_HZ);
        imu.set_stream_hz(hz);
        imu.set_stream(true);
        Ok(format!("OK IMU stream on ({hz} Hz)"))
    } else {
        imu.set_stream(false);
        Ok("OK IMU stream off".to_string())
    }
}

fn parse_motor_index(raw: &str) -> Result<Motor, String> {
    let idx: usize = raw
        .parse()
        .map_err(|_| format!("invalid motor index '{raw}' (use 1-4)"))?;
    if !(1..=4).contains(&idx) {
        return Err(format!("motor index must be 1-4, got {idx}"));
    }
    Motor::try_from(idx - 1).map_err(|_| "invalid motor index".to_string())
}

fn parse_percent(raw: &str) -> Result<u32, String> {
    let value: u32 = raw
        .parse()
        .map_err(|_| format!("invalid duty '{raw}' (use 0-100)"))?;
    if value > 100 {
        return Err(format!("duty must be 0-100, got {value}"));
    }
    Ok(value)
}

fn parse_duration_ms(raw: &str) -> Result<u32, String> {
    let ms: u32 = raw
        .parse()
        .map_err(|_| format!("invalid duration '{raw}' (use 1-{MAX_PULSE_MS} ms)"))?;
    if ms == 0 || ms > MAX_PULSE_MS {
        return Err(format!("duration must be 1-{MAX_PULSE_MS} ms, got {ms}"));
    }
    Ok(ms)
}

fn parse_gain(raw: &str) -> Result<f32, String> {
    let value: f32 = raw.parse().map_err(|_| format!("invalid gain '{raw}'"))?;
    if !value.is_finite() || !(0.0..=crate::flight::PID_GAIN_MAX).contains(&value) {
        return Err(format!(
            "gain must be 0..={}, got '{raw}'",
            crate::flight::PID_GAIN_MAX
        ));
    }
    Ok(value)
}

fn parse_angle(raw: &str) -> Result<f32, String> {
    let value: f32 = raw.parse().map_err(|_| format!("invalid angle '{raw}'"))?;
    if !value.is_finite() {
        return Err(format!("invalid angle '{raw}'"));
    }
    Ok(value)
}

fn parse_hz(raw: &str) -> Result<u32, String> {
    let hz: u32 = raw.parse().map_err(|_| format!("invalid hz '{raw}'"))?;
    if !(1..=100).contains(&hz) {
        return Err(format!("hz must be 1-100, got {hz}"));
    }
    Ok(hz)
}

fn reply(msg: &str) {
    println!("{msg}");
    let _ = std::io::Write::flush(&mut std::io::stdout());
}
