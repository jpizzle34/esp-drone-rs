//! UART bench command REPL (motor duty, IMU stream gating).
//!
//! Uses the ESP-IDF default **non-blocking** console VFS (no `esp_vfs_dev_uart_use_driver`).
//! That driver switch deadlocks / trips the interrupt WDT in Wokwi and some hosts.
//!
//! **Input modes:** command lines (`> ` prompt) vs joystick (`joy on`, E/C + WASD; `Q` to exit).
//! Command mode echoes typed characters (for Wokwi); joystick mode does not.

mod commands;
mod joystick;

use std::io::{self, Read, Write};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use esp_idf_hal::delay::FreeRtos;

use commands::CommandAction;
use joystick::{FlightStick, KeyResult};

use crate::safety::MotorGate;
use crate::sensors::ImuService;

/// Poll interval when stdin has no data (ms).
const POLL_MS: u32 = 5;

enum InputMode {
    Command,
    Joystick(FlightStick),
}

/// Background task: poll stdin for lines and dispatch bench commands.
pub fn spawn(
    gate: Arc<MotorGate>,
    imu: Option<ImuService>,
) -> anyhow::Result<JoinHandle<()>> {
    thread::Builder::new()
        .name("serial_cmd".into())
        .stack_size(8192)
        .spawn(move || {
            log::info!("Serial command task ready — disarmed; type 'help'");
            commands::print_help();

            let mut stdin = io::stdin().lock();
            let mut mode = InputMode::Command;
            let mut line = String::new();
            let mut prompt_shown = false;

            loop {
                match read_byte(&mut stdin) {
                    Ok(Some(byte)) => match &mut mode {
                        InputMode::Command => match byte {
                            b'\n' | b'\r' => {
                                if line.is_empty() {
                                    continue;
                                }
                                echo_newline();
                                match commands::handle_line(&line, &gate, imu.as_ref()) {
                                    CommandAction::EnterJoystick => {
                                        mode = InputMode::Joystick(FlightStick::new());
                                        FlightStick::print_enter_banner();
                                    }
                                    CommandAction::None => {}
                                }
                                line.clear();
                                prompt_shown = false;
                            }
                            0x08 | 0x7f => {
                                if line.pop().is_some() {
                                    echo_backspace();
                                }
                            }
                            _ if byte.is_ascii() && !byte.is_ascii_control() => {
                                show_prompt(&mut prompt_shown);
                                line.push(byte as char);
                                echo_byte(byte);
                            }
                            _ => {}
                        },
                        InputMode::Joystick(joy) => {
                            if matches!(joy.handle_key(byte, &gate), KeyResult::Quit) {
                                FlightStick::print_quit_banner();
                                mode = InputMode::Command;
                                prompt_shown = false;
                            }
                        }
                    },
                    Ok(None) => {
                        if matches!(mode, InputMode::Command) && line.is_empty() {
                            show_prompt(&mut prompt_shown);
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

fn echo_byte(byte: u8) {
    let _ = io::stdout().write_all(&[byte]);
    let _ = io::stdout().flush();
}

fn echo_backspace() {
    let _ = io::stdout().write_all(b"\x08 \x08");
    let _ = io::stdout().flush();
}

fn echo_newline() {
    let _ = io::stdout().write_all(b"\r\n");
    let _ = io::stdout().flush();
}

fn show_prompt(prompt_shown: &mut bool) {
    if !*prompt_shown {
        print!("> ");
        let _ = io::stdout().flush();
        *prompt_shown = true;
    }
}
