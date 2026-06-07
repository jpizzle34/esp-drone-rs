//! Motor wiring metadata and compile-time ordering checks.
//!
//! Board profiles supply [`MotorPinMeta`] (GPIO + labels, no domain types).
//! This module builds [`MotorMeta`] tables and validates M1→M4 slot order.

use super::slot::Motor;

/// Board-supplied wiring descriptor for one motor slot (no [`Motor`] enum).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MotorPinMeta {
    pub gpio: u8,
    pub header: &'static str,
    pub corner: &'static str,
}

/// Full motor table entry: logical slot + board wiring metadata.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MotorMeta {
    pub motor: Motor,
    pub gpio: u8,
    pub header: &'static str,
    pub corner: &'static str,
}

/// Build M1→M4 table from board pin metadata (index 0 = M1, … index 3 = M4).
pub const fn build_table(pins: &[MotorPinMeta; 4]) -> [MotorMeta; 4] {
    [
        MotorMeta {
            motor: Motor::M1,
            gpio: pins[0].gpio,
            header: pins[0].header,
            corner: pins[0].corner,
        },
        MotorMeta {
            motor: Motor::M2,
            gpio: pins[1].gpio,
            header: pins[1].header,
            corner: pins[1].corner,
        },
        MotorMeta {
            motor: Motor::M3,
            gpio: pins[2].gpio,
            header: pins[2].header,
            corner: pins[2].corner,
        },
        MotorMeta {
            motor: Motor::M4,
            gpio: pins[3].gpio,
            header: pins[3].header,
            corner: pins[3].corner,
        },
    ]
}

/// Assert table row *i* carries motor slot *i* (M1=0 … M4=3).
pub const fn assert_slot_order(table: &[MotorMeta; 4]) {
    assert!(table[0].motor as usize == 0);
    assert!(table[1].motor as usize == 1);
    assert!(table[2].motor as usize == 2);
    assert!(table[3].motor as usize == 3);
}

/// Assert GPIO numbers in `order` match the board pin metadata (M1→M4 index order).
pub const fn assert_gpio_order(order: &[u8; 4], pins: &[MotorPinMeta; 4]) {
    assert!(order[0] == pins[0].gpio);
    assert!(order[1] == pins[1].gpio);
    assert!(order[2] == pins[2].gpio);
    assert!(order[3] == pins[3].gpio);
}

/// Assert metadata GPIOs match the typed-claim order used when wiring LEDC ch0→ch3.
pub const fn assert_table_gpio_order(table: &[MotorMeta; 4], order: &[u8; 4]) {
    assert!(table[0].gpio == order[0]);
    assert!(table[1].gpio == order[1]);
    assert!(table[2].gpio == order[2]);
    assert!(table[3].gpio == order[3]);
}
