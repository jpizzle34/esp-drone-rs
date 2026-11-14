/** Live X-frame. Nose is +Y, right is +X, up is +Z. Duty index is M1..M4. */

export type DroneView = {
  setAttitude(roll: number, pitch: number, yaw: number): void;
  setMotors(duties: number[], armed: boolean): void;
};

type Vec3 = { x: number; y: number; z: number };
type Pt = { x: number; y: number; depth: number };

const ARM = 1.05;
const ROTOR = 0.42;
const EYE = { x: 0.15, y: -3.15, z: 1.85 };
const LOOK = { x: 0, y: 0, z: 0.05 };

const MOTORS: { name: string; at: Vec3 }[] = [
  { name: "M1", at: { x: ARM, y: ARM, z: 0 } },
  { name: "M2", at: { x: ARM, y: -ARM, z: 0 } },
  { name: "M3", at: { x: -ARM, y: -ARM, z: 0 } },
  { name: "M4", at: { x: -ARM, y: ARM, z: 0 } },
];

export function mountDrone(canvas: HTMLCanvasElement): DroneView {
  const context = canvas.getContext("2d");
  if (!context) {
    throw new Error("drone canvas has no 2d context");
  }
  const ctx: CanvasRenderingContext2D = context;

  const target = { roll: 0, pitch: 0, yaw: 0 };
  const shown = { roll: 0, pitch: 0, yaw: 0 };
  let duties = [0, 0, 0, 0];
  let armed = false;
  const spin = [0, 0, 0, 0];
  let last = performance.now();
  let ariaAt = 0;
  // Measured outside the frame so a serial-log reflow cannot resize the bitmap mid-draw.
  let viewW = canvas.clientWidth;
  let viewH = canvas.clientHeight;
  const resizeObserver = new ResizeObserver((entries) => {
    const rect = entries[0]?.contentRect;
    if (!rect) return;
    const w = Math.round(rect.width);
    const h = Math.round(rect.height);
    if (Math.abs(w - viewW) <= 1 && Math.abs(h - viewH) <= 1) return;
    viewW = w;
    viewH = h;
  });
  resizeObserver.observe(canvas);

  function frame(now: number) {
    const dt = Math.min(0.05, (now - last) / 1000);
    last = now;
    shown.roll = damp(shown.roll, target.roll, dt);
    shown.pitch = damp(shown.pitch, target.pitch, dt);
    shown.yaw = dampAngle(shown.yaw, target.yaw, dt);
    for (let i = 0; i < 4; i++) {
      spin[i] += dt * (0.4 + (duties[i] / 100) * 14);
    }
    draw(ctx, canvas, shown, duties, armed, spin, viewW, viewH);
    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);

  return {
    setAttitude(roll, pitch, yaw) {
      target.roll = roll;
      target.pitch = pitch;
      target.yaw = yaw;
      const now = performance.now();
      if (now - ariaAt < 250) return;
      ariaAt = now;
      canvas.setAttribute(
        "aria-label",
        `Drone attitude roll ${roll.toFixed(0)} degrees, pitch ${pitch.toFixed(0)} degrees, yaw ${yaw.toFixed(0)} degrees`,
      );
    },
    setMotors(next, isArmed) {
      armed = isArmed;
      duties = [0, 1, 2, 3].map((i) => clamp(next[i] ?? 0, 0, 100));
    },
  };
}

function draw(
  ctx: CanvasRenderingContext2D,
  canvas: HTMLCanvasElement,
  pose: { roll: number; pitch: number; yaw: number },
  duties: number[],
  armed: boolean,
  spin: number[],
  w: number,
  h: number,
) {
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  if (w < 2 || h < 2) return;
  if (canvas.width !== Math.floor(w * dpr) || canvas.height !== Math.floor(h * dpr)) {
    canvas.width = Math.floor(w * dpr);
    canvas.height = Math.floor(h * dpr);
  }
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, w, h);

  const forward = norm(sub(LOOK, EYE));
  const right = norm(cross(forward, { x: 0, y: 0, z: 1 }));
  const up = cross(right, forward);
  const scale = Math.min(w, h) * 1.22;

  const project = (p: Vec3): Pt => {
    const rel = sub(p, EYE);
    const depth = Math.max(0.25, dot(rel, forward));
    return {
      x: w / 2 + (dot(rel, right) / depth) * scale,
      y: h / 2 - (dot(rel, up) / depth) * scale,
      depth,
    };
  };

  const body = (p: Vec3) => project(rotate(p, pose.roll, pose.pitch, pose.yaw));

  drawGrid(ctx, project);

  type Item = { depth: number; paint: () => void };
  const items: Item[] = [];

  const deck = box(0.34, 0.52, 0.02, 0.14);
  for (const face of faces(deck)) {
    const world = face.map((p) => rotate(p, pose.roll, pose.pitch, pose.yaw));
    const pts = world.map(project);
    const depth = avg(pts.map((p) => p.depth));
    items.push({
      depth,
      paint: () => {
        poly(ctx, pts, "#243140", "rgba(139,152,168,0.45)");
      },
    });
  }

  for (let i = 0; i < MOTORS.length; i++) {
    const motor = MOTORS[i];
    const duty = duties[i] ?? 0;
    const hub = { ...motor.at, z: 0.08 };
    const disc = circle(hub, ROTOR, spin[i]);
    const pts = disc.map((p) => body(p));
    const depth = avg(pts.map((p) => p.depth));
    const hot = armed ? duty / 100 : 0;
    items.push({
      depth: depth + 0.02,
      paint: () => {
        const root = body({ x: motor.at.x * 0.22, y: motor.at.y * 0.22, z: 0.06 });
        const tip = body(hub);
        ctx.strokeStyle = "#6d7c8d";
        ctx.lineWidth = 7;
        ctx.lineCap = "round";
        ctx.beginPath();
        ctx.moveTo(root.x, root.y);
        ctx.lineTo(tip.x, tip.y);
        ctx.stroke();
      },
    });
    items.push({
      depth,
      paint: () => {
        poly(
          ctx,
          pts,
          `rgba(61,187,143,${(0.08 + hot * 0.45).toFixed(3)})`,
          `rgba(61,187,143,${(0.35 + hot * 0.6).toFixed(3)})`,
        );
        const c = body(hub);
        ctx.fillStyle = hot > 0.04 ? `rgba(232,238,245,${0.35 + hot * 0.55})` : "#8b98a8";
        ctx.beginPath();
        ctx.arc(c.x, c.y, 4 + hot * 3, 0, Math.PI * 2);
        ctx.fill();
        ctx.font = "600 12px IBM Plex Sans, Segoe UI, sans-serif";
        ctx.fillStyle = "#c5d0dc";
        ctx.textAlign = "center";
        ctx.textBaseline = "bottom";
        ctx.fillText(motor.name, c.x, c.y - 16);
        ctx.font = "11px IBM Plex Sans, Segoe UI, sans-serif";
        ctx.fillStyle = hot > 0.04 ? "#3dbb8f" : "#8b98a8";
        ctx.textBaseline = "top";
        ctx.fillText(`${Math.round(duty)}%`, c.x, c.y + 10);
      },
    });
  }

  const nose = [
    { x: 0, y: 0.78, z: 0.16 },
    { x: -0.12, y: 0.48, z: 0.16 },
    { x: 0.12, y: 0.48, z: 0.16 },
  ].map((p) => body(p));
  items.push({
    depth: avg(nose.map((p) => p.depth)) - 0.01,
    paint: () => poly(ctx, nose, "#5b9fd6", "rgba(91,159,214,0.2)"),
  });

  const led = body({ x: 0, y: -0.28, z: 0.16 });
  items.push({
    depth: led.depth - 0.02,
    paint: () => {
      ctx.fillStyle = armed ? "#3dbb8f" : "#5c6770";
      ctx.beginPath();
      ctx.arc(led.x, led.y, 3.5, 0, Math.PI * 2);
      ctx.fill();
    },
  });

  items.sort((a, b) => b.depth - a.depth);
  for (const item of items) item.paint();
  drawAttitude(ctx, w, h, pose);
}

const ROLL_COLOR = "#5b9fd6";
const PITCH_COLOR = "#3dbb8f";
const YAW_COLOR = "#d6a15b";

function drawAttitude(
  ctx: CanvasRenderingContext2D,
  w: number,
  h: number,
  pose: { roll: number; pitch: number; yaw: number },
) {
  ctx.save();
  ctx.font = "600 12px IBM Plex Sans, Segoe UI, sans-serif";
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  if (w >= 680) {
    drawRollCue(ctx, w / 2, 70, pose.roll);
    drawPitchCue(ctx, 78, h * 0.56, pose.pitch);
    drawYawCue(ctx, w - 78, h * 0.62, pose.yaw);
  } else {
    const y = h - 28;
    drawRollCue(ctx, w * 0.2, y, pose.roll, true);
    drawPitchCue(ctx, w * 0.5, y, pose.pitch, true);
    drawYawCue(ctx, w * 0.8, y, pose.yaw, true);
  }
  ctx.restore();
}

function drawRollCue(ctx: CanvasRenderingContext2D, x: number, glyphY: number, roll: number, compact = false) {
  const words = Math.abs(roll) < 1.5 ? "wings level" : roll > 0 ? "right side down" : "left side down";
  cueLabel(ctx, x, glyphY, `roll ${signed(roll)}°`, words, ROLL_COLOR, compact);
  ctx.save();
  ctx.translate(x, glyphY);
  ctx.strokeStyle = "rgba(139,152,168,0.45)";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(-22, 0);
  ctx.lineTo(22, 0);
  ctx.stroke();
  ctx.rotate((clamp(roll, -60, 60) * Math.PI) / 180);
  ctx.strokeStyle = ROLL_COLOR;
  ctx.lineWidth = 2.5;
  ctx.lineCap = "round";
  ctx.beginPath();
  ctx.moveTo(-18, 0);
  ctx.lineTo(-6, 0);
  ctx.moveTo(6, 0);
  ctx.lineTo(18, 0);
  ctx.moveTo(0, -3);
  ctx.lineTo(0, 4);
  ctx.stroke();
  ctx.restore();
}

function drawPitchCue(ctx: CanvasRenderingContext2D, x: number, glyphY: number, pitch: number, compact = false) {
  const words = Math.abs(pitch) < 1.5 ? "level" : pitch > 0 ? "nose up" : "nose down";
  cueLabel(ctx, x, glyphY, `pitch ${signed(pitch)}°`, words, PITCH_COLOR, compact);
  ctx.save();
  ctx.translate(x, glyphY);
  ctx.strokeStyle = "rgba(139,152,168,0.35)";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(-16, 0);
  ctx.lineTo(16, 0);
  ctx.stroke();
  ctx.rotate((-clamp(pitch, -60, 60) * Math.PI) / 180);
  ctx.strokeStyle = PITCH_COLOR;
  ctx.fillStyle = PITCH_COLOR;
  ctx.lineWidth = 2.5;
  ctx.lineCap = "round";
  ctx.beginPath();
  ctx.moveTo(-14, 0);
  ctx.lineTo(8, 0);
  ctx.stroke();
  ctx.beginPath();
  ctx.moveTo(8, -5);
  ctx.lineTo(16, 0);
  ctx.lineTo(8, 5);
  ctx.closePath();
  ctx.fill();
  ctx.restore();
}

function drawYawCue(ctx: CanvasRenderingContext2D, x: number, glyphY: number, yaw: number, compact = false) {
  const heading = norm180(yaw);
  const words = Math.abs(heading) < 1.5 ? "nose ahead" : heading > 0 ? "nose right" : "nose left";
  cueLabel(ctx, x, glyphY, `yaw ${signed(heading)}°`, words, YAW_COLOR, compact, 36);
  ctx.save();
  ctx.translate(x, glyphY);
  ctx.strokeStyle = "rgba(139,152,168,0.55)";
  ctx.fillStyle = "#8b98a8";
  ctx.lineWidth = 1.25;
  ctx.beginPath();
  ctx.arc(0, 0, 16, 0, Math.PI * 2);
  ctx.stroke();
  ctx.beginPath();
  ctx.moveTo(0, -16);
  ctx.lineTo(0, -22);
  ctx.stroke();
  ctx.font = "10px IBM Plex Sans, Segoe UI, sans-serif";
  ctx.textAlign = "center";
  ctx.textBaseline = "bottom";
  ctx.fillText("0", 0, -30);
  ctx.rotate((heading * Math.PI) / 180);
  ctx.fillStyle = YAW_COLOR;
  ctx.beginPath();
  ctx.moveTo(0, -12);
  ctx.lineTo(5, 7);
  ctx.lineTo(0, 3);
  ctx.lineTo(-5, 7);
  ctx.closePath();
  ctx.fill();
  ctx.restore();
}

function cueLabel(
  ctx: CanvasRenderingContext2D,
  x: number,
  glyphY: number,
  value: string,
  words: string,
  color: string,
  compact: boolean,
  lift = 0,
) {
  ctx.textAlign = "center";
  ctx.textBaseline = "middle";
  ctx.fillStyle = color;
  ctx.font = "600 13px IBM Plex Sans, Segoe UI, sans-serif";
  ctx.fillText(value, x, glyphY - (compact ? 30 : 48 + lift));
  if (!compact) {
    ctx.fillStyle = "#8b98a8";
    ctx.font = "12px IBM Plex Sans, Segoe UI, sans-serif";
    ctx.fillText(words, x, glyphY - (28 + lift));
  }
}

function drawGrid(ctx: CanvasRenderingContext2D, project: (p: Vec3) => Pt) {
  ctx.strokeStyle = "rgba(42,51,64,0.9)";
  ctx.lineWidth = 1;
  const z = -0.72;
  for (let i = -4; i <= 4; i++) {
    const t = i * 0.45;
    line(ctx, project({ x: -1.8, y: t, z }), project({ x: 1.8, y: t, z }));
    line(ctx, project({ x: t, y: -1.8, z }), project({ x: t, y: 1.8, z }));
  }
}

function line(ctx: CanvasRenderingContext2D, a: Pt, b: Pt) {
  ctx.beginPath();
  ctx.moveTo(a.x, a.y);
  ctx.lineTo(b.x, b.y);
  ctx.stroke();
}

function poly(ctx: CanvasRenderingContext2D, pts: Pt[], fill: string, stroke: string) {
  if (pts.length < 3) return;
  ctx.beginPath();
  ctx.moveTo(pts[0].x, pts[0].y);
  for (let i = 1; i < pts.length; i++) ctx.lineTo(pts[i].x, pts[i].y);
  ctx.closePath();
  ctx.fillStyle = fill;
  ctx.fill();
  ctx.strokeStyle = stroke;
  ctx.lineWidth = 1.25;
  ctx.stroke();
}

function circle(center: Vec3, radius: number, spin: number): Vec3[] {
  const pts: Vec3[] = [];
  const n = 22;
  for (let i = 0; i < n; i++) {
    const a = spin + (i / n) * Math.PI * 2;
    pts.push({
      x: center.x + Math.cos(a) * radius,
      y: center.y + Math.sin(a) * radius,
      z: center.z,
    });
  }
  return pts;
}

function box(hx: number, hy: number, z0: number, z1: number): Vec3[] {
  const xs = [-hx, hx];
  const ys = [-hy, hy];
  const zs = [z0, z1];
  const corners: Vec3[] = [];
  for (const z of zs) {
    for (const y of ys) {
      for (const x of xs) corners.push({ x, y, z });
    }
  }
  return corners;
}

function faces(corners: Vec3[]): Vec3[][] {
  const quad = (a: number, b: number, c: number, d: number) => [
    corners[a],
    corners[b],
    corners[c],
    corners[d],
  ];
  return [
    quad(0, 1, 3, 2),
    quad(4, 6, 7, 5),
    quad(0, 4, 5, 1),
    quad(2, 3, 7, 6),
    quad(0, 2, 6, 4),
    quad(1, 5, 7, 3),
  ];
}

function rotate(p: Vec3, rollDeg: number, pitchDeg: number, yawDeg: number): Vec3 {
  const roll = (rollDeg * Math.PI) / 180;
  const pitch = (pitchDeg * Math.PI) / 180;
  const yaw = (-yawDeg * Math.PI) / 180;

  const x1 = p.x * Math.cos(roll) + p.z * Math.sin(roll);
  const z1 = -p.x * Math.sin(roll) + p.z * Math.cos(roll);
  const y2 = p.y * Math.cos(pitch) - z1 * Math.sin(pitch);
  const z2 = p.y * Math.sin(pitch) + z1 * Math.cos(pitch);
  return {
    x: x1 * Math.cos(yaw) - y2 * Math.sin(yaw),
    y: x1 * Math.sin(yaw) + y2 * Math.cos(yaw),
    z: z2,
  };
}

function damp(from: number, to: number, dt: number) {
  const err = Math.abs(to - from);
  // Large steps (a fast move, or a frame that arrived late) catch up instead of easing for a third of a second.
  const rate = err > 12 ? 34 : 14;
  const k = 1 - Math.exp(-dt * rate);
  return from + (to - from) * k;
}

function dampAngle(from: number, to: number, dt: number) {
  let delta = ((to - from + 540) % 360) - 180;
  return damp(from, from + delta, dt);
}

function clamp(n: number, min: number, max: number) {
  return Math.max(min, Math.min(max, n));
}

function norm180(deg: number) {
  return ((deg + 540) % 360) - 180;
}

function signed(n: number) {
  const text = n.toFixed(1);
  return n > 0 ? `+${text}` : text;
}

function avg(xs: number[]) {
  return xs.reduce((s, n) => s + n, 0) / xs.length;
}

function sub(a: Vec3, b: Vec3): Vec3 {
  return { x: a.x - b.x, y: a.y - b.y, z: a.z - b.z };
}

function dot(a: Vec3, b: Vec3) {
  return a.x * b.x + a.y * b.y + a.z * b.z;
}

function cross(a: Vec3, b: Vec3): Vec3 {
  return {
    x: a.y * b.z - a.z * b.y,
    y: a.z * b.x - a.x * b.z,
    z: a.x * b.y - a.y * b.x,
  };
}

function norm(a: Vec3): Vec3 {
  const m = Math.hypot(a.x, a.y, a.z) || 1;
  return { x: a.x / m, y: a.y / m, z: a.z / m };
}
