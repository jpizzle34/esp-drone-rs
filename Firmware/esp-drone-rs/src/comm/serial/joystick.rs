//! WASD + EC flight stick — active only after `joy on` (see [`super::serial`] input modes).
//!
//! X-quad layout: M1 front-right, M2 back-right, M3 back-left, M4 front-left.

use std::io::Write;
use std::sync::{Arc, Mutex};

use crate::drivers::motors::{Motor, Motors};

/// Stick / throttle change per keypress (%).
const AXIS_STEP: i32 = 5;

/// Max pitch / roll trim before mixing (±%).
const AXIS_LIMIT: i32 = 100;

const HELP: &str = "\
Joystick mode (command mode suspended):
  E / C       Throttle up / down
  W / S       Pitch forward / back
  A / D       Roll left / right
  Space       Reset — all axes 0, motors off
  Q           Quit — return to command prompt
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
        reply("OK joystick mode — E/C throttle, WASD tilt, Space stop, Q quit");
        reply(HELP);
        Self::print_status(0, 0, 0, [0; 4]);
    }

    pub fn print_quit_banner() {
        reply("OK command mode — type 'help'");
    }

    pub fn handle_key(&mut self, byte: u8, motors: &Arc<Mutex<Motors>>) -> KeyResult {
        let changed = match byte {
            b'e' | b'E' => self.bump_throttle(AXIS_STEP as u32, false),
            b'c' | b'C' => self.bump_throttle(AXIS_STEP as u32, true),
            b'w' | b'W' => Self::bump_value(&mut self.pitch, AXIS_STEP),
            b's' | b'S' => Self::bump_value(&mut self.pitch, -AXIS_STEP),
            b'a' | b'A' => Self::bump_value(&mut self.roll, -AXIS_STEP),
            b'd' | b'D' => Self::bump_value(&mut self.roll, AXIS_STEP),
            b' ' => {
                self.reset();
                self.apply(motors);
                Self::print_status(self.throttle, self.pitch, self.roll, self.mix());
                true
            }
            b'q' | b'Q' => {
                self.reset();
                let _ = motors.lock().map(|mut m| m.all_off());
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
            self.apply(motors);
            Self::print_status(self.throttle, self.pitch, self.roll, self.mix());
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
            base - self.pitch + self.roll, // M1 front-right
            base + self.pitch + self.roll, // M2 back-right
            base + self.pitch - self.roll, // M3 back-left
            base - self.pitch - self.roll, // M4 front-left
        ];
        duties.map(clamp_duty)
    }

    fn apply(&self, motors: &Arc<Mutex<Motors>>) {
        let duties = self.mix();
        if let Ok(mut motors) = motors.lock() {
            for (motor, &duty) in Motor::ALL.iter().zip(duties.iter()) {
                let _ = motors.set_duty_percent(*motor, duty);
            }
        }
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
