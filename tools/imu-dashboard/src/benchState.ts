/** Bench arm / stick / duty state derived from firmware serial replies. */

export type Duties = [number, number, number, number];

export type BenchSnapshot = {
  armed: boolean;
  joystick: boolean;
  attitude: boolean;
  pulsing: boolean;
  duties: Duties;
};

export type BenchUpdate = {
  state: BenchSnapshot;
  /** This line ended or replaced a pulse, so the local duration timer must stop. */
  cancelPulse: boolean;
};

export function initialBench(): BenchSnapshot {
  return { armed: false, joystick: false, attitude: false, pulsing: false, duties: [0, 0, 0, 0] };
}

function clampDuty(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.min(100, Math.round(value)));
}

function dutiesEqual(a: Duties, b: Duties): boolean {
  return a[0] === b[0] && a[1] === b[1] && a[2] === b[2] && a[3] === b[3];
}

function same(a: BenchSnapshot, b: BenchSnapshot): boolean {
  return (
    a.armed === b.armed &&
    a.joystick === b.joystick &&
    a.attitude === b.attitude &&
    a.pulsing === b.pulsing &&
    dutiesEqual(a.duties, b.duties)
  );
}

/**
 * Apply one serial line.
 *
 * Duty `OK` lines confirm output. `OK all motors N% for M ms` is printed only
 * after the pulse has already stopped, so it clears the bars. `OK command mode`
 * is the joystick quit banner (Q disarms). `ERR` clears armed output but does
 * not leave joystick mode — Q is still the way out.
 */
export function reduceBench(state: BenchSnapshot, line: string): BenchUpdate {
  const text = line.trim();
  let next: BenchSnapshot = {
    armed: state.armed,
    joystick: state.joystick,
    attitude: state.attitude,
    pulsing: state.pulsing,
    duties: [state.duties[0], state.duties[1], state.duties[2], state.duties[3]],
  };
  let cancelPulse = false;

  const zero = (patch: { armed?: boolean; joystick?: boolean }) => {
    next = {
      armed: patch.armed ?? next.armed,
      joystick: patch.joystick ?? next.joystick,
      attitude: false,
      pulsing: false,
      duties: [0, 0, 0, 0],
    };
    cancelPulse = true;
  };

  if (text.startsWith("OK armed")) {
    next.armed = true;
    next.attitude = false;
    next.pulsing = false;
    next.duties = [0, 0, 0, 0];
    cancelPulse = true;
  } else if (text.startsWith("OK disarmed")) {
    zero({ armed: false, joystick: false });
  } else if (text.startsWith("OK joystick mode")) {
    next.armed = true;
    next.joystick = true;
    next.pulsing = false;
    next.duties = [0, 0, 0, 0];
    cancelPulse = true;
  } else if (text.startsWith("OK command mode")) {
    zero({ armed: false, joystick: false });
  } else if (text.startsWith("OK attitude on")) {
    next.armed = true;
    next.joystick = false;
    next.attitude = true;
    next.pulsing = false;
    next.duties = [0, 0, 0, 0];
    cancelPulse = true;
  } else if (text.startsWith("OK attitude off")) {
    next.attitude = false;
    next.pulsing = false;
    next.duties = [0, 0, 0, 0];
    cancelPulse = true;
  } else if (text.startsWith("ERR ")) {
    zero({ armed: false });
  } else {
    const one = /^OK (M[1-4]) duty (\d{1,3})%$/.exec(text);
    const all = /^OK all motors duty (\d{1,3})%$/.exec(text);
    const pulseDone = /^OK all motors (\d{1,3})% for (\d+) ms$/.exec(text);
    const stick =
      /^thr (\d+)% pitch [+-]?\d+ roll [+-]?\d+ \| M1=(\d+) M2=(\d+) M3=(\d+) M4=(\d+)$/.exec(
        text,
      );
    const attitude =
      /^att thr (\d+)% sp=[+-]?\d+(?:\.\d+)?\/[+-]?\d+(?:\.\d+)?\/[+-]?\d+(?:\.\d+)? meas=[+-]?\d+(?:\.\d+)?\/[+-]?\d+(?:\.\d+)?\/[+-]?\d+(?:\.\d+)? \| M1=(\d+) M2=(\d+) M3=(\d+) M4=(\d+)$/.exec(
        text,
      );

    if (one) {
      const index = Number(one[1].slice(1)) - 1;
      next.duties[index] = clampDuty(Number(one[2]));
      next.armed = true;
      next.pulsing = false;
      cancelPulse = true;
    } else if (all) {
      const duty = clampDuty(Number(all[1]));
      next.duties = [duty, duty, duty, duty];
      next.armed = true;
      next.pulsing = false;
      cancelPulse = true;
    } else if (pulseDone) {
      zero({});
    } else if (stick) {
      next.duties = [
        clampDuty(Number(stick[2])),
        clampDuty(Number(stick[3])),
        clampDuty(Number(stick[4])),
        clampDuty(Number(stick[5])),
      ];
      next.armed = true;
      next.joystick = true;
      next.attitude = false;
      next.pulsing = false;
      cancelPulse = true;
    } else if (attitude) {
      next.duties = [
        clampDuty(Number(attitude[2])),
        clampDuty(Number(attitude[3])),
        clampDuty(Number(attitude[4])),
        clampDuty(Number(attitude[5])),
      ];
      next.armed = true;
      next.joystick = false;
      next.attitude = true;
      next.pulsing = false;
      cancelPulse = true;
    }
  }

  if (same(state, next)) {
    return { state, cancelPulse: false };
  }
  return { state: next, cancelPulse };
}
