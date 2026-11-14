export type ImuSample = {
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
};

const IMU_RE =
  /IMU accel=\(([+-]?\d+(?:\.\d+)?),([+-]?\d+(?:\.\d+)?),([+-]?\d+(?:\.\d+)?)\)g gyro=\(([+-]?\d+(?:\.\d+)?),([+-]?\d+(?:\.\d+)?),([+-]?\d+(?:\.\d+)?)\)(?:°|deg)\/s roll=([+-]?\d+(?:\.\d+)?)° pitch=([+-]?\d+(?:\.\d+)?)° yaw=([+-]?\d+(?:\.\d+)?)° temp=(\d+(?:\.\d+)?)°C/;

export function parseImuLine(line: string): ImuSample | null {
  const match = IMU_RE.exec(line);
  if (!match) {
    return null;
  }
  const [, ax, ay, az, gx, gy, gz, roll, pitch, yaw, temp] = match;
  return {
    ax: Number(ax),
    ay: Number(ay),
    az: Number(az),
    gx: Number(gx),
    gy: Number(gy),
    gz: Number(gz),
    roll: Number(roll),
    pitch: Number(pitch),
    yaw: Number(yaw),
    temp: Number(temp),
  };
}
