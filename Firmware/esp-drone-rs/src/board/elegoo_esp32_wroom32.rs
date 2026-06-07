//! Pin map for the Elegoo ESP32-WROOM-32 devkit (30-pin headers, CP2102 USB).
//!
//! **Profile: `POC_LEFT_HEADER`** — all drone signals on the left header
//! (USB at bottom, VIN at bottom-left). See `docs/hardware/` for diagrams.
//!
//! GPIO numbers match ESP32-DevKitC and other WROOM-32 clones; silkscreen `D32` = GPIO 32.

use esp_idf_hal::gpio::{Gpio13, Gpio14, Gpio25, Gpio26, Gpio27, Gpio32, Gpio33, Pins};
use esp_idf_hal::ledc::{config::TimerConfig, LedcDriver, LedcTimerDriver, LEDC};

use crate::board::profile::{BoardPin, BoardProfile, HasMotorPins};
use crate::drivers::motors::{
    assert_gpio_order, assert_slot_order, assert_table_gpio_order, build_table, MotorMeta,
    MotorPinMeta, Motors, MotorWiring,
};

/// Board profile marker — implements [`BoardProfile`] for the Elegoo POC wiring.
pub struct ElegooPoc;

// ---------------------------------------------------------------------------
// Metadata — GPIO + devkit silkscreen (no HAL types, no domain enums)
// ---------------------------------------------------------------------------

const STATUS_LED: BoardPin = BoardPin {
    gpio: 27,
    header: "D27",
};
const IMU_SDA: BoardPin = BoardPin {
    gpio: 14,
    header: "D14",
};
const IMU_SCL: BoardPin = BoardPin {
    gpio: 13,
    header: "D13",
};

/// M1→M4 for `POC_LEFT_HEADER`. Index order = LEDC ch0→ch3.
const MOTOR_PIN_META: [MotorPinMeta; 4] = [
    MotorPinMeta {
        gpio: 32,
        header: "D32",
        corner: "front-right",
    },
    MotorPinMeta {
        gpio: 33,
        header: "D33",
        corner: "back-right",
    },
    MotorPinMeta {
        gpio: 25,
        header: "D25",
        corner: "back-left",
    },
    MotorPinMeta {
        gpio: 26,
        header: "D26",
        corner: "front-left",
    },
];

const MOTOR_TABLE: [MotorMeta; 4] = build_table(&MOTOR_PIN_META);

// ---------------------------------------------------------------------------
// Typed pin bundles — HAL ownership, grouped by role
// ---------------------------------------------------------------------------

/// Brushed motor PWM lines for `POC_LEFT_HEADER` (Crazyflie X-quad corners).
pub struct MotorPins {
    pub m1: Gpio32<'static>,
    pub m2: Gpio33<'static>,
    pub m3: Gpio25<'static>,
    pub m4: Gpio26<'static>,
}

impl MotorWiring for MotorPins {
    const GPIO_ORDER: [u8; 4] = ElegooPoc::MOTOR_GPIO_ORDER;

    fn wire(self, ledc: LEDC) -> anyhow::Result<Motors> {
        let timer_config = TimerConfig::default().frequency(Motors::pwm_frequency());
        let timer = LedcTimerDriver::new(ledc.timer0, &timer_config)?;

        let drivers = [
            LedcDriver::new(ledc.channel0, &timer, self.m1)?,
            LedcDriver::new(ledc.channel1, &timer, self.m2)?,
            LedcDriver::new(ledc.channel2, &timer, self.m3)?,
            LedcDriver::new(ledc.channel3, &timer, self.m4)?,
        ];

        Motors::from_drivers(timer, drivers)
    }
}

/// MPU-6050 I2C — reserved for future sensor bring-up.
#[allow(dead_code)]
pub struct ImuPins {
    pub sda: Gpio14<'static>,
    pub scl: Gpio13<'static>,
}

/// All drone peripherals for `POC_LEFT_HEADER`, claimed from HAL [`Pins`].
pub struct DronePins {
    /// External status LED + resistor on D27 / GPIO27 (onboard LED is GPIO2).
    pub status_led: Gpio27<'static>,
    pub motors: MotorPins,
    /// MPU-6050 I2C — reserved for future sensor bring-up.
    #[allow(dead_code)]
    pub imu: ImuPins,
}

impl DronePins {
    fn take(pins: Pins) -> Self {
        Self {
            status_led: pins.gpio27,
            motors: MotorPins {
                m1: pins.gpio32,
                m2: pins.gpio33,
                m3: pins.gpio25,
                m4: pins.gpio26,
            },
            imu: ImuPins {
                sda: pins.gpio14,
                scl: pins.gpio13,
            },
        }
    }
}

impl HasMotorPins for DronePins {
    type MotorPins = MotorPins;
    type StatusLedPin = Gpio27<'static>;

    fn split(self) -> (Gpio27<'static>, MotorPins) {
        (self.status_led, self.motors)
    }
}

impl BoardProfile for ElegooPoc {
    type DronePins = DronePins;

    const BOARD_NAME: &'static str = "Elegoo ESP32-WROOM-32";
    const PIN_PROFILE: &'static str = "POC_LEFT_HEADER";
    const MOTOR_GPIO_ORDER: [u8; 4] = [32, 33, 25, 26];

    fn take(pins: Pins) -> DronePins {
        DronePins::take(pins)
    }

    fn status_led() -> BoardPin {
        STATUS_LED
    }

    fn imu_i2c() -> (BoardPin, BoardPin) {
        (IMU_SDA, IMU_SCL)
    }

    fn motor_pin_meta() -> &'static [MotorPinMeta; 4] {
        &MOTOR_PIN_META
    }

    fn motor_table() -> &'static [MotorMeta; 4] {
        &MOTOR_TABLE
    }

    fn log_pinout() {
        let (imu_sda, imu_scl) = Self::imu_i2c();
        let status = Self::status_led();

        log::info!("Board: {} ({})", Self::BOARD_NAME, Self::PIN_PROFILE);
        log::info!(
            "Status LED: GPIO{} ({}, external) | IMU (I2C): SDA=GPIO{} ({}), SCL=GPIO{} ({})",
            status.gpio,
            status.header,
            imu_sda.gpio,
            imu_sda.header,
            imu_scl.gpio,
            imu_scl.header,
        );
        for meta in Self::motor_table() {
            log::info!(
                "Motor {}: GPIO{} ({}, {}) — LEDC PWM",
                meta.motor.name(),
                meta.gpio,
                meta.header,
                meta.corner,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Compile-time glue: metadata ↔ slot order ↔ LEDC wiring order
// ---------------------------------------------------------------------------

const _: () = {
    assert!(STATUS_LED.gpio == 27);
    assert!(IMU_SDA.gpio == 14);
    assert!(IMU_SCL.gpio == 13);
    assert_slot_order(&MOTOR_TABLE);
    assert_gpio_order(&ElegooPoc::MOTOR_GPIO_ORDER, &MOTOR_PIN_META);
    assert_table_gpio_order(&MOTOR_TABLE, &ElegooPoc::MOTOR_GPIO_ORDER);
    assert_gpio_order(&MotorPins::GPIO_ORDER, &MOTOR_PIN_META);
};
