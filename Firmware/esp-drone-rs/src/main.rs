mod board;
mod motors;

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("ESP-Drone Rust firmware — motor spin bench test");
    board::log_pinout();

    let _nvs = EspDefaultNvsPartition::take()?;
    log::info!("NVS initialized");

    // Take only the peripherals we need and drop the rest before LEDC/GPIO init
    // to keep main-task stack headroom (debug builds with std are stack-hungry).
    let (mut status_led, mut motor_pwm) = {
        let peripherals = Peripherals::take()?;
        log::info!("Peripherals claimed");
        let esp_idf_hal::gpio::Pins {
            gpio27,
            gpio32,
            gpio33,
            gpio25,
            gpio26,
            ..
        } = peripherals.pins;
        let status_led = PinDriver::output(gpio27)?;
        log::info!("Status LED on GPIO{} ready", board::STATUS_LED);
        let motor_pwm = motors::Motors::new(peripherals.ledc, gpio32, gpio33, gpio25, gpio26)?;
        log::info!("Motor PWM (4x LEDC) ready");
        (status_led, motor_pwm)
    };

    status_led.set_high()?;
    FreeRtos::delay_ms(200);
    status_led.set_low()?;
    FreeRtos::delay_ms(200);

    motor_pwm.run_sequential_spin_test()?;

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
