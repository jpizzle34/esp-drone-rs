# IMU live dashboard

Graphs attitude, accel, and gyro from the ESP32 serial IMU stream in a browser.

Free the serial port first. Stop `cargo`, `espflash`, or any other monitor that holds `/dev/ttyUSB0`.

## Run

```bash
cd tools/imu-dashboard
npm install
npm start
```

Open [http://127.0.0.1:8765](http://127.0.0.1:8765).

`npm start` builds `public/app.js`, opens the serial port, sends `imu on`, and serves the page.

The same page sends the bench REPL (`arm`, motor duty and pulse, `joy on`, attitude setpoints, PID gains) and shows a serial log. Motor bars are commanded duty from the board's `OK` lines, in the POC frame: M4 front-left, M1 front-right, M3 back-left, M2 back-right. Joystick keys are raw bytes with no newline, and only after the stick banner.

## Options

Pass flags after `--`:

```bash
npm start -- --port /dev/ttyUSB0 --hz 50 --no-reset
```

| Flag | Default | Meaning |
|------|---------|---------|
| `--port PATH` | `/dev/ttyUSB0` | Serial device |
| `--baud N` | `115200` | Baud rate |
| `--hz N` | `25` | IMU stream rate (1-100) |
| `--host ADDR` | `127.0.0.1` | Bind address |
| `--http-port N` | `8765` | Dashboard HTTP port |
| `--no-reset` | off | Skip RTS/DTR toggle (use if the board is already running) |

## Checks

```bash
npm run typecheck
npm run test:parse
npm run build:client
```

## Firmware format

With `imu on`, the board prints lines like:

```text
IMU accel=(+0.05,-0.08,+0.92)g gyro=(+0,+0,+0)°/s roll=-4.6° pitch=+3.2° yaw=+1.4° temp=29.1°C
```

See [serial-commands.md](../../docs/hardware/serial-commands.md).
