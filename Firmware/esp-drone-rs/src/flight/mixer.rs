//! X-quad mixer for the POC motor map.
//!
//! Top view, front is the nose:
//!
//! ```text
//!   M4 front-left     M1 front-right
//!   M3 back-left      M2 back-right
//! ```
//!
//! Positive roll drops the right side (more left thrust). Positive pitch raises
//! the nose (more front thrust). Positive yaw yaws the nose right — more thrust
//! on the front-left / back-right pair. Roll and pitch commands arrive already
//! scaled; this is the same corner geometry the joystick uses, with the sign
//! that levels the IMU rather than the open-loop stick.

/// Mix thrust and attitude corrections, in percent, to M1–M4 duty.
pub fn mix_x(thrust: f32, roll: f32, pitch: f32, yaw: f32) -> [u32; 4] {
    let duties = [
        thrust - roll + pitch - yaw,
        thrust - roll - pitch + yaw,
        thrust + roll - pitch - yaw,
        thrust + roll + pitch + yaw,
    ];
    duties.map(clamp_percent)
}

fn clamp_percent(value: f32) -> u32 {
    value.clamp(0.0, 100.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::mix_x;

    #[test]
    fn thrust_only_is_even() {
        assert_eq!(mix_x(40.0, 0.0, 0.0, 0.0), [40, 40, 40, 40]);
    }

    #[test]
    fn positive_roll_drops_the_right_side() {
        let [m1, m2, m3, m4] = mix_x(40.0, 10.0, 0.0, 0.0);
        assert!(m3 > 40 && m4 > 40, "left motors should rise, got {m3} {m4}");
        assert!(
            m1 < 40 && m2 < 40,
            "right motors should fall, got {m1} {m2}"
        );
    }

    #[test]
    fn positive_pitch_lifts_the_nose() {
        let [m1, m2, m3, m4] = mix_x(40.0, 0.0, 10.0, 0.0);
        assert!(
            m1 > 40 && m4 > 40,
            "front motors should rise, got {m1} {m4}"
        );
        assert!(m2 < 40 && m3 < 40, "back motors should fall, got {m2} {m3}");
    }

    #[test]
    fn positive_yaw_is_a_couple() {
        let [m1, m2, m3, m4] = mix_x(40.0, 0.0, 0.0, 10.0);
        assert!(m2 > 40 && m4 > 40, "BR/FL should rise, got {m2} {m4}");
        assert!(m1 < 40 && m3 < 40, "FR/BL should fall, got {m1} {m3}");
    }

    #[test]
    fn duties_clamp() {
        assert_eq!(mix_x(90.0, -20.0, 0.0, 0.0)[0], 100);
        assert_eq!(mix_x(5.0, 20.0, 0.0, 0.0)[0], 0);
    }
}
