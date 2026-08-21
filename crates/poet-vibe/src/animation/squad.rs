//! Spherical and Quadrangle (SQUAD) Quaternion Spline Interpolation (Zero-Heap).
//!
//! Provides C¹-continuous orientation interpolation across multi-keyframe paths,
//! eliminating instantaneous angular acceleration spikes characteristic of linear SLERP.

/// A 4D Unit Quaternion representing 3D spatial orientation [w, x, y, z].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quat {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Quat {
    pub const fn identity() -> Self {
        Self {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub const fn new(w: f64, x: f64, y: f64, z: f64) -> Self {
        Self { w, x, y, z }
    }

    #[inline]
    pub fn dot(&self, other: &Self) -> f64 {
        self.w * other.w + self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn normalize(&self) -> Self {
        let mag_sq = self.dot(self);
        if mag_sq < 1e-12 {
            return Self::identity();
        }
        let inv_mag = 1.0 / mag_sq.sqrt();
        Self {
            w: self.w * inv_mag,
            x: self.x * inv_mag,
            y: self.y * inv_mag,
            z: self.z * inv_mag,
        }
    }

    pub fn conjugate(&self) -> Self {
        Self {
            w: self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }

    pub fn mul(&self, other: &Self) -> Self {
        Self {
            w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        }
    }

    /// Quaternion logarithm: ln(q) -> imaginary vector [x, y, z].
    pub fn log(&self) -> [f64; 3] {
        let v_len_sq = self.x * self.x + self.y * self.y + self.z * self.z;
        if v_len_sq < 1e-12 {
            return [0.0, 0.0, 0.0];
        }
        let v_len = v_len_sq.sqrt();
        let theta = v_len.atan2(self.w);
        let scale = theta / v_len;
        [self.x * scale, self.y * scale, self.z * scale]
    }

    /// Quaternion exponential: exp([x, y, z]) -> unit quaternion.
    pub fn exp(v: [f64; 3]) -> Self {
        let theta_sq = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
        if theta_sq < 1e-12 {
            return Self::identity();
        }
        let theta = theta_sq.sqrt();
        let sin_theta = theta.sin();
        let scale = sin_theta / theta;
        Self {
            w: theta.cos(),
            x: v[0] * scale,
            y: v[1] * scale,
            z: v[2] * scale,
        }
    }

    /// Standard Spherical Linear Interpolation (SLERP) along the shortest arc.
    pub fn slerp(q0: &Self, q1: &Self, t: f64) -> Self {
        if t <= 0.0 {
            return *q0;
        }
        if t >= 1.0 {
            return *q1;
        }

        let mut dot = q0.dot(q1);
        let mut target = *q1;

        // Take the shortest path on S³
        if dot < 0.0 {
            dot = -dot;
            target = Self {
                w: -target.w,
                x: -target.x,
                y: -target.y,
                z: -target.z,
            };
        }

        if dot > 0.9995 {
            // Linear interpolation fallback when quaternions are nearly parallel
            return Self {
                w: q0.w + t * (target.w - q0.w),
                x: q0.x + t * (target.x - q0.x),
                y: q0.y + t * (target.y - q0.y),
                z: q0.z + t * (target.z - q0.z),
            }
            .normalize();
        }

        let theta = dot.clamp(-1.0, 1.0).acos();
        let sin_theta = theta.sin();
        let s0 = ((1.0 - t) * theta).sin() / sin_theta;
        let s1 = (t * theta).sin() / sin_theta;

        Self {
            w: s0 * q0.w + s1 * target.w,
            x: s0 * q0.x + s1 * target.x,
            y: s0 * q0.y + s1 * target.y,
            z: s0 * q0.z + s1 * target.z,
        }
    }

    /// Compute intermediate control point `a_i` for SQUAD given three consecutive orientations (q_{i-1}, q_i, q_{i+1}).
    pub fn compute_inner_control_point(q_prev: &Self, q_curr: &Self, q_next: &Self) -> Self {
        let q_curr_inv = q_curr.conjugate();
        let log_next = q_curr_inv.mul(q_next).log();
        let log_prev = q_curr_inv.mul(q_prev).log();

        let mut sum = [0.0; 3];
        for k in 0..3 {
            sum[k] = -(log_next[k] + log_prev[k]) * 0.25;
        }

        let exp_term = Self::exp(sum);
        q_curr.mul(&exp_term).normalize()
    }

    /// Spherical and Quadrangle Spline (SQUAD) interpolation between `q_i` and `q_{i+1}`.
    /// Uses inner control points `a_i` and `b_{i+1}` for C¹ continuity.
    pub fn squad(q_i: &Self, q_next: &Self, a_i: &Self, b_next: &Self, t: f64) -> Self {
        let slerp_q = Self::slerp(q_i, q_next, t);
        let slerp_ab = Self::slerp(a_i, b_next, t);
        let factor = 2.0 * t * (1.0 - t);
        Self::slerp(&slerp_q, &slerp_ab, factor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slerp_endpoints() {
        let q0 = Quat::identity();
        let q1 = Quat::new(0.0, 1.0, 0.0, 0.0);
        let mid = Quat::slerp(&q0, &q1, 0.5);
        assert!((mid.w - 0.707106).abs() < 1e-4);
        assert!((mid.x - 0.707106).abs() < 1e-4);
    }

    #[test]
    fn squad_endpoints_match() {
        let q0 = Quat::identity();
        let q1 = Quat::new(0.7071, 0.7071, 0.0, 0.0);
        let q2 = Quat::new(0.0, 1.0, 0.0, 0.0);

        let a0 = Quat::compute_inner_control_point(&q0, &q0, &q1);
        let b1 = Quat::compute_inner_control_point(&q0, &q1, &q2);

        let start = Quat::squad(&q0, &q1, &a0, &b1, 0.0);
        let end = Quat::squad(&q0, &q1, &a0, &b1, 1.0);

        assert!((start.dot(&q0).abs() - 1.0).abs() < 1e-4);
        assert!((end.dot(&q1).abs() - 1.0).abs() < 1e-4);
    }
}
