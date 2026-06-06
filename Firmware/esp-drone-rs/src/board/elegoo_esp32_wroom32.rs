//! Pin map for the Elegoo ESP32-WROOM-32 devkit (30-pin headers, CP2102 USB).
//!
//! **Active profile: `POC_LEFT_HEADER`** — all drone signals on the left header
//! (USB at bottom, VIN at bottom-left). See `docs/hardware/` for diagrams.
//!
//! A future drone-PCB profile (`ESPLANE_V1`) uses mixed left/right pins and is
//! documented for reference only.

pub const BOARD_NAME: &str = "Elegoo ESP32-WROOM-32";
pub const PIN_PROFILE: &str = "POC_LEFT_HEADER";

// ---------------------------------------------------------------------------
// POC — left header only (VIN = bottom-left, USB = bottom edge)
// ---------------------------------------------------------------------------

/// Status LED — external LED + resistor on D27 (onboard LED is GPIO2 on the right header).
pub const STATUS_LED: u8 = 27;

pub const I2C0_SDA: u8 = 14;
pub const I2C0_SCL: u8 = 13;

/// Brushed motor PWM — Crazyflie X-quad corners (all pins on left header).
pub const MOTOR_M1: u8 = 32; // D32 — front-right
pub const MOTOR_M2: u8 = 33; // D33 — back-right
pub const MOTOR_M3: u8 = 25; // D25 — back-left
pub const MOTOR_M4: u8 = 26; // D26 — front-left

#[allow(dead_code)]
pub const MOTOR_PINS: [u8; 4] = [MOTOR_M1, MOTOR_M2, MOTOR_M3, MOTOR_M4];

// ---------------------------------------------------------------------------
// Reference — ESPLANE_V1 / mixed-header (not active in POC firmware)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub mod esplane_v1 {
    pub const STATUS_LED: u8 = 2;
    pub const I2C0_SDA: u8 = 21;
    pub const I2C0_SCL: u8 = 22;
    pub const MOTOR_M1: u8 = 4;
    pub const MOTOR_M2: u8 = 33;
    pub const MOTOR_M3: u8 = 32;
    pub const MOTOR_M4: u8 = 25;
}

pub fn log_pinout() {
    log::info!("Board: {} ({})", BOARD_NAME, PIN_PROFILE);
    log::info!(
        "Status LED: GPIO{} (D27, external) | I2C: SDA=GPIO{} (D14), SCL=GPIO{} (D13)",
        STATUS_LED,
        I2C0_SDA,
        I2C0_SCL
    );
    log::info!(
        "Motors (LEDC PWM): M1=GPIO{} (D32, FR), M2=GPIO{} (D33, BR), M3=GPIO{} (D25, BL), M4=GPIO{} (D26, FL)",
        MOTOR_M1,
        MOTOR_M2,
        MOTOR_M3,
        MOTOR_M4
    );
}
