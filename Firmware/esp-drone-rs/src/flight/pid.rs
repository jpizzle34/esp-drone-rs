//! PID regulator.
//!
//! Same update order as Crazyflie `pid.c` (GPL-3): P, then D from the error
//! delta, then a clamped integrator. The D term is not low-pass filtered; the
//! ESP-Drone attitude path leaves that filter off. The first `update` after
//! `reset` reports zero D so enabling the loop does not kick the motors.

/// Proportional, integral, and derivative gains. The integrator clamp stays on the [`Pid`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PidGains {
    pub kp: f32,
    pub ki: f32,
    pub kd: f32,
}

/// PID state. `i_limit` and `output_limit` of `0` mean "no limit", matching `pid.c`.
#[derive(Debug, Clone)]
pub struct Pid {
    kp: f32,
    ki: f32,
    kd: f32,
    i_limit: f32,
    output_limit: f32,
    integ: f32,
    prev_error: f32,
    primed: bool,
}

impl Pid {
    pub fn new(kp: f32, ki: f32, kd: f32, i_limit: f32) -> Self {
        Self {
            kp,
            ki,
            kd,
            i_limit,
            output_limit: 0.0,
            integ: 0.0,
            prev_error: 0.0,
            primed: false,
        }
    }

    pub fn reset(&mut self) {
        self.integ = 0.0;
        self.prev_error = 0.0;
        self.primed = false;
    }

    pub fn gains(&self) -> PidGains {
        PidGains {
            kp: self.kp,
            ki: self.ki,
            kd: self.kd,
        }
    }

    /// Replace Kp/Ki/Kd. Integrator and derivative history are left alone;
    /// the cascade resets a loop itself when its gains actually change.
    pub fn set_gains(&mut self, gains: PidGains) {
        self.kp = gains.kp;
        self.ki = gains.ki;
        self.kd = gains.kd;
    }

    /// `error` is desired − measured. `dt` is seconds since the previous update.
    pub fn update(&mut self, error: f32, dt: f32) -> f32 {
        let p = self.kp * error;

        let deriv = if !self.primed {
            self.primed = true;
            0.0
        } else if dt > 1.0e-4 {
            (error - self.prev_error) / dt
        } else {
            0.0
        };
        let d = self.kd * deriv;

        self.integ += error * dt;
        if self.i_limit > 0.0 {
            self.integ = self.integ.clamp(-self.i_limit, self.i_limit);
        }
        let i = self.ki * self.integ;

        self.prev_error = error;

        let output = p + i + d;
        if self.output_limit > 0.0 {
            output.clamp(-self.output_limit, self.output_limit)
        } else {
            output
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Pid, PidGains};

    #[test]
    fn proportional_only() {
        let mut pid = Pid::new(2.0, 0.0, 0.0, 0.0);
        assert!((pid.update(3.0, 0.01) - 6.0).abs() < 1.0e-4);
    }

    #[test]
    fn integrator_clamps() {
        let mut pid = Pid::new(0.0, 10.0, 0.0, 0.05);
        let _ = pid.update(1.0, 1.0);
        let second = pid.update(1.0, 1.0);
        assert!(
            (second - 0.5).abs() < 1.0e-4,
            "integ clamped to 0.05, ki 10 → 0.5, got {second}"
        );
    }

    #[test]
    fn first_derivative_is_zero_then_tracks_delta() {
        let mut pid = Pid::new(0.0, 0.0, 2.0, 0.0);
        assert_eq!(pid.update(4.0, 0.01), 0.0);
        let second = pid.update(5.0, 0.01);
        assert!((second - 200.0).abs() < 1.0e-2, "got {second}");
    }

    #[test]
    fn reset_clears_integral() {
        let mut pid = Pid::new(0.0, 1.0, 0.0, 100.0);
        let _ = pid.update(10.0, 1.0);
        pid.reset();
        assert_eq!(pid.update(0.0, 0.01), 0.0);
    }

    #[test]
    fn set_gains_takes_effect_on_the_next_update() {
        let mut pid = Pid::new(1.0, 0.0, 0.0, 0.0);
        pid.set_gains(PidGains {
            kp: 4.0,
            ki: 0.0,
            kd: 0.0,
        });
        assert!((pid.update(2.0, 0.01) - 8.0).abs() < 1.0e-4);
    }
}
