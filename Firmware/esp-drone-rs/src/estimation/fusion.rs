//! Mahony attitude filter (gyro + accelerometer).
//!
//! Matches the Crazyflie `sensfusion6.c` Mahony path (GPL-3): gyro in °/s,
//! accelerometer in g, Euler angles in degrees. Axes are the MPU-6050 chip
//! frame — not yet rotated into the airframe. Yaw integrates gyro only and
//! drifts without a magnetometer.

/// Roll / pitch / yaw in degrees.
#[derive(Debug, Clone, Copy, Default)]
pub struct AttitudeDeg {
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
}

/// Specific-force band treated as gravity. A quick move leaves this band;
/// the gyro still integrates, and accelerometer correction resumes once the
/// reading is near 1 g again.
const ACCEL_GRAVITY_MIN_G: f32 = 0.50;
const ACCEL_GRAVITY_MAX_G: f32 = 2.50;

/// Mahony AHRS. `two_kp` / `two_ki` match Crazyflie defaults (`2 * 0.4`, `2 * 0.001`).
pub struct Mahony {
    two_kp: f32,
    two_ki: f32,
    /// Scalar-first quaternion, identity until [`Mahony::seed_from_accel`].
    q: [f32; 4],
    integral: [f32; 3],
}

impl Default for Mahony {
    fn default() -> Self {
        Self::new()
    }
}

impl Mahony {
    pub fn new() -> Self {
        Self {
            two_kp: 2.0 * 0.4,
            two_ki: 2.0 * 0.001,
            q: [1.0, 0.0, 0.0, 0.0],
            integral: [0.0; 3],
        }
    }

    /// Snap roll/pitch to the gravity vector and zero yaw.
    ///
    /// Ignores readings that are not roughly 1 g so a bad sample cannot flip the estimate.
    pub fn seed_from_accel(&mut self, accel_g: [f32; 3]) {
        let [ax, ay, az] = accel_g;
        let norm = (ax * ax + ay * ay + az * az).sqrt();
        if !(0.5..=1.5).contains(&norm) {
            return;
        }

        let ax = (ax / norm).clamp(-1.0, 1.0);
        let roll = ay.atan2(az);
        // Extracted pitch is `asin(gx)`; quaternion pitch is the negation of that.
        let pitch = -ax.asin();
        let (hr, hp) = (roll * 0.5, pitch * 0.5);
        let (cr, sr) = (hr.cos(), hr.sin());
        let (cp, sp) = (hp.cos(), hp.sin());

        self.q = [cp * cr, cp * sr, sp * cr, -sp * sr];
        self.integral = [0.0; 3];
    }

    /// Integrate one sample. `dt_s` is the time since the previous update.
    ///
    /// Accelerometer correction runs only while specific force is near 1 g.
    /// Outside that band the gyro still integrates, so a fast move does not
    /// freeze the estimate or pull it toward the acceleration vector.
    pub fn update(&mut self, gyro_dps: [f32; 3], accel_g: [f32; 3], dt_s: f32) -> AttitudeDeg {
        let mut gx = gyro_dps[0].to_radians();
        let mut gy = gyro_dps[1].to_radians();
        let mut gz = gyro_dps[2].to_radians();

        if accel_is_gravity(accel_g) {
            let [mut ax, mut ay, mut az] = accel_g;
            let recip = inv_sqrt(ax * ax + ay * ay + az * az);
            ax *= recip;
            ay *= recip;
            az *= recip;

            let [qw, qx, qy, qz] = self.q;
            let halfvx = qx * qz - qw * qy;
            let halfvy = qw * qx + qy * qz;
            let halfvz = qw * qw - 0.5 + qz * qz;

            let halfex = ay * halfvz - az * halfvy;
            let halfey = az * halfvx - ax * halfvz;
            let halfez = ax * halfvy - ay * halfvx;

            if self.two_ki > 0.0 {
                self.integral[0] += self.two_ki * halfex * dt_s;
                self.integral[1] += self.two_ki * halfey * dt_s;
                self.integral[2] += self.two_ki * halfez * dt_s;
            } else {
                self.integral = [0.0; 3];
            }

            gx += self.two_kp * halfex;
            gy += self.two_kp * halfey;
            gz += self.two_kp * halfez;
        }

        // Keep the bias estimate applied while correction is gated off.
        if self.two_ki > 0.0 {
            gx += self.integral[0];
            gy += self.integral[1];
            gz += self.integral[2];
        }

        gx *= 0.5 * dt_s;
        gy *= 0.5 * dt_s;
        gz *= 0.5 * dt_s;

        let [qw, qx, qy, qz] = self.q;
        self.q = [
            qw + (-qx * gx - qy * gy - qz * gz),
            qx + (qw * gx + qy * gz - qz * gy),
            qy + (qw * gy - qx * gz + qz * gx),
            qz + (qw * gz + qx * gy - qy * gx),
        ];

        let recip = inv_sqrt(self.q.iter().map(|c| c * c).sum());
        for component in &mut self.q {
            *component *= recip;
        }

        self.euler()
    }

    fn euler(&self) -> AttitudeDeg {
        let [qw, qx, qy, qz] = self.q;
        let gx = (2.0 * (qx * qz - qw * qy)).clamp(-1.0, 1.0);
        let gy = 2.0 * (qw * qx + qy * qz);
        let gz = qw * qw - qx * qx - qy * qy + qz * qz;

        AttitudeDeg {
            yaw: (2.0 * (qw * qz + qx * qy))
                .atan2(qw * qw + qx * qx - qy * qy - qz * qz)
                .to_degrees(),
            pitch: gx.asin().to_degrees(),
            roll: gy.atan2(gz).to_degrees(),
        }
    }
}

fn accel_is_gravity(accel_g: [f32; 3]) -> bool {
    let [ax, ay, az] = accel_g;
    let mag_sq = ax * ax + ay * ay + az * az;
    mag_sq >= ACCEL_GRAVITY_MIN_G * ACCEL_GRAVITY_MIN_G
        && mag_sq <= ACCEL_GRAVITY_MAX_G * ACCEL_GRAVITY_MAX_G
}

fn inv_sqrt(x: f32) -> f32 {
    1.0 / x.sqrt()
}
