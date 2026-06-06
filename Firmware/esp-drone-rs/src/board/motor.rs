//! Logical motor slots (M1–M4). GPIO mapping lives in each board file.

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[repr(usize)]
pub enum Motor {
    M1 = 0,
    M2 = 1,
    M3 = 2,
    M4 = 3,
}

impl Motor {
    pub const ALL: [Motor; 4] = [Motor::M1, Motor::M2, Motor::M3, Motor::M4];

    pub const fn as_usize(self) -> usize {
        self as usize
    }

    pub const fn name(self) -> &'static str {
        match self {
            Motor::M1 => "M1",
            Motor::M2 => "M2",
            Motor::M3 => "M3",
            Motor::M4 => "M4",
        }
    }
}

/// Bench-test / wiring metadata for one motor (GPIO number is for humans + logs).
pub struct MotorMeta {
    pub motor: Motor,
    pub gpio: u8,
    pub header: &'static str,
    pub corner: &'static str,
}
