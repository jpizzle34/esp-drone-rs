import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { SerialPort } from "serialport";
import { WebSocketServer, type WebSocket } from "ws";
import {
  initialBench,
  reduceBench,
  type BenchSnapshot,
  type Duties,
} from "./benchState.ts";
import { parseImuLine, type ImuSample } from "./parseImu.ts";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PUBLIC = path.resolve(__dirname, "../public");
const MAX_LOG = 200;

const STICK_KEYS = new Set([
  "e",
  "E",
  "c",
  "C",
  "w",
  "W",
  "a",
  "A",
  "s",
  "S",
  "d",
  "D",
  "q",
  "Q",
  " ",
  "?",
]);

type LogEntry = { text: string };

type OutMessage =
  | {
      type: "status";
      status: string;
      port: string;
      hz: number;
      error?: string;
    }
  | { type: "sample"; sample: ImuSample & { t: number } }
  | { type: "event"; message: string }
  | { type: "log"; lines: LogEntry[]; reset?: boolean }
  | {
      type: "bench";
      armed: boolean;
      joystick: boolean;
      attitude: boolean;
      pulsing: boolean;
      duties: Duties;
    };

class PortError extends Error {}

function parseArgs(argv: string[]) {
  const opts = {
    port: "/dev/ttyUSB0",
    baud: 115200,
    hz: 25,
    host: "127.0.0.1",
    httpPort: 8765,
    reset: true,
  };

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    const next = argv[i + 1];
    switch (arg) {
      case "--port":
        opts.port = next ?? opts.port;
        i++;
        break;
      case "--baud":
        opts.baud = Number(next ?? opts.baud);
        i++;
        break;
      case "--hz":
        opts.hz = Math.max(1, Math.min(100, Number(next ?? opts.hz)));
        i++;
        break;
      case "--host":
        opts.host = next ?? opts.host;
        i++;
        break;
      case "--http-port":
        opts.httpPort = Number(next ?? opts.httpPort);
        i++;
        break;
      case "--no-reset":
        opts.reset = false;
        break;
      case "--help":
      case "-h":
        console.log(`Usage: npm start -- [options]
  --port PATH        serial device (default /dev/ttyUSB0)
  --baud N           baud rate (default 115200)
  --hz N             imu stream rate 1-100 (default 25)
  --host ADDR        bind address (default 127.0.0.1)
  --http-port N      dashboard port (default 8765)
  --no-reset         do not toggle RTS/DTR`);
        process.exit(0);
    }
  }
  return opts;
}

const opts = parseArgs(process.argv.slice(2));
const clients = new Set<WebSocket>();
const logHistory: LogEntry[] = [];
let serial: SerialPort | null = null;
let status = "starting";
let lastError = "";
let bench: BenchSnapshot = initialBench();
let pulseGen = 0;
let pulseTimer: ReturnType<typeof setTimeout> | null = null;
let readyWatch: ((text: string) => void) | null = null;

function broadcast(message: OutMessage) {
  const raw = JSON.stringify(message);
  for (const client of clients) {
    if (client.readyState === client.OPEN) {
      try {
        client.send(raw);
      } catch {
        // one dead socket should not stop the serial reader
      }
    }
  }
}

function sendEvent(ws: WebSocket, message: string) {
  if (ws.readyState !== ws.OPEN) return;
  ws.send(JSON.stringify({ type: "event", message } satisfies OutMessage));
}

function setStatus(next: string, error = "") {
  status = next;
  lastError = error;
  broadcast({
    type: "status",
    status,
    port: opts.port,
    hz: opts.hz,
    error: error || undefined,
  });
}

function broadcastBench() {
  broadcast({
    type: "bench",
    armed: bench.armed,
    joystick: bench.joystick,
    attitude: bench.attitude,
    pulsing: bench.pulsing,
    duties: bench.duties,
  });
}

function publishBench(next: BenchSnapshot) {
  bench = next;
  broadcastBench();
}

function clearPulseTimer() {
  if (pulseTimer !== null) {
    clearTimeout(pulseTimer);
    pulseTimer = null;
  }
}

function pushLog(text: string) {
  const entry = { text };
  logHistory.push(entry);
  if (logHistory.length > MAX_LOG) {
    logHistory.shift();
  }
  broadcast({ type: "log", lines: [entry] });
}

function applySerial(line: string) {
  const update = reduceBench(bench, line);
  if (update.cancelPulse) {
    pulseGen += 1;
    clearPulseTimer();
  }
  if (update.state !== bench) {
    publishBench(update.state);
  }
}

let writeChain: Promise<void> = Promise.resolve();

function enqueueWrite<T>(job: () => Promise<T>): Promise<T> {
  const run = writeChain.then(job, job);
  writeChain = run.then(
    () => undefined,
    () => undefined,
  );
  return run;
}

function writePort(data: Buffer): Promise<void> {
  return enqueueWrite(() => writePortNow(data));
}

function writePortNow(data: Buffer): Promise<void> {
  const port = serial;
  if (!port?.isOpen) {
    return Promise.reject(new PortError("serial port not open"));
  }
  return new Promise((resolve, reject) => {
    port.write(data, (writeErr) => {
      if (writeErr) {
        reject(new PortError(writeErr.message));
        return;
      }
      port.drain((drainErr) => {
        if (drainErr) reject(new PortError(drainErr.message));
        else resolve();
      });
    });
  });
}

function writeLine(line: string): Promise<void> {
  return writePort(Buffer.from(`${line.trim()}\n`, "utf8"));
}

async function sendLine(cmd: string) {
  if (bench.joystick) {
    throw new Error("stick is active — press Q to quit before line commands");
  }
  const token = ++pulseGen;
  clearPulseTimer();
  if (bench.pulsing) {
    publishBench({ ...bench, pulsing: false, duties: [0, 0, 0, 0] });
  }
  await writeLine(cmd);
  const pulse = /^motor pulse (\d+) (\d+)$/.exec(cmd.trim());
  if (!pulse || token !== pulseGen || !bench.armed || bench.joystick) return;
  const duty = Number(pulse[1]);
  const ms = Number(pulse[2]);
  if (!Number.isInteger(duty) || duty < 0 || duty > 100) return;
  if (!Number.isInteger(ms) || ms < 1 || ms > 30_000) return;
  publishBench({
    armed: bench.armed,
    joystick: false,
    attitude: bench.attitude,
    pulsing: true,
    duties: [duty, duty, duty, duty],
  });
  pulseTimer = setTimeout(() => {
    if (token !== pulseGen) return;
    pulseTimer = null;
    publishBench({ ...bench, pulsing: false, duties: [0, 0, 0, 0] });
  }, ms);
}

async function sendStick(key: string) {
  if (!bench.joystick) {
    throw new Error("joystick mode is not active");
  }
  if (!STICK_KEYS.has(key)) {
    throw new Error("unsupported stick key");
  }
  await writePort(Buffer.from(key, "ascii"));
}

async function sleep(ms: number) {
  await new Promise((resolve) => setTimeout(resolve, ms));
}

function handleLine(line: string) {
  if (!line) return;
  const sample = parseImuLine(line);
  if (sample) {
    // Samples already have their own message. Logging each one at the stream
    // rate forces a DOM layout next to the airframe canvas and hitches it.
    broadcast({ type: "sample", sample: { ...sample, t: Date.now() / 1000 } });
    return;
  }
  pushLog(line);
  applySerial(line);
  if (line.startsWith("IMU fault") || line.startsWith("IMU recovered")) {
    broadcast({ type: "event", message: line });
  }
}

function attachPort(port: SerialPort) {
  let buffer = "";

  port.on("data", (chunk: Buffer) => {
    buffer += chunk.toString("utf8");
    let newline = buffer.indexOf("\n");
    while (newline >= 0) {
      const line = buffer.slice(0, newline).replace(/\r$/, "");
      buffer = buffer.slice(newline + 1);
      handleLine(line);
      readyWatch?.(line);
      newline = buffer.indexOf("\n");
    }
    if (buffer.length > 8192) {
      buffer = buffer.slice(-4096);
    }
    readyWatch?.(buffer);
  });

  port.on("error", (err) => {
    setStatus("error", err.message);
  });

  port.on("close", () => {
    clearPulseTimer();
    if (status !== "error") {
      setStatus("stopped");
    }
  });
}

function waitForReady(timeoutMs = 12_000) {
  return new Promise<void>((resolve) => {
    let settled = false;
    const finish = () => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      readyWatch = null;
      resolve();
    };
    const timer = setTimeout(finish, timeoutMs);
    readyWatch = (text: string) => {
      if (
        text.includes("IMU ready") ||
        text.includes("Serial command task ready") ||
        text.includes("\n>") ||
        text.startsWith("> ")
      ) {
        finish();
      }
    };
  });
}

async function openSerial() {
  setStatus("connecting");
  const port = new SerialPort({
    path: opts.port,
    baudRate: opts.baud,
    autoOpen: false,
  });

  await new Promise<void>((resolve, reject) => {
    port.open((err) => (err ? reject(err) : resolve()));
  });
  serial = port;
  // One reader for the life of the port, before reset and before `imu on`.
  attachPort(port);

  if (opts.reset) {
    setStatus("booting");
    const ready = waitForReady();
    port.set({ dtr: false, rts: true });
    await sleep(100);
    port.set({ dtr: false, rts: false });
    await sleep(50);
    await ready;
  } else {
    setStatus("connected");
  }

  await sleep(300);
  try {
    await writeLine(`imu on ${opts.hz}`);
    await writeLine("pid");
    setStatus("streaming");
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    setStatus("error", message);
    console.error(err);
  }
}

const MIME: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".svg": "image/svg+xml",
  ".ico": "image/x-icon",
};

const httpServer = createServer(async (req, res) => {
  try {
    const url = new URL(req.url ?? "/", `http://${req.headers.host ?? "localhost"}`);
    let filePath = url.pathname === "/" ? "/index.html" : url.pathname;
    filePath = path.normalize(filePath).replace(/^(\.\.[/\\])+/, "");
    const abs = path.join(PUBLIC, filePath);
    if (!abs.startsWith(PUBLIC)) {
      res.writeHead(403).end("forbidden");
      return;
    }
    const data = await readFile(abs);
    const ext = path.extname(abs);
    res.writeHead(200, { "Content-Type": MIME[ext] ?? "application/octet-stream" });
    res.end(data);
  } catch {
    res.writeHead(404).end("not found");
  }
});

const wss = new WebSocketServer({ server: httpServer, path: "/ws" });

wss.on("connection", (ws) => {
  const history = logHistory.slice();
  const benchNow = bench;
  clients.add(ws);
  ws.send(
    JSON.stringify({
      type: "status",
      status,
      port: opts.port,
      hz: opts.hz,
      error: lastError || undefined,
    } satisfies OutMessage),
  );
  ws.send(
    JSON.stringify({
      type: "bench",
      armed: benchNow.armed,
      joystick: benchNow.joystick,
      attitude: benchNow.attitude,
      pulsing: benchNow.pulsing,
      duties: benchNow.duties,
    } satisfies OutMessage),
  );
  ws.send(JSON.stringify({ type: "log", lines: history, reset: true } satisfies OutMessage));

  ws.on("message", (data) => {
    void (async () => {
      let msg: { type?: string; cmd?: string; key?: string };
      try {
        msg = JSON.parse(String(data)) as { type?: string; cmd?: string; key?: string };
      } catch (err) {
        sendEvent(ws, `command failed: ${err instanceof Error ? err.message : String(err)}`);
        return;
      }
      try {
        if (msg.type === "stick") {
          await sendStick(msg.key ?? "");
          return;
        }
        if (msg.type === "command" && msg.cmd) {
          await sendLine(msg.cmd);
        }
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        sendEvent(ws, `command failed: ${message}`);
        if (err instanceof PortError) {
          setStatus("error", message);
        }
      }
    })();
  });

  ws.on("close", () => {
    clients.delete(ws);
  });
});

async function shutdown() {
  clearPulseTimer();
  try {
    if (serial?.isOpen) {
      if (bench.joystick) {
        await writePort(Buffer.from("q", "ascii"));
        await sleep(30);
      }
      await writeLine("imu off");
      await sleep(50);
      await new Promise<void>((resolve) => serial!.close(() => resolve()));
    }
  } catch {
    // ignore
  }
  process.exit(0);
}

process.on("SIGINT", () => void shutdown());
process.on("SIGTERM", () => void shutdown());

httpServer.listen(opts.httpPort, opts.host, () => {
  console.log(`IMU dashboard → http://${opts.host}:${opts.httpPort}`);
  console.log(`Serial ${opts.port} @ ${opts.baud}, stream ${opts.hz} Hz`);
  openSerial().catch((err: Error) => {
    setStatus("error", err.message);
    console.error(err);
  });
});
