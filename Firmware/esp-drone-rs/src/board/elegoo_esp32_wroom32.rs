//! Pin map for the Elegoo ESP32-WROOM-32 devkit (30-pin headers, CP2102 USB).
//!
//! **Active profile: `POC_LEFT_HEADER`** — all drone signals on the left header
//! (USB at bottom, VIN at bottom-left). See `docs/hardware/` for diagrams.
//!
//! GPIO numbers match ESP32-DevKitC and other WROOM-32 clones; silkscreen `D32` = GPIO 32.
//!
//! A future drone-PCB profile (`ESPLANE_V1`) uses mixed left/right pins and is
//! documented for reference only.

use esp_idf_hal::gpio::{Gpio13, Gpio14, Gpio25, Gpio26, Gpio27, Gpio32, Gpio33, Pins};
use esp_idf_hal::ledc::{config::TimerConfig, LedcDriver, LedcTimerDriver, LEDC};

use crate::motors::Motors;

use super::{Motor, MotorMeta};

pub const BOARD_NAME: &str = "Elegoo ESP32-WROOM-32";
pub const PIN_PROFILE: &str = "POC_LEFT_HEADER";

/// Human-readable label for a board GPIO (number + devkit silkscreen).
#[derive(Clone, Copy)]
pub struct BoardPin {
    pub gpio: u8,
    pub header: &'static str,
}

/// Single source of truth for motor bench logs, [`MOTOR_TABLE`], and [`init_motors`] order.
#[derive(Clone, Copy)]
pub struct MotorPinDef {
    pub motor: Motor,
    pub gpio: u8,
    pub header: &'static str,
    pub corner: &'static str,
}

pub const STATUS_LED_PIN: BoardPin = BoardPin {
    gpio: 27,
    header: "D27",
};
pub const I2C_SDA_PIN: BoardPin = BoardPin {
    gpio: 14,
    header: "D14",
};
pub const I2C_SCL_PIN: BoardPin = BoardPin {
    gpio: 13,
    header: "D13",
};

/// M1 → M4 wiring for [`PIN_PROFILE`] (Crazyflie X-quad corners).
pub const MOTOR_PINS: [MotorPinDef; 4] = [
    MotorPinDef {
        motor: Motor::M1,
        gpio: 32,
        header: "D32",
        corner: "front-right",
    },
    MotorPinDef {
        motor: Motor::M2,
        gpio: 33,
        header: "D33",
        corner: "back-right",
    },
    MotorPinDef {
        motor: Motor::M3,
        gpio: 25,
        header: "D25",
        corner: "back-left",
    },
    MotorPinDef {
        motor: Motor::M4,
        gpio: 26,
        header: "D26",
        corner: "front-left",
    },
];

const fn motor_meta(def: MotorPinDef) -> MotorMeta {
    MotorMeta {
        motor: def.motor,
        gpio: def.gpio,
        header: def.header,
        corner: def.corner,
    }
}

/// Bench-test metadata derived from [`MOTOR_PINS`].
pub const MOTOR_TABLE: [MotorMeta; 4] = [
    motor_meta(MOTOR_PINS[0]),
    motor_meta(MOTOR_PINS[1]),
    motor_meta(MOTOR_PINS[2]),
    motor_meta(MOTOR_PINS[3]),
];

// `take_pins` / `init_motors` must use the same GPIO numbers as `MOTOR_PINS`.
const _: () = {
    assert!(STATUS_LED_PIN.gpio == 27);
    assert!(I2C_SDA_PIN.gpio == 14);
    assert!(I2C_SCL_PIN.gpio == 13);
    assert!(MOTOR_PINS[0].gpio == 32);
    assert!(MOTOR_PINS[1].gpio == 33);
    assert!(MOTOR_PINS[2].gpio == 25);
    assert!(MOTOR_PINS[3].gpio == 26);
};

/// Brushed motor PWM pins for [`PIN_PROFILE`] (Crazyflie X-quad corners).
pub struct MotorPinMap {
    pub m1: Gpio32<'static>,
    pub m2: Gpio33<'static>,
    pub m3: Gpio25<'static>,
    pub m4: Gpio26<'static>,
}

/// All drone peripherals claimed from [`Pins`] for the active wiring profile.
pub struct DronePins {
    /// External status LED + resistor on D27 (onboard LED is GPIO2 on the right header).
    pub status_led: Gpio27<'static>,
    pub motors: MotorPinMap,
    /// MPU-6050 I2C — reserved for future sensor bring-up.
    #[allow(dead_code)]
    pub i2c: (Gpio14<'static>, Gpio13<'static>),
}

/// Claim GPIOs for [`PIN_PROFILE`]. Each pin may only be taken once.
///
/// Motor GPIO assignment order matches [`MOTOR_PINS`] (M1 → M4).
pub fn take_pins(pins: Pins) -> DronePins {
    let Pins {
        gpio27,
        gpio32,
        gpio33,
        gpio25,
        gpio26,
        gpio14,
        gpio13,
        ..
    } = pins;

    DronePins {
        status_led: gpio27,
        motors: MotorPinMap {
            m1: gpio32,
            m2: gpio33,
            m3: gpio25,
            m4: gpio26,
        },
        i2c: (gpio14, gpio13),
    }
}

/// Wire [`MotorPinMap`] to LEDC channels and return a ready [`Motors`] driver.
///
/// Channel order follows [`MOTOR_PINS`] (M1 → ch0 … M4 → ch3).
pub fn init_motors(ledc: LEDC, pins: MotorPinMap) -> anyhow::Result<Motors> {
    let MotorPinMap { m1, m2, m3, m4 } = pins;

    let timer_config = TimerConfig::default().frequency(Motors::pwm_frequency());
    let timer = LedcTimerDriver::new(ledc.timer0, &timer_config)?;

    let drivers = [
        LedcDriver::new(ledc.channel0, &timer, m1)?,
        LedcDriver::new(ledc.channel1, &timer, m2)?,
        LedcDriver::new(ledc.channel2, &timer, m3)?,
        LedcDriver::new(ledc.channel3, &timer, m4)?,
    ];

    Motors::from_drivers(timer, drivers)
}

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
        "Status LED: GPIO{} ({}, external) | I2C: SDA=GPIO{} ({}), SCL=GPIO{} ({})",
        STATUS_LED_PIN.gpio,
        STATUS_LED_PIN.header,
        I2C_SDA_PIN.gpio,
        I2C_SDA_PIN.header,
        I2C_SCL_PIN.gpio,
        I2C_SCL_PIN.header,
    );
    for def in MOTOR_PINS {
        log::info!(
            "Motor {}: GPIO{} ({}, {}) — LEDC PWM",
            def.motor.name(),
            def.gpio,
            def.header,
            def.corner,
        );
    }
}
