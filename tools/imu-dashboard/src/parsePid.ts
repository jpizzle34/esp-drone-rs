/** Serial `OK pid <loop> <kp> <ki> <kd>` lines from the firmware. */

export const PID_GAIN_MAX = 5000;

export const PID_LOOPS = [
  "roll",
  "pitch",
  "yaw",
  "roll-rate",
  "pitch-rate",
  "yaw-rate",
] as const;

export type PidLoop = (typeof PID_LOOPS)[number];

export type PidTerms = { kp: number; ki: number; kd: number };

/** ESP32 Crazyflie defaults. Same numbers as `CascadeGains::defaults`. */
export const PID_DEFAULTS: Record<PidLoop, PidTerms> = {
  roll: { kp: 5.9, ki: 2.9, kd: 0 },
  pitch: { kp: 5.9, ki: 2.9, kd: 0 },
  yaw: { kp: 6, ki: 1, kd: 0.35 },
  "roll-rate": { kp: 250, ki: 500, kd: 2.5 },
  "pitch-rate": { kp: 250, ki: 500, kd: 2.5 },
  "yaw-rate": { kp: 120, ki: 16.7, kd: 0 },
};

const PID_LINE =
  /^OK pid (roll-rate|pitch-rate|yaw-rate|roll|pitch|yaw) (\S+) (\S+) (\S+)$/;

export function formatGain(value: number): string {
  const scaled = Math.round(value * 10000);
  const whole = Math.trunc(scaled / 10000);
  const frac = Math.abs(scaled % 10000);
  if (frac === 0) return String(whole);
  const fracText = String(frac).padStart(4, "0").replace(/0+$/, "");
  return `${whole}.${fracText}`;
}

export function parseGain(raw: string): number | null {
  if (!/^[+]?\d+(?:\.\d+)?$/.test(raw)) return null;
  const value = Number(raw);
  if (!Number.isFinite(value) || value < 0 || value > PID_GAIN_MAX) return null;
  const rounded = Math.round(value * 10000) / 10000;
  if (rounded < 0 || rounded > PID_GAIN_MAX) return null;
  return rounded;
}

export function parsePidLine(line: string): { loop: PidLoop; terms: PidTerms } | null {
  const match = PID_LINE.exec(line.trim());
  if (!match) return null;
  const kp = parseGain(match[2]);
  const ki = parseGain(match[3]);
  const kd = parseGain(match[4]);
  if (kp === null || ki === null || kd === null) return null;
  return {
    loop: match[1] as PidLoop,
    terms: { kp, ki, kd },
  };
}
