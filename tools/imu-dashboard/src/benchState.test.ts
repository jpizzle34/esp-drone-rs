import assert from "node:assert/strict";
import { initialBench, reduceBench, type BenchSnapshot } from "./benchState.ts";

const armed = "OK armed \u2014 motor output enabled";
const disarmed = "OK disarmed \u2014 all motors off";
const joy = "OK joystick mode \u2014 E/C throttle, WASD tilt, Space stop, Q disarm";
const quit = "OK command mode \u2014 type 'help'";
const errDisarmed = "ERR disarmed \u2014 type 'arm' first";

function apply(state: BenchSnapshot, line: string): BenchSnapshot {
  return reduceBench(state, line).state;
}

let state = initialBench();
assert.equal(state.armed, false);
assert.deepEqual([...state.duties], [0, 0, 0, 0]);

state = apply(state, armed);
assert.equal(state.armed, true);
assert.deepEqual([...state.duties], [0, 0, 0, 0]);

state = apply(state, "OK M1 duty 30%");
assert.deepEqual([...state.duties], [30, 0, 0, 0]);

state = apply(state, "OK M3 duty 15%");
assert.deepEqual([...state.duties], [30, 0, 15, 0]);

state = apply(state, "OK all motors duty 20%");
assert.equal(state.armed, true);
assert.deepEqual([...state.duties], [20, 20, 20, 20]);

const finished = reduceBench(state, "OK all motors 20% for 500 ms");
assert.equal(finished.cancelPulse, true);
assert.equal(finished.state.armed, true);
assert.equal(finished.state.pulsing, false);
assert.deepEqual([...finished.state.duties], [0, 0, 0, 0]);
state = finished.state;

state = apply(state, "OK all motors duty 40%");
state = apply(state, disarmed);
assert.equal(state.armed, false);
assert.equal(state.joystick, false);
assert.deepEqual([...state.duties], [0, 0, 0, 0]);

state = apply(initialBench(), errDisarmed);
assert.equal(state.armed, false);

state = apply(apply(initialBench(), armed), "ERR unknown command \u2014 type help");
assert.equal(state.armed, false);
assert.deepEqual([...state.duties], [0, 0, 0, 0]);

state = apply(apply(initialBench(), "OK all motors duty 40%"), joy);
assert.equal(state.joystick, true);
assert.equal(state.armed, true);
assert.deepEqual([...state.duties], [0, 0, 0, 0]);

state = apply(state, "thr 30% pitch +10 roll -5 | M1=35 M2=35 M3=25 M4=15");
assert.deepEqual([...state.duties], [35, 35, 25, 15]);
assert.equal(state.joystick, true);

state = apply(state, "thr 0% pitch +0 roll +0 | M1=0 M2=0 M3=0 M4=0");
assert.deepEqual([...state.duties], [0, 0, 0, 0]);
assert.equal(state.armed, true);

const erred = reduceBench(state, "ERR motor apply failed");
assert.equal(erred.cancelPulse, true);
assert.equal(erred.state.joystick, true, "ERR must not drop joystick mode");
assert.equal(erred.state.armed, false);
assert.deepEqual([...erred.state.duties], [0, 0, 0, 0]);

state = apply(apply(initialBench(), joy), quit);
assert.equal(state.joystick, false);
assert.equal(state.armed, false);
assert.deepEqual([...state.duties], [0, 0, 0, 0]);

const imu =
  "IMU accel=(+0.05,-0.08,+0.92)g gyro=(+0,+0,+0)°/s roll=-4.6° pitch=+3.2° yaw=+1.4° temp=29.1°C";
const afterImu = reduceBench(apply(initialBench(), armed), imu);
assert.equal(afterImu.cancelPulse, false);
assert.equal(afterImu.state.armed, true);
assert.equal(reduceBench(afterImu.state, "OK IMU stream on (25 Hz)").state, afterImu.state);
assert.equal(
  reduceBench(afterImu.state, "OK calibrating gyro \u2014 keep the board still").state,
  afterImu.state,
);

const clamped = apply(initialBench(), "thr 100% pitch +0 roll +0 | M1=140 M2=0 M3=0 M4=0");
assert.equal(clamped.duties[0], 100);

const attOn = "OK attitude on \u2014 holding yaw +12.3\u00b0, thrust 0";
state = apply(apply(initialBench(), armed), attOn);
assert.equal(state.attitude, true);
assert.equal(state.armed, true);
assert.deepEqual([...state.duties], [0, 0, 0, 0]);

state = apply(
  state,
  "att thr 20% sp=+0.0/+0.0/+12.3 meas=+1.2/-0.4/+12.1 | M1=22 M2=22 M3=18 M4=18",
);
assert.equal(state.attitude, true);
assert.deepEqual([...state.duties], [22, 22, 18, 18]);

state = apply(state, "OK attitude off \u2014 motors stopped");
assert.equal(state.attitude, false);
assert.equal(state.armed, true);
assert.deepEqual([...state.duties], [0, 0, 0, 0]);

state = apply(state, disarmed);
assert.equal(state.attitude, false);
assert.equal(state.armed, false);

console.log("benchState ok");
