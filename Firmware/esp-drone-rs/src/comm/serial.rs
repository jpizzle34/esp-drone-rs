//! UART bench command REPL (motor duty, IMU stream gating).
//!
//! Uses the ESP-IDF default **non-blocking** console VFS (no `esp_vfs_dev_uart_use_driver`).
//! That driver switch deadlocks / trips the interrupt WDT in Wokwi and some hosts.
//!
//! **Input modes:** command lines (`> ` prompt) vs joystick (`joy on`, E/C + WASD; `Q` to exit).

mod commands;
mod joystick;

use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use esp_idf_hal::delay::FreeRtos;

use commands::CommandAction;
use joystick::{FlightStick, KeyResult};

use crate::drivers::motors::Motors;
use crate::sensors::ImuService;

/// Poll interval when stdin has no data (ms).
const POLL_MS: u32 = 20;

enum InputMode {
    Command,
    Joystick(FlightStick),
}

/// Background task: poll stdin for lines and dispatch bench commands.
pub fn spawn(
    motors: Arc<Mutex<Motors>>,
    imu: Option<ImuService>,
) -> anyhow::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("serial_cmd".into())
        .stack_size(8192)
        .spawn(move || {
            log::info!("Serial command task ready — type 'help'");
            commands::print_help();

            let mut stdin = io::stdin().lock();
            let mut mode = InputMode::Command;
            let mut line = String::new();
            let mut prompt_shown = false;

            loop {
                match read_byte(&mut stdin) {
                    Ok(Some(byte)) => {
                        prompt_shown = false;
                        match &mut mode {
                            InputMode::Command => match byte {
                                b'\n' | b'\r' => {
                                    if !line.is_empty() {
                                        match commands::handle_line(
                                            &line,
                                            &motors,
                                            imu.as_ref(),
                                        ) {
                                            CommandAction::EnterJoystick => {
                                                mode = InputMode::Joystick(FlightStick::new());
                                                FlightStick::print_enter_banner();
                                            }
                                            CommandAction::None => {}
                                        }
                                        line.clear();
                                    }
                                }
                                0x08 | 0x7f => {
                                    line.pop();
                                }
                                _ if byte.is_ascii() && !byte.is_ascii_control() => {
                                    line.push(byte as char);
                                }
                                _ => {}
                            },
                            InputMode::Joystick(joy) => {
                                if matches!(joy.handle_key(byte, &motors), KeyResult::Quit) {
                                    FlightStick::print_quit_banner();
                                    mode = InputMode::Command;
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        if matches!(mode, InputMode::Command)
                            && !prompt_shown
                            && line.is_empty()
                        {
                            print!("> ");
                            let _ = io::stdout().flush();
                            prompt_shown = true;
                        }
                        FreeRtos::delay_ms(POLL_MS);
                    }
                    Err(err) => {
                        log::error!("serial read failed: {err}");
                        FreeRtos::delay_ms(100);
                    }
                }
            }
        })
        .map_err(|err| anyhow::anyhow!("failed to spawn serial command task: {err}"))
}

fn read_byte(stdin: &mut impl Read) -> io::Result<Option<u8>> {
    let mut byte = [0u8; 1];
    match stdin.read(&mut byte) {
        Ok(0) => Ok(None),
        Ok(1) => Ok(Some(byte[0])),
        Ok(_) => Ok(None),
        Err(err) if err.kind() == io::ErrorKind::WouldBlock => Ok(None),
        Err(err) => Err(err),
    }
}
