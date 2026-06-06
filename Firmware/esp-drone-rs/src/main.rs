mod board;
mod motors;

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("ESP-Drone Rust firmware — motor LED bench test");
    board::log_pinout();

    let peripherals = Peripherals::take()?;
    let _nvs = EspDefaultNvsPartition::take()?;
    log::info!("NVS initialized");

    let ledc = peripherals.ledc;
    let pins = peripherals.pins;
    let mut status_led = PinDriver::output(pins.gpio27)?;
    let mut motor_pwm = motors::Motors::new(
        ledc,
        pins.gpio32,
        pins.gpio33,
        pins.gpio25,
        pins.gpio26,
    )?;

    status_led.set_high()?;
    FreeRtos::delay_ms(200);
    status_led.set_low()?;
    FreeRtos::delay_ms(200);

    motor_pwm.run_sequential_led_test()?;

    for _ in 0..2 {
        status_led.set_high()?;
        FreeRtos::delay_ms(150);
        status_led.set_low()?;
        FreeRtos::delay_ms(150);
    }

    log::info!("Idle — status LED heartbeat on D27 / GPIO{}", board::STATUS_LED);

    loop {
        status_led.set_high()?;
        FreeRtos::delay_ms(500);
        status_led.set_low()?;
        FreeRtos::delay_ms(500);
    }
}
