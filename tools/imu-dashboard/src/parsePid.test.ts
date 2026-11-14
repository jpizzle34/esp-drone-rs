import assert from "node:assert/strict";
import { formatGain, parsePidLine } from "./parsePid.ts";

assert.equal(formatGain(16.7), "16.7");
assert.equal(formatGain(250), "250");
assert.equal(formatGain(0.35), "0.35");
assert.equal(formatGain(0), "0");

const roll = parsePidLine("OK pid roll 5.9 2.9 0");
assert.deepEqual(roll, { loop: "roll", terms: { kp: 5.9, ki: 2.9, kd: 0 } });

const yawRate = parsePidLine("OK pid yaw-rate 120 16.7 0");
assert.equal(yawRate?.loop, "yaw-rate");
assert.equal(yawRate?.terms.kd, 0);

assert.equal(parsePidLine("OK pid roll-rate 250 500 2.5")?.loop, "roll-rate");
assert.equal(parsePidLine("OK attitude on — holding yaw"), null);
assert.equal(parsePidLine("OK pid roll 5.9 2.9"), null);
assert.equal(parsePidLine("OK pid roll -1 2.9 0"), null);
assert.equal(parsePidLine("ERR gain must be 0..=5000, got 'nope'"), null);

console.log("parsePid ok");
