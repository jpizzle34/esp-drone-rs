//! Gated motor output — all PWM paths check [`Arming`] before writing duty.

use std::sync::{Arc, Mutex};

use crate::drivers::motors::{run_arm_chirp, Motor, MotorMeta, Motors};

use super::arming::{Arming, DisarmReason};

pub const DISARMED_MSG: &str = "disarmed — type 'arm' first";

/// Shared motors + arming state for every control path (serial, joystick, future stabilizer).
pub struct MotorGate {
    motors: Arc<Mutex<Motors>>,
    arming: Arming,
}

impl MotorGate {
    pub fn new(motors: Arc<Mutex<Motors>>) -> Arc<Self> {
        Arc::new(Self {
            motors,
            arming: Arming::new(),
        })
    }

    pub fn is_armed(&self) -> bool {
        self.arming.is_armed()
    }

    /// Arm after a short per-motor chirp (M1 → M4) to confirm wiring.
    pub fn arm(&self, motor_table: &[MotorMeta]) -> Result<(), String> {
        if self.is_armed() {
            return Ok(());
        }
        {
            let mut motors = self
                .motors
                .lock()
                .map_err(|_| "motor mutex poisoned".to_string())?;
            run_arm_chirp(&mut motors, motor_table)
                .map_err(|err| format!("arm chirp failed: {err:#}"))?;
        }
        self.arming.arm();
        Ok(())
    }

    /// Cut PWM immediately and clear armed state.
    pub fn disarm(&self, reason: DisarmReason) -> Result<(), String> {
        self.arming.disarm(reason);
        let mut motors = self
            .motors
            .lock()
            .map_err(|_| "motor mutex poisoned".to_string())?;
        motors
            .all_off()
            .map_err(|err| format!("disarm stop failed: {err:#}"))?;
        Ok(())
    }

    /// Stop all motors without changing armed state (joystick reset while armed).
    pub fn stop_output(&self) -> Result<(), String> {
        let mut motors = self
            .motors
            .lock()
            .map_err(|_| "motor mutex poisoned".to_string())?;
        motors
            .all_off()
            .map_err(|err| format!("stop failed: {err:#}"))?;
        Ok(())
    }

    pub fn set_duty_percent(&self, motor: Motor, percent: u32) -> Result<(), String> {
        if !self.is_armed() {
            return Err(DISARMED_MSG.to_string());
        }
        let mut motors = self
            .motors
            .lock()
            .map_err(|_| "motor mutex poisoned".to_string())?;
        motors
            .set_duty_percent(motor, percent)
            .map_err(|err| format!("motor set failed: {err:#}"))?;
        Ok(())
    }

    pub fn set_all_duty_percent(&self, percent: u32) -> Result<(), String> {
        if !self.is_armed() {
            return Err(DISARMED_MSG.to_string());
        }
        let mut motors = self
            .motors
            .lock()
            .map_err(|_| "motor mutex poisoned".to_string())?;
        motors
            .set_all_duty_percent(percent)
            .map_err(|err| format!("motor set failed: {err:#}"))?;
        Ok(())
    }

    pub fn apply_duty_percents(&self, duties: [u32; 4]) -> Result<(), String> {
        if !self.is_armed() {
            return Err(DISARMED_MSG.to_string());
        }
        let mut motors = self
            .motors
            .lock()
            .map_err(|_| "motor mutex poisoned".to_string())?;
        for (motor, &duty) in Motor::ALL.iter().zip(duties.iter()) {
            motors
                .set_duty_percent(*motor, duty)
                .map_err(|err| format!("motor set failed: {err:#}"))?;
        }
        Ok(())
    }
}
