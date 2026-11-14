//! WASD + EC flight stick — active only after `joy on` when armed.

use std::io::Write;
use std::sync::Arc;

use crate::flight::AttitudeService;
use crate::safety::{DisarmReason, MotorGate};

/// Stick / throttle change per keypress (%).
const AXIS_STEP: i32 = 5;

/// Max pitch / roll trim before mixing (±%).
const AXIS_LIMIT: i32 = 100;

const HELP: &str = "\
Joystick mode (command mode suspended):
  E / C       Throttle up / down
  W / S       Pitch forward / back
  A / D       Roll left / right
  Space       Reset stick axes, motors off (stay armed)
  Q           Quit joystick and disarm
  ?           Show this help";

pub struct FlightStick {
    throttle: u32,
    pitch: i32,
    roll: i32,
}

pub enum KeyResult {
    Continue,
    Quit,
}

impl FlightStick {
    pub fn new() -> Self {
        Self {
            throttle: 0,
            pitch: 0,
            roll: 0,
        }
    }

    pub fn print_enter_banner() {
        reply("OK joystick mode — E/C throttle, WASD tilt, Space stop, Q disarm");
        reply(HELP);
        Self::print_status(0, 0, 0, [0; 4]);
    }

    pub fn print_quit_banner() {
        reply("OK command mode — type 'help'");
    }

    pub fn handle_key(
        &mut self,
        byte: u8,
        gate: &Arc<MotorGate>,
        attitude: &AttitudeService,
    ) -> KeyResult {
        let changed = match byte {
            b'e' | b'E' => self.bump_throttle(AXIS_STEP as u32, false),
            b'c' | b'C' => self.bump_throttle(AXIS_STEP as u32, true),
            b'w' | b'W' => Self::bump_value(&mut self.pitch, AXIS_STEP),
            b's' | b'S' => Self::bump_value(&mut self.pitch, -AXIS_STEP),
            b'a' | b'A' => Self::bump_value(&mut self.roll, -AXIS_STEP),
            b'd' | b'D' => Self::bump_value(&mut self.roll, AXIS_STEP),
            b' ' => {
                self.reset();
                let _ = gate.stop_output();
                Self::print_status(self.throttle, self.pitch, self.roll, self.mix());
                true
            }
            b'q' | b'Q' => {
                self.reset();
                attitude.disable();
                let _ = gate.disarm(DisarmReason::JoyQuit);
                return KeyResult::Quit;
            }
            b'?' => {
                reply(HELP);
                false
            }
            b'\n' | b'\r' => false,
            _ => false,
        };

        if changed {
            match gate.apply_duty_percents(self.mix()) {
                Ok(()) => Self::print_status(self.throttle, self.pitch, self.roll, self.mix()),
                Err(err) => reply(&format!("ERR {err}")),
            }
        }

        KeyResult::Continue
    }

    fn bump_value(axis: &mut i32, delta: i32) -> bool {
        let next = (*axis + delta).clamp(-AXIS_LIMIT, AXIS_LIMIT);
        if next == *axis {
            return false;
        }
        *axis = next;
        true
    }

    fn bump_throttle(&mut self, step: u32, down: bool) -> bool {
        let next = if down {
            self.throttle.saturating_sub(step)
        } else {
            (self.throttle + step).min(100)
        };
        if next == self.throttle {
            return false;
        }
        self.throttle = next;
        true
    }

    fn reset(&mut self) {
        self.throttle = 0;
        self.pitch = 0;
        self.roll = 0;
    }

    /// X-quad mix — pitch forward raises back (M2/M3), roll right raises right (M1/M2).
    fn mix(&self) -> [u32; 4] {
        let base = self.throttle as i32;
        let duties = [
            base - self.pitch + self.roll,
            base + self.pitch + self.roll,
            base + self.pitch - self.roll,
            base - self.pitch - self.roll,
        ];
        duties.map(clamp_duty)
    }

    fn print_status(throttle: u32, pitch: i32, roll: i32, duties: [u32; 4]) {
        reply(&format!(
            "thr {throttle}% pitch {pitch:+} roll {roll:+} | \
             M1={} M2={} M3={} M4={}",
            duties[0], duties[1], duties[2], duties[3]
        ));
    }
}

fn clamp_duty(value: i32) -> u32 {
    value.clamp(0, 100) as u32
}

fn reply(msg: &str) {
    println!("{msg}");
    let _ = Write::flush(&mut std::io::stdout());
}
