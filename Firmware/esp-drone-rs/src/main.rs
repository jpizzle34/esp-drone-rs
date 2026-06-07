mod board;
mod comm;
mod drivers;
mod estimation;
mod flight;
mod safety;
mod sensors;

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;

use board::{ActiveBoard, BoardProfile, HasMotorPins};
use drivers::motors::{run_bench_test, BenchMode, MotorWiring};

/// Change before flashing: `Led` for wiring checks, `Spin` for prop spin test.
const BENCH_MODE: BenchMode = BenchMode::Spin;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("ESP-Drone Rust firmware — motor bench test ({BENCH_MODE:?})");
    ActiveBoard::log_pinout();

    let _nvs = EspDefaultNvsPartition::take()?;
    log::info!("NVS initialized");

    // Take only the peripherals we need and drop the rest before LEDC/GPIO init
    // to keep main-task stack headroom (debug builds with std are stack-hungry).
    let (mut status_led, mut motors) = {
        let peripherals = Peripherals::take()?;
        log::info!("Peripherals claimed");
        let drone_pins = ActiveBoard::take(peripherals.pins);
        let (status_led_pin, motor_pins) = drone_pins.split();
        let status_led = PinDriver::output(status_led_pin)?;
        log::info!("Status LED ready");
        let motors = motor_pins.wire(peripherals.ledc)?;
        log::info!("Motor PWM (4x LEDC) ready");
        (status_led, motors)
    };

    status_led.set_high()?;
    FreeRtos::delay_ms(200);
    status_led.set_low()?;
    FreeRtos::delay_ms(200);

    run_bench_test(&mut motors, ActiveBoard::motor_table(), BENCH_MODE)?;

    for _ in 0..2 {
        status_led.set_high()?;
        FreeRtos::delay_ms(150);
        status_led.set_low()?;
        FreeRtos::delay_ms(150);
    }

    let status = ActiveBoard::status_led();
    log::info!(
        "Idle — status LED heartbeat on {} / GPIO{}",
        status.header,
        status.gpio
    );

    loop {
        status_led.set_high()?;
        FreeRtos::delay_ms(500);
        status_led.set_low()?;
        FreeRtos::delay_ms(500);
    }
}
