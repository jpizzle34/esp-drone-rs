import assert from "node:assert/strict";
import { parseImuLine } from "./parseImu.ts";

const line =
  "IMU accel=(+0.05,-0.08,+0.92)g gyro=(+0,+0,+0)°/s roll=-4.6° pitch=+3.2° yaw=+1.4° temp=29.1°C";

const sample = parseImuLine(line);
assert.ok(sample, "expected a sample");
assert.equal(sample.ax, 0.05);
assert.equal(sample.ay, -0.08);
assert.equal(sample.az, 0.92);
assert.equal(sample.gx, 0);
assert.equal(sample.gy, 0);
assert.equal(sample.gz, 0);
assert.equal(sample.roll, -4.6);
assert.equal(sample.pitch, 3.2);
assert.equal(sample.yaw, 1.4);
assert.equal(sample.temp, 29.1);

assert.equal(parseImuLine("not an imu line"), null);
assert.equal(parseImuLine(""), null);

const degAlias =
  "IMU accel=(1,2,3)g gyro=(4,5,6)deg/s roll=7° pitch=8° yaw=9° temp=10°C";
const alias = parseImuLine(degAlias);
assert.ok(alias);
assert.equal(alias.gx, 4);
assert.equal(alias.temp, 10);

console.log("parseImu ok");
