mod board;
mod motors;

use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::PinDriver;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use motors::BenchMode;

/// Change before flashing: `Led` for wiring checks, `Spin` for prop spin test.
const BENCH_MODE: BenchMode = BenchMode::Spin;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("ESP-Drone Rust firmware — motor bench test ({BENCH_MODE:?})");
    board::log_pinout();

    let _nvs = EspDefaultNvsPartition::take()?;
    log::info!("NVS initialized");

    // Take only the peripherals we need and drop the rest before LEDC/GPIO init
    // to keep main-task stack headroom (debug builds with std are stack-hungry).
    let (mut status_led, mut motor_pwm) = {
        let peripherals = Peripherals::take()?;
        log::info!("Peripherals claimed");
        let drone_pins = board::take_pins(peripherals.pins);
        let status_led = PinDriver::output(drone_pins.status_led)?;
        log::info!("Status LED ready");
        let motor_pwm = board::init_motors(peripherals.ledc, drone_pins.motors)?;
        log::info!("Motor PWM (4x LEDC) ready");
        (status_led, motor_pwm)
    };

    status_led.set_high()?;
    FreeRtos::delay_ms(200);
    status_led.set_low()?;
    FreeRtos::delay_ms(200);

    motor_pwm.run_bench_test(BENCH_MODE)?;

    for _ in 0..2 {
        status_led.set_high()?;
        FreeRtos::delay_ms(150);
        status_led.set_low()?;
        FreeRtos::delay_ms(150);
    }

    log::info!("Idle — status LED heartbeat on D27 / GPIO27");

    loop {
        status_led.set_high()?;
        FreeRtos::delay_ms(500);
        status_led.set_low()?;
        FreeRtos::delay_ms(500);
    }
}
