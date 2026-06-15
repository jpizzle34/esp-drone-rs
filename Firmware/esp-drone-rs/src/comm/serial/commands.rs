//! Bench serial command parser and handlers.

use std::sync::Arc;

use esp_idf_hal::delay::FreeRtos;

use crate::board::{ActiveBoard, BoardProfile};
use crate::drivers::motors::Motor;
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
  imu                        IMU stream status
  imu on [hz]                Stream IMU (default 10 Hz, max 100 Hz)
  imu off                    Stop IMU stream";

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
) -> CommandAction {
    let line = line.trim();
    if line.is_empty() {
        return CommandAction::None;
    }

    let parts: Vec<&str> = line.split_whitespace().collect();
    let (result, action) = match parts.as_slice() {
        ["help"] | ["?"] => (Ok(HELP.to_string()), CommandAction::None),
        ["arm"] => (cmd_arm(gate), CommandAction::None),
        ["disarm"] => (cmd_disarm(gate, DisarmReason::Command), CommandAction::None),
        ["stop"] => (cmd_disarm(gate, DisarmReason::Stop), CommandAction::None),
        ["motor", pct] => (set_motor(gate, Motor::M1, pct), CommandAction::None),
        ["motor", "all", pct] => (set_all_motors(gate, pct), CommandAction::None),
        ["motor", "pulse", pct, ms] => (pulse_all_motors(gate, pct, ms), CommandAction::None),
        ["motor", idx, pct] if *idx != "all" && *idx != "pulse" => {
            match parse_motor_index(idx) {
                Ok(motor) => (set_motor(gate, motor, pct), CommandAction::None),
                Err(msg) => (Err(msg), CommandAction::None),
            }
        }
        ["joy", "on"] => {
            if gate.is_armed() {
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
        ["joy", "off"] => (cmd_disarm(gate, DisarmReason::Command), CommandAction::None),
        ["imu"] => (Ok(imu_status(imu)), CommandAction::None),
        ["imu", "on"] => (imu_set_stream(imu, true, None), CommandAction::None),
        ["imu", "on", hz] => match parse_hz(hz) {
            Ok(hz) => (imu_set_stream(imu, true, Some(hz)), CommandAction::None),
            Err(msg) => (Err(msg), CommandAction::None),
        },
        ["imu", "off"] => (imu_set_stream(imu, false, None), CommandAction::None),
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

fn cmd_disarm(gate: &Arc<MotorGate>, reason: DisarmReason) -> Result<String, String> {
    gate.disarm(reason)?;
    Ok("OK disarmed — all motors off".to_string())
}

fn set_motor(gate: &Arc<MotorGate>, motor: Motor, pct: &str) -> Result<String, String> {
    let percent = parse_percent(pct)?;
    gate.set_duty_percent(motor, percent)?;
    Ok(format!("OK {} duty {}%", motor.name(), percent))
}

fn set_all_motors(gate: &Arc<MotorGate>, pct: &str) -> Result<String, String> {
    let percent = parse_percent(pct)?;
    gate.set_all_duty_percent(percent)?;
    Ok(format!("OK all motors duty {percent}%"))
}

fn pulse_all_motors(gate: &Arc<MotorGate>, pct: &str, ms: &str) -> Result<String, String> {
    let percent = parse_percent(pct)?;
    let duration_ms = parse_duration_ms(ms)?;
    gate.set_all_duty_percent(percent)?;
    FreeRtos::delay_ms(duration_ms);
    gate.stop_output()?;
    Ok(format!("OK all motors {percent}% for {duration_ms} ms"))
}

fn imu_status(imu: Option<&ImuService>) -> String {
    let Some(imu) = imu else {
        return "IMU not available".to_string();
    };
    if imu.stream_enabled() {
        format!("IMU stream on ({} Hz)", imu.stream_hz())
    } else {
        "IMU stream off — use: imu on".to_string()
    }
}

fn imu_set_stream(
    imu: Option<&ImuService>,
    on: bool,
    hz: Option<u32>,
) -> Result<String, String> {
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

fn parse_hz(raw: &str) -> Result<u32, String> {
    let hz: u32 = raw
        .parse()
        .map_err(|_| format!("invalid hz '{raw}'"))?;
    if !(1..=100).contains(&hz) {
        return Err(format!("hz must be 1-100, got {hz}"));
    }
    Ok(hz)
}

fn reply(msg: &str) {
    println!("{msg}");
    let _ = std::io::Write::flush(&mut std::io::stdout());
}
