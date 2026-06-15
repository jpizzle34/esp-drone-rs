//! Bench serial command parser and handlers.

use std::sync::{Arc, Mutex};

use esp_idf_hal::delay::FreeRtos;

use crate::drivers::motors::{Motor, Motors};
use crate::sensors::{ImuService, DEFAULT_STREAM_HZ};

/// Longest allowed `motor pulse` duration (ms).
const MAX_PULSE_MS: u32 = 30_000;

const HELP: &str = "\
Commands:
  help                       Show this help
  motor <0-100>              Set M1 duty percent
  motor <1-4> <0-100>        Set motor duty percent
  motor all <0-100>          Set all motors to same duty percent
  motor pulse <0-100> <ms>   Run all motors at duty for ms, then stop
  stop                       All motors off
  joy on                     Flight stick — E/C throttle, WASD tilt (Q quit)
  joy off                    Stop motors (use Q if already in joystick mode)
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
    motors: &Arc<Mutex<Motors>>,
    imu: Option<&ImuService>,
) -> CommandAction {
    let line = line.trim();
    if line.is_empty() {
        return CommandAction::None;
    }

    let parts: Vec<&str> = line.split_whitespace().collect();
    let (result, action) = match parts.as_slice() {
        ["help"] | ["?"] => (Ok(HELP.to_string()), CommandAction::None),
        ["stop"] => (stop_motors(motors), CommandAction::None),
        ["motor", pct] => (set_motor(motors, Motor::M1, pct), CommandAction::None),
        ["motor", "all", pct] => (set_all_motors(motors, pct), CommandAction::None),
        ["motor", "pulse", pct, ms] => (pulse_all_motors(motors, pct, ms), CommandAction::None),
        ["motor", idx, pct] if *idx != "all" && *idx != "pulse" => {
            match parse_motor_index(idx) {
                Ok(motor) => (set_motor(motors, motor, pct), CommandAction::None),
                Err(msg) => (Err(msg), CommandAction::None),
            }
        }
        ["joy", "on"] => (
            Ok("entering joystick mode".to_string()),
            CommandAction::EnterJoystick,
        ),
        ["joy", "off"] => (stop_motors(motors), CommandAction::None),
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

fn stop_motors(motors: &Arc<Mutex<Motors>>) -> Result<String, String> {
    let mut motors = motors.lock().map_err(|_| "motor mutex poisoned".to_string())?;
    motors
        .all_off()
        .map_err(|err| format!("stop failed: {err:#}"))?;
    Ok("OK all motors stopped".to_string())
}

fn set_motor(motors: &Arc<Mutex<Motors>>, motor: Motor, pct: &str) -> Result<String, String> {
    let percent = parse_percent(pct)?;
    let mut motors = motors.lock().map_err(|_| "motor mutex poisoned".to_string())?;
    motors
        .set_duty_percent(motor, percent)
        .map_err(|err| format!("motor set failed: {err:#}"))?;
    Ok(format!(
        "OK {} duty {}%",
        motor.name(),
        percent
    ))
}

fn set_all_motors(motors: &Arc<Mutex<Motors>>, pct: &str) -> Result<String, String> {
    let percent = parse_percent(pct)?;
    let mut motors = motors.lock().map_err(|_| "motor mutex poisoned".to_string())?;
    motors
        .set_all_duty_percent(percent)
        .map_err(|err| format!("motor set failed: {err:#}"))?;
    Ok(format!("OK all motors duty {percent}%"))
}

fn pulse_all_motors(
    motors: &Arc<Mutex<Motors>>,
    pct: &str,
    ms: &str,
) -> Result<String, String> {
    let percent = parse_percent(pct)?;
    let duration_ms = parse_duration_ms(ms)?;
    let mut motors = motors.lock().map_err(|_| "motor mutex poisoned".to_string())?;
    motors
        .set_all_duty_percent(percent)
        .map_err(|err| format!("motor pulse failed: {err:#}"))?;
    FreeRtos::delay_ms(duration_ms);
    motors
        .all_off()
        .map_err(|err| format!("motor stop failed: {err:#}"))?;
    Ok(format!(
        "OK all motors {percent}% for {duration_ms} ms"
    ))
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
