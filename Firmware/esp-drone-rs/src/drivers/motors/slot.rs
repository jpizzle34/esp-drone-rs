//! Logical motor slots (M1–M4). Board profiles map each slot to GPIO.

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

impl TryFrom<usize> for Motor {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Self::ALL.get(value).copied().ok_or(())
    }
}
