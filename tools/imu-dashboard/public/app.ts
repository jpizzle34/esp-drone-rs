import { mountDrone } from "./droneScene.ts";
import {
  PID_DEFAULTS,
  PID_GAIN_MAX,
  PID_LOOPS,
  formatGain,
  parseGain,
  parsePidLine,
  type PidLoop,
  type PidTerms,
} from "../src/parsePid.ts";

type ImuSample = {
  ax: number;
  ay: number;
  az: number;
  gx: number;
  gy: number;
  gz: number;
  roll: number;
  pitch: number;
  yaw: number;
  temp: number;
  t: number;
};

type LogLine = { text: string };

type BenchMsg = {
  type: "bench";
  armed: boolean;
  joystick: boolean;
  attitude: boolean;
  pulsing: boolean;
  duties: number[];
};

type InMessage =
  | {
      type: "status";
      status: string;
      port: string;
      hz: number;
      error?: string;
    }
  | { type: "sample"; sample: ImuSample }
  | { type: "event"; message: string }
  | { type: "log"; lines: LogLine[]; reset?: boolean }
  | BenchMsg;

// Chart.js UMD attaches to window; ES modules need an explicit read.
const Chart = (globalThis as unknown as { Chart: any }).Chart;

const MAX_POINTS = 300;
const MAX_LOG = 200;

const statusDot = document.querySelector("#statusDot") as HTMLElement;
const statusText = document.querySelector("#statusText") as HTMLElement;
const statusMeta = document.querySelector("#statusMeta") as HTMLElement;
const eventLog = document.querySelector("#eventLog") as HTMLElement;
const hzInput = document.querySelector("#hzInput") as HTMLInputElement;
const dutyInput = document.querySelector("#dutyInput") as HTMLInputElement;
const pulseMs = document.querySelector("#pulseMs") as HTMLInputElement;
const armState = document.querySelector("#armState") as HTMLElement;
const motorQuad = document.querySelector("#motorQuad") as HTMLElement;
const pulseNote = document.querySelector("#pulseNote") as HTMLElement;
const stickHint = document.querySelector("#stickHint") as HTMLElement;
const stickPad = document.querySelector("#stickPad") as HTMLElement;
const attState = document.querySelector("#attState") as HTMLElement;
const attThrust = document.querySelector("#attThrust") as HTMLInputElement;
const attRoll = document.querySelector("#attRoll") as HTMLInputElement;
const attPitch = document.querySelector("#attPitch") as HTMLInputElement;
const serialLog = document.querySelector("#serialLog") as HTMLElement;

const btnOn = document.querySelector("#btnOn") as HTMLButtonElement;
const btnOff = document.querySelector("#btnOff") as HTMLButtonElement;
const btnCal = document.querySelector("#btnCal") as HTMLButtonElement;
const btnImu = document.querySelector("#btnImu") as HTMLButtonElement;
const btnHelp = document.querySelector("#btnHelp") as HTMLButtonElement;
const btnArm = document.querySelector("#btnArm") as HTMLButtonElement;
const btnDisarm = document.querySelector("#btnDisarm") as HTMLButtonElement;
const btnStop = document.querySelector("#btnStop") as HTMLButtonElement;
const btnM1 = document.querySelector("#btnM1") as HTMLButtonElement;
const btnM2 = document.querySelector("#btnM2") as HTMLButtonElement;
const btnM3 = document.querySelector("#btnM3") as HTMLButtonElement;
const btnM4 = document.querySelector("#btnM4") as HTMLButtonElement;
const btnAll = document.querySelector("#btnAll") as HTMLButtonElement;
const btnPulse = document.querySelector("#btnPulse") as HTMLButtonElement;
const btnJoyOn = document.querySelector("#btnJoyOn") as HTMLButtonElement;
const btnJoyOff = document.querySelector("#btnJoyOff") as HTMLButtonElement;
const btnAttOn = document.querySelector("#btnAttOn") as HTMLButtonElement;
const btnAttOff = document.querySelector("#btnAttOff") as HTMLButtonElement;
const btnAttLevel = document.querySelector("#btnAttLevel") as HTMLButtonElement;
const btnAttThrust = document.querySelector("#btnAttThrust") as HTMLButtonElement;
const btnAttAngles = document.querySelector("#btnAttAngles") as HTMLButtonElement;
const btnPidRead = document.querySelector("#btnPidRead") as HTMLButtonElement;
const btnPidApply = document.querySelector("#btnPidApply") as HTMLButtonElement;
const btnPidDefaults = document.querySelector("#btnPidDefaults") as HTMLButtonElement;
const pidState = document.querySelector("#pidState") as HTMLElement;

type PidFields = { kp: HTMLInputElement; ki: HTMLInputElement; kd: HTMLInputElement };

function pidField(loop: PidLoop, term: keyof PidTerms): HTMLInputElement {
  const input = document.querySelector(
    `input[data-loop="${loop}"][data-term="${term}"]`,
  ) as HTMLInputElement | null;
  if (!input) {
    throw new Error(`missing PID input ${loop} ${term}`);
  }
  return input;
}

const pidFields = Object.fromEntries(
  PID_LOOPS.map((loop) => [loop, { kp: pidField(loop, "kp"), ki: pidField(loop, "ki"), kd: pidField(loop, "kd") }]),
) as Record<PidLoop, PidFields>;

const boardGains = {} as Record<PidLoop, PidTerms>;
let seenBoard = false;

const vRoll = document.querySelector("#vRoll") as HTMLElement;
const vPitch = document.querySelector("#vPitch") as HTMLElement;
const vYaw = document.querySelector("#vYaw") as HTMLElement;
const vTemp = document.querySelector("#vTemp") as HTMLElement;

const bars = [0, 1, 2, 3].map(
  (i) => document.querySelector(`[data-bar="${i}"]`) as HTMLElement,
);
const pcts = [0, 1, 2, 3].map(
  (i) => document.querySelector(`[data-pct="${i}"]`) as HTMLElement,
);

const drone = mountDrone(document.querySelector("#droneView") as HTMLCanvasElement);

const lineButtons = [btnOn, btnOff, btnCal, btnImu, btnHelp, btnArm, btnDisarm, btnStop];
const motorButtons = [btnM1, btnM2, btnM3, btnM4, btnAll, btnPulse];

function series(label: string, color: string) {
  return {
    label,
    data: [] as { x: number; y: number }[],
    borderColor: color,
    backgroundColor: "transparent",
    borderWidth: 1.5,
    pointRadius: 0,
    tension: 0.15,
    normalized: true,
    parsing: false,
  };
}

function makeChart(
  canvasId: string,
  datasets: ReturnType<typeof series>[],
  min: number,
  max: number,
) {
  const ctx = (document.querySelector(`#${canvasId}`) as HTMLCanvasElement).getContext("2d");
  return new Chart(ctx, {
    type: "line",
    data: { datasets },
    options: {
      animation: false,
      responsive: true,
      maintainAspectRatio: false,
      resizeDelay: 200,
      // Hover hit-testing restyles the canvas on pointer moves and stalls the airframe frame.
      events: [],
      scales: {
        x: {
          type: "linear",
          ticks: { color: "#8b98a8", maxTicksLimit: 6 },
          grid: { color: "rgba(42,51,64,0.7)" },
        },
        y: {
          min,
          max,
          ticks: { color: "#8b98a8" },
          grid: { color: "rgba(42,51,64,0.7)" },
        },
      },
      plugins: {
        legend: {
          labels: { color: "#c5d0dc", boxWidth: 10, boxHeight: 10 },
        },
        tooltip: { enabled: false },
      },
    },
  });
}

const attChart = makeChart(
  "attChart",
  [
    series("roll", "#5b9fd6"),
    series("pitch", "#3dbb8f"),
    series("yaw", "#d6a15b"),
  ],
  -180,
  180,
);
const accChart = makeChart(
  "accChart",
  [series("ax", "#e07a5f"), series("ay", "#81b29a"), series("az", "#f2cc8f")],
  -4,
  4,
);
const gyroChart = makeChart(
  "gyroChart",
  [series("gx", "#9b5de5"), series("gy", "#00bbf9"), series("gz", "#fee440")],
  -800,
  800,
);

let t0: number | null = null;
let chartQueued = false;

function push(ds: { data: { x: number; y: number }[] }, x: number, y: number) {
  ds.data.push({ x, y });
  if (ds.data.length > MAX_POINTS) {
    ds.data.shift();
  }
}

function scheduleCharts() {
  if (chartQueued) return;
  chartQueued = true;
  // Off the airframe frame. A chart rescale on every sample is what made the
  // 3D view hitch while the board was moving.
  window.setTimeout(() => {
    chartQueued = false;
    attChart.update("none");
    accChart.update("none");
    gyroChart.update("none");
  }, 100);
}

function onSample(sample: ImuSample) {
  if (t0 === null) {
    t0 = sample.t;
  }
  const x = sample.t - t0;

  push(attChart.data.datasets[0], x, sample.roll);
  push(attChart.data.datasets[1], x, sample.pitch);
  push(attChart.data.datasets[2], x, sample.yaw);
  push(accChart.data.datasets[0], x, sample.ax);
  push(accChart.data.datasets[1], x, sample.ay);
  push(accChart.data.datasets[2], x, sample.az);
  push(gyroChart.data.datasets[0], x, sample.gx);
  push(gyroChart.data.datasets[1], x, sample.gy);
  push(gyroChart.data.datasets[2], x, sample.gz);
  scheduleCharts();

  vRoll.textContent = `${sample.roll.toFixed(1)}°`;
  vPitch.textContent = `${sample.pitch.toFixed(1)}°`;
  vYaw.textContent = `${sample.yaw.toFixed(1)}°`;
  vTemp.textContent = `${sample.temp.toFixed(1)}°C`;
  drone.setAttitude(sample.roll, sample.pitch, sample.yaw);
}

function setStatus(msg: Extract<InMessage, { type: "status" }>) {
  statusText.textContent = msg.status;
  statusMeta.textContent = `${msg.port} · ${msg.hz} Hz${msg.error ? ` · ${msg.error}` : ""}`;
  statusDot.className = "dot";
  if (msg.status === "streaming" || msg.status === "connected") {
    statusDot.classList.add("ok");
  } else if (msg.status === "error") {
    statusDot.classList.add("bad");
  } else {
    statusDot.classList.add("warn");
  }
  if (msg.hz) {
    hzInput.value = String(msg.hz);
  }
}

let logQueue: LogLine[] = [];
let logRaf = 0;

function enqueueLog(lines: LogLine[]) {
  if (lines.length === 0) return;
  logQueue.push(...lines);
  if (logQueue.length > MAX_LOG) {
    logQueue.splice(0, logQueue.length - MAX_LOG);
  }
  if (!logRaf) {
    logRaf = requestAnimationFrame(flushLog);
  }
}

function flushLog() {
  logRaf = 0;
  const lines = logQueue;
  logQueue = [];
  if (lines.length === 0) return;
  const stick = serialLog.scrollHeight - serialLog.scrollTop - serialLog.clientHeight <= 32;
  const frag = document.createDocumentFragment();
  for (const line of lines) {
    const row = document.createElement("div");
    row.className = "log-line";
    if (line.text.startsWith("ERR ")) row.classList.add("err");
    else if (line.text.startsWith("OK ")) row.classList.add("ok");
    row.textContent = line.text;
    frag.appendChild(row);
  }
  serialLog.appendChild(frag);
  while (serialLog.childElementCount > MAX_LOG) {
    const first = serialLog.firstElementChild as HTMLElement | null;
    if (!first) break;
    const height = stick ? 0 : first.offsetHeight;
    first.remove();
    if (!stick && height) {
      serialLog.scrollTop = Math.max(0, serialLog.scrollTop - height);
    }
  }
  if (stick) {
    serialLog.scrollTop = serialLog.scrollHeight;
  }
}

function writePid(loop: PidLoop, terms: PidTerms) {
  const fields = pidFields[loop];
  fields.kp.value = formatGain(terms.kp);
  fields.ki.value = formatGain(terms.ki);
  fields.kd.value = formatGain(terms.kd);
}

function readPid(loop: PidLoop): PidTerms | null {
  const fields = pidFields[loop];
  const kp = parseGain(fields.kp.value.trim());
  const ki = parseGain(fields.ki.value.trim());
  const kd = parseGain(fields.kd.value.trim());
  if (kp === null || ki === null || kd === null) return null;
  return { kp, ki, kd };
}

function sameTerms(a: PidTerms, b: PidTerms): boolean {
  return a.kp === b.kp && a.ki === b.ki && a.kd === b.kd;
}

function refreshPidState() {
  let edited = false;
  for (const loop of PID_LOOPS) {
    const terms = readPid(loop);
    if (!terms || !sameTerms(terms, boardGains[loop])) {
      edited = true;
      break;
    }
  }
  if (!seenBoard && !edited) {
    pidState.textContent = "Defaults";
    pidState.className = "state";
  } else if (edited) {
    pidState.textContent = "Edited";
    pidState.className = "state stick";
  } else {
    pidState.textContent = "On board";
    pidState.className = "state att";
  }
}

function absorbPid(lines: LogLine[]) {
  let hit = false;
  for (const line of lines) {
    const parsed = parsePidLine(line.text);
    if (!parsed) continue;
    hit = true;
    boardGains[parsed.loop] = parsed.terms;
    writePid(parsed.loop, parsed.terms);
  }
  if (hit) {
    seenBoard = true;
    refreshPidState();
  }
}

for (const loop of PID_LOOPS) {
  boardGains[loop] = { ...PID_DEFAULTS[loop] };
  writePid(loop, boardGains[loop]);
  for (const input of Object.values(pidFields[loop])) {
    input.addEventListener("input", refreshPidState);
  }
}

function onLog(msg: { lines: LogLine[]; reset?: boolean }) {
  if (msg.reset) {
    logQueue = [];
    if (logRaf) {
      cancelAnimationFrame(logRaf);
      logRaf = 0;
    }
    serialLog.replaceChildren();
  }
  enqueueLog(msg.lines);
  absorbPid(msg.lines);
}

function applyBench(msg: BenchMsg) {
  const duties = msg.duties.length === 4 ? msg.duties : [0, 0, 0, 0];
  const shown = msg.armed ? duties : [0, 0, 0, 0];
  motorQuad.classList.toggle("disarmed", !msg.armed);
  for (let i = 0; i < 4; i++) {
    const duty = Math.max(0, Math.min(100, shown[i] ?? 0));
    bars[i].style.width = `${duty}%`;
    pcts[i].textContent = `${duty}%`;
  }
  armState.className = msg.joystick
    ? "state stick"
    : msg.attitude
      ? "state att"
      : msg.armed
        ? "state armed"
        : "state";
  armState.textContent = msg.joystick ? "Stick" : msg.attitude ? "Attitude" : msg.armed ? "Armed" : "Disarmed";
  attState.className = msg.attitude ? "state att" : "state";
  attState.textContent = msg.attitude ? "On" : "Off";
  motorQuad.setAttribute(
    "aria-label",
    msg.armed
      ? `Motor output ${armState.textContent} M1 ${shown[0]}% M2 ${shown[1]}% M3 ${shown[2]}% M4 ${shown[3]}%`
      : "Motor output disarmed",
  );
  drone.setMotors(shown, msg.armed);

  pulseNote.hidden = !msg.pulsing;
  if (msg.pulsing) {
    pulseNote.textContent = `Pulse ${duties[0]}% — drops to 0 when the board finishes`;
  }

  const blockLines = msg.joystick;
  const blockMotors = !msg.armed || msg.joystick || msg.attitude;
  for (const button of lineButtons) button.disabled = blockLines;
  hzInput.disabled = blockLines;
  for (const button of motorButtons) button.disabled = blockMotors;
  dutyInput.disabled = blockMotors;
  pulseMs.disabled = blockMotors;
  btnJoyOn.disabled = !msg.armed || msg.joystick || msg.attitude;
  btnJoyOff.disabled = !msg.armed || msg.joystick;
  const attLive = msg.attitude && !msg.joystick;
  btnAttOn.disabled = !msg.armed || msg.joystick || msg.attitude;
  btnAttOff.disabled = !attLive;
  btnAttLevel.disabled = !attLive;
  btnAttThrust.disabled = !attLive;
  btnAttAngles.disabled = !attLive;
  attThrust.disabled = !attLive;
  attRoll.disabled = !attLive;
  attPitch.disabled = !attLive;
  const pidLocked = msg.joystick;
  btnPidRead.disabled = pidLocked;
  btnPidApply.disabled = pidLocked;
  btnPidDefaults.disabled = pidLocked;
  for (const loop of PID_LOOPS) {
    for (const input of Object.values(pidFields[loop])) {
      input.disabled = pidLocked;
    }
  }
  stickPad.hidden = !msg.joystick;
  if (msg.joystick) {
    stickHint.textContent = "Stick active. Q quits and disarms. Line commands are off.";
  } else if (msg.attitude) {
    stickHint.textContent = "Attitude mode owns the motors. att off stops them and stays armed.";
  } else if (msg.armed) {
    stickHint.textContent = "joy on enters stick mode. joy off disarms from the prompt. Q is the quit key once the stick is active.";
  } else {
    stickHint.textContent = "Arm first. joy on is rejected while disarmed. Q quits and disarms.";
  }
}

function send(payload: { type: "command"; cmd: string } | { type: "stick"; key: string }) {
  if (ws?.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(payload));
    return;
  }
  eventLog.textContent = "not connected";
}

document.body.addEventListener("click", (ev) => {
  const button = (ev.target as HTMLElement).closest("button");
  if (!button || button.disabled) return;
  button.classList.add("ack");
  window.setTimeout(() => button.classList.remove("ack"), 220);
});

function sendCommand(cmd: string) {
  send({ type: "command", cmd });
}

function sendStick(key: string) {
  send({ type: "stick", key });
}

function readAngle(input: HTMLInputElement): number | null {
  const n = Number(input.value);
  if (!Number.isFinite(n) || n < -30 || n > 30) {
    eventLog.textContent = "angle must be between -30 and 30";
    return null;
  }
  return Math.round(n * 10) / 10;
}

function readRange(input: HTMLInputElement, min: number, max: number, label: string): number | null {
  const n = Number(input.value);
  if (!Number.isInteger(n) || n < min || n > max) {
    eventLog.textContent = `${label} must be an integer ${min}–${max}`;
    return null;
  }
  return n;
}

function sendMotor(index: number) {
  const duty = readRange(dutyInput, 0, 100, "duty");
  if (duty === null) return;
  sendCommand(`motor ${index} ${duty}`);
}

btnOn.addEventListener("click", () => {
  const hz = readRange(hzInput, 1, 100, "Hz");
  if (hz === null) return;
  sendCommand(`imu on ${hz}`);
});
btnOff.addEventListener("click", () => sendCommand("imu off"));
btnCal.addEventListener("click", () => sendCommand("imu cal"));
btnImu.addEventListener("click", () => sendCommand("imu"));
btnHelp.addEventListener("click", () => sendCommand("help"));
btnArm.addEventListener("click", () => sendCommand("arm"));
btnDisarm.addEventListener("click", () => sendCommand("disarm"));
btnStop.addEventListener("click", () => sendCommand("stop"));
btnM1.addEventListener("click", () => sendMotor(1));
btnM2.addEventListener("click", () => sendMotor(2));
btnM3.addEventListener("click", () => sendMotor(3));
btnM4.addEventListener("click", () => sendMotor(4));
btnAll.addEventListener("click", () => {
  const duty = readRange(dutyInput, 0, 100, "duty");
  if (duty === null) return;
  sendCommand(`motor all ${duty}`);
});
btnPulse.addEventListener("click", () => {
  const duty = readRange(dutyInput, 0, 100, "duty");
  const ms = readRange(pulseMs, 1, 30000, "pulse");
  if (duty === null || ms === null) return;
  sendCommand(`motor pulse ${duty} ${ms}`);
});
btnJoyOn.addEventListener("click", () => sendCommand("joy on"));
btnJoyOff.addEventListener("click", () => sendCommand("joy off"));
btnAttOn.addEventListener("click", () => sendCommand("att on"));
btnAttOff.addEventListener("click", () => sendCommand("att off"));
btnAttLevel.addEventListener("click", () => sendCommand("att level"));
btnAttThrust.addEventListener("click", () => {
  const thrust = readRange(attThrust, 0, 100, "thrust");
  if (thrust === null) return;
  sendCommand(`att thrust ${thrust}`);
});
btnAttAngles.addEventListener("click", () => {
  const roll = readAngle(attRoll);
  const pitch = readAngle(attPitch);
  if (roll === null || pitch === null) return;
  sendCommand(`att roll ${roll}`);
  sendCommand(`att pitch ${pitch}`);
});
btnPidRead.addEventListener("click", () => sendCommand("pid"));
btnPidDefaults.addEventListener("click", () => sendCommand("pid defaults"));
btnPidApply.addEventListener("click", () => {
  const commands: string[] = [];
  for (const loop of PID_LOOPS) {
    const fields = pidFields[loop];
    const terms = {
      kp: parseGain(fields.kp.value.trim()),
      ki: parseGain(fields.ki.value.trim()),
      kd: parseGain(fields.kd.value.trim()),
    };
    if (terms.kp === null || terms.ki === null || terms.kd === null) {
      eventLog.textContent = `${loop} gains must be numbers from 0 to ${PID_GAIN_MAX}`;
      return;
    }
    const next = { kp: terms.kp, ki: terms.ki, kd: terms.kd };
    if (!seenBoard || !sameTerms(next, boardGains[loop])) {
      commands.push(`pid ${loop} ${formatGain(next.kp)} ${formatGain(next.ki)} ${formatGain(next.kd)}`);
    }
  }
  if (commands.length === 0) {
    eventLog.textContent = "PID gains already match the board";
    return;
  }
  for (const cmd of commands) sendCommand(cmd);
});

stickPad.addEventListener("click", (ev) => {
  const button = (ev.target as HTMLElement).closest("button");
  if (!button || !stickPad.contains(button)) return;
  const key = button.getAttribute("data-key");
  if (key !== null) sendStick(key);
});

const stickKeys = new Set(["e", "E", "c", "C", "w", "W", "a", "A", "s", "S", "d", "D", "q", "Q", " ", "?"]);

window.addEventListener("keydown", (ev) => {
  if (stickPad.hidden || ev.repeat) return;
  const tag = (ev.target as HTMLElement | null)?.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
  if (!stickKeys.has(ev.key)) return;
  ev.preventDefault();
  sendStick(ev.key);
});

let ws: WebSocket | null = null;

function connect() {
  const proto = location.protocol === "https:" ? "wss" : "ws";
  ws = new WebSocket(`${proto}://${location.host}/ws`);
  ws.onmessage = (ev) => {
    const msg = JSON.parse(String(ev.data)) as InMessage;
    if (msg.type === "status") {
      setStatus(msg);
    } else if (msg.type === "sample") {
      onSample(msg.sample);
    } else if (msg.type === "event") {
      eventLog.textContent = msg.message;
    } else if (msg.type === "log") {
      onLog(msg);
    } else if (msg.type === "bench") {
      applyBench(msg);
    }
  };
  ws.onclose = () => {
    statusText.textContent = "websocket closed — retrying";
    statusDot.className = "dot warn";
    setTimeout(connect, 1000);
  };
}

connect();
