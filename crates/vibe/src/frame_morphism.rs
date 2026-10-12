//! Frame morphisms — Galilean now, Lorentz later (W5).
//!
//! A frame morphism transforms a pose from one reference frame to
//! another. The simplest case is a Galilean transformation: a rigid
//! translation + rotation between frames moving at constant velocity
//! relative to each other. Lorentz transformations (special relativity)
//! are deferred — they require the full 4D spacetime metric and
//! time dilation.
//!
//! ## Why this matters
//!
//! Without frame morphisms, a pose in "camera space" and a pose in
//! "world space" are just two Vec\<f64\> values with no relationship.
//! The engine can't compose them, invert them, or chain them. Frame
//! morphisms make the relationship explicit and computable.
//!
//! ## Galilean transformation
//!
//! A Galilean transformation between frame A and frame B is:
//! - A translation (offset of B's origin in A's coordinates)
//! - A rotation (orientation of B's axes relative to A's)
//! - A velocity (for moving frames — B moves at constant velocity
//!   relative to A)
//!
//! Position transforms as: p_B = R^(-1) * (p_A - offset - v * t)
//! Time is absolute: t_B = t_A (no time dilation)
//!
//! Lorentz transformation (future) would add:
//! - Time dilation: t_B = γ * (t_A - v·x_A / c²)
//! - Length contraction: x_B = γ * (x_A - v * t_A)
//! - Where γ = 1 / sqrt(1 - v²/c²)
//!
//! Reference: `docs/vibescript-full-impl-PLAN.md` §8.15 W5,
//! excellence-first §4.

use crate::value::{Pose, Value};
use std::collections::BTreeMap;

/// A Galilean frame morphism — translation + rotation + velocity (W5).
#[derive(Debug, Clone, PartialEq)]
pub struct GalileanMorphism {
    /// Translation: offset of the target frame's origin in the
    /// source frame's coordinates.
    pub translation: Vec<f64>,
    /// Rotation as a 3x3 matrix (row-major). Identity = no rotation.
    pub rotation: [[f64; 3]; 3],
    /// Velocity of the target frame relative to the source (m/s).
    /// For stationary frames, this is [0, 0, 0].
    pub velocity: Vec<f64>,
}

impl GalileanMorphism {
    /// Identity morphism — no transformation.
    pub fn identity() -> Self {
        Self {
            translation: vec![0.0, 0.0, 0.0],
            rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            velocity: vec![0.0, 0.0, 0.0],
        }
    }

    /// Create a pure translation morphism.
    pub fn translation(tx: f64, ty: f64, tz: f64) -> Self {
        Self {
            translation: vec![tx, ty, tz],
            ..Self::identity()
        }
    }

    /// Create a pure velocity morphism (moving frame, no rotation).
    pub fn velocity(vx: f64, vy: f64, vz: f64) -> Self {
        Self {
            velocity: vec![vx, vy, vz],
            ..Self::identity()
        }
    }

    /// Transform a position from source frame to target frame.
    /// p_target = R^T * (p_source - translation - velocity * t)
    pub fn transform_position(&self, pos: &[f64], t: f64) -> Vec<f64> {
        if pos.len() < 3 {
            return pos.to_vec();
        }
        // Subtract translation and velocity * t
        let x = pos[0] - self.translation[0] - self.velocity[0] * t;
        let y = pos[1] - self.translation[1] - self.velocity[1] * t;
        let z = pos[2] - self.translation[2] - self.velocity[2] * t;
        // Apply inverse rotation (R^T * p)
        let r = &self.rotation;
        let nx = r[0][0] * x + r[1][0] * y + r[2][0] * z;
        let ny = r[0][1] * x + r[1][1] * y + r[2][1] * z;
        let nz = r[0][2] * x + r[1][2] * y + r[2][2] * z;
        vec![nx, ny, nz]
    }

    /// Transform a pose from source frame to target frame.
    /// Time is absolute in Galilean relativity (t_target = t_source).
    pub fn transform_pose(&self, pose: &Pose, t: f64) -> Pose {
        let new_position = self.transform_position(&pose.position, t);
        // Transform orientation (rotate by inverse rotation)
        let new_orientation = if pose.orientation.len() >= 4 {
            // Quaternion rotation by inverse frame rotation
            // For simplicity, keep orientation as-is for now
            // (full quaternion composition is a future enhancement)
            pose.orientation.clone()
        } else {
            pose.orientation.clone()
        };
        Pose {
            position: new_position,
            orientation: new_orientation,
            frame: None, // Target frame
        }
    }

    /// Inverse morphism — transforms from target back to source.
    pub fn inverse(&self) -> Self {
        // Inverse: swap translation sign, transpose rotation, negate velocity
        let r = &self.rotation;
        // Transpose rotation
        let rt = [
            [r[0][0], r[1][0], r[2][0]],
            [r[0][1], r[1][1], r[2][1]],
            [r[0][2], r[1][2], r[2][2]],
        ];
        // Inverse translation: -R^T * translation
        let inv_tx = -(rt[0][0] * self.translation[0]
            + rt[0][1] * self.translation[1]
            + rt[0][2] * self.translation[2]);
        let inv_ty = -(rt[1][0] * self.translation[0]
            + rt[1][1] * self.translation[1]
            + rt[1][2] * self.translation[2]);
        let inv_tz = -(rt[2][0] * self.translation[0]
            + rt[2][1] * self.translation[1]
            + rt[2][2] * self.translation[2]);
        // Inverse velocity: -R^T * velocity
        let inv_vx = -(rt[0][0] * self.velocity[0]
            + rt[0][1] * self.velocity[1]
            + rt[0][2] * self.velocity[2]);
        let inv_vy = -(rt[1][0] * self.velocity[0]
            + rt[1][1] * self.velocity[1]
            + rt[1][2] * self.velocity[2]);
        let inv_vz = -(rt[2][0] * self.velocity[0]
            + rt[2][1] * self.velocity[1]
            + rt[2][2] * self.velocity[2]);
        Self {
            translation: vec![inv_tx, inv_ty, inv_tz],
            rotation: rt,
            velocity: vec![inv_vx, inv_vy, inv_vz],
        }
    }

    /// Compose two morphisms: self ∘ other (apply other first, then self).
    pub fn compose(&self, other: &Self) -> Self {
        // For Galilean: combined translation = R_self * other.translation + self.translation
        // combined rotation = R_self * R_other
        // combined velocity = R_self * other.velocity + self.velocity
        let r = &self.rotation;
        let ro = &other.rotation;
        // Combined rotation: R_self * R_other
        let cr = [
            [
                r[0][0] * ro[0][0] + r[0][1] * ro[1][0] + r[0][2] * ro[2][0],
                r[0][0] * ro[0][1] + r[0][1] * ro[1][1] + r[0][2] * ro[2][1],
                r[0][0] * ro[0][2] + r[0][1] * ro[1][2] + r[0][2] * ro[2][2],
            ],
            [
                r[1][0] * ro[0][0] + r[1][1] * ro[1][0] + r[1][2] * ro[2][0],
                r[1][0] * ro[0][1] + r[1][1] * ro[1][1] + r[1][2] * ro[2][1],
                r[1][0] * ro[0][2] + r[1][1] * ro[1][2] + r[1][2] * ro[2][2],
            ],
            [
                r[2][0] * ro[0][0] + r[2][1] * ro[1][0] + r[2][2] * ro[2][0],
                r[2][0] * ro[0][1] + r[2][1] * ro[1][1] + r[2][2] * ro[2][1],
                r[2][0] * ro[0][2] + r[2][1] * ro[1][2] + r[2][2] * ro[2][2],
            ],
        ];
        // Combined translation: R_self * other.translation + self.translation
        let ot = &other.translation;
        let ct = vec![
            r[0][0] * ot[0] + r[0][1] * ot[1] + r[0][2] * ot[2] + self.translation[0],
            r[1][0] * ot[0] + r[1][1] * ot[1] + r[1][2] * ot[2] + self.translation[1],
            r[2][0] * ot[0] + r[2][1] * ot[1] + r[2][2] * ot[2] + self.translation[2],
        ];
        // Combined velocity: R_self * other.velocity + self.velocity
        let ov = &other.velocity;
        let cv = vec![
            r[0][0] * ov[0] + r[0][1] * ov[1] + r[0][2] * ov[2] + self.velocity[0],
            r[1][0] * ov[0] + r[1][1] * ov[1] + r[1][2] * ov[2] + self.velocity[1],
            r[2][0] * ov[0] + r[2][1] * ov[1] + r[2][2] * ov[2] + self.velocity[2],
        ];
        Self {
            translation: ct,
            rotation: cr,
            velocity: cv,
        }
    }

    /// Convert to a Value::Record for inspection and graph storage.
    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("kind".into(), Value::String("galilean".into()));
        rec.insert(
            "translation".into(),
            Value::List(self.translation.iter().map(|v| Value::F64(*v)).collect()),
        );
        rec.insert(
            "rotation".into(),
            Value::List(
                self.rotation
                    .iter()
                    .flat_map(|row| row.iter().map(|v| Value::F64(*v)))
                    .collect(),
            ),
        );
        rec.insert(
            "velocity".into(),
            Value::List(self.velocity.iter().map(|v| Value::F64(*v)).collect()),
        );
        Value::Record(rec)
    }
}

/// A Lorentz frame morphism — special relativistic 4D spacetime transformation.
///
/// Implements the exact 4-vector boost for arbitrary velocity vectors $\vec{v}$ with $|\vec{v}| < c$:
/// $$ct' = \gamma (ct - \vec{\beta} \cdot \vec{r})$$
/// $$\vec{r}' = \vec{r} + \left( \frac{\gamma - 1}{v^2} (\vec{v} \cdot \vec{r}) - \gamma t \right) \vec{v}$$
/// Preserving the Minkowski spacetime invariant interval:
/// $$s^2 = (c \Delta t)^2 - \Delta x^2 - \Delta y^2 - \Delta z^2$$
#[derive(Debug, Clone, PartialEq)]
pub struct LorentzMorphism {
    /// Velocity of the target frame relative to the source (m/s).
    pub velocity: Vec<f64>,
    /// Speed of light (default: 299_792_458 m/s).
    pub c: f64,
}

impl LorentzMorphism {
    /// Standard speed of light in vacuum (m/s).
    pub const C_DEFAULT: f64 = 299_792_458.0;

    /// Create a new Lorentz boost morphism.
    pub fn new(velocity: Vec<f64>, c: f64) -> Self {
        Self { velocity, c }
    }

    /// Create an identity Lorentz morphism (zero boost).
    pub fn identity() -> Self {
        Self {
            velocity: vec![0.0, 0.0, 0.0],
            c: Self::C_DEFAULT,
        }
    }

    /// Create a pure velocity boost along given vector.
    pub fn boost(vx: f64, vy: f64, vz: f64) -> Self {
        Self {
            velocity: vec![vx, vy, vz],
            c: Self::C_DEFAULT,
        }
    }

    /// Lorentz factor γ = 1 / sqrt(1 - v²/c²)
    pub fn gamma(&self) -> f64 {
        let v2 = self.velocity.iter().map(|v| v * v).sum::<f64>();
        let c2 = self.c * self.c;
        if v2 >= c2 {
            return f64::INFINITY;
        }
        1.0 / (1.0 - v2 / c2).sqrt()
    }

    /// Compute the 4x4 Lorentz boost transformation matrix $\Lambda^\mu_{\ \nu}$.
    /// Basis order: [0]=ct, [1]=x, [2]=y, [3]=z.
    pub fn boost_matrix_4x4(&self) -> [[f64; 4]; 4] {
        let vx = self.velocity.first().copied().unwrap_or(0.0);
        let vy = self.velocity.get(1).copied().unwrap_or(0.0);
        let vz = self.velocity.get(2).copied().unwrap_or(0.0);
        let v2 = vx * vx + vy * vy + vz * vz;
        if v2 == 0.0 {
            return [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ];
        }

        let gamma = self.gamma();
        let bx = vx / self.c;
        let by = vy / self.c;
        let bz = vz / self.c;
        let beta2 = v2 / (self.c * self.c);
        let g_minus_1_over_b2 = (gamma - 1.0) / beta2;

        [
            [gamma, -gamma * bx, -gamma * by, -gamma * bz],
            [-gamma * bx, 1.0 + g_minus_1_over_b2 * bx * bx, g_minus_1_over_b2 * bx * by, g_minus_1_over_b2 * bx * bz],
            [-gamma * by, g_minus_1_over_b2 * by * bx, 1.0 + g_minus_1_over_b2 * by * by, g_minus_1_over_b2 * by * bz],
            [-gamma * bz, g_minus_1_over_b2 * bz * bx, g_minus_1_over_b2 * bz * by, 1.0 + g_minus_1_over_b2 * bz * bz],
        ]
    }

    /// Transform an event given as time t (seconds) and position [x, y, z] (meters).
    /// Returns (t_prime, [x_prime, y_prime, z_prime]).
    pub fn transform_event(&self, t: f64, pos: &[f64]) -> (f64, Vec<f64>) {
        let ct = self.c * t;
        let (ct_prime, r_prime) = self.boost_event(ct, pos);
        (ct_prime / self.c, r_prime)
    }

    /// Transform a 4-vector event (ct, [x, y, z]).
    pub fn boost_event(&self, ct: f64, r: &[f64]) -> (f64, Vec<f64>) {
        let rx = r.first().copied().unwrap_or(0.0);
        let ry = r.get(1).copied().unwrap_or(0.0);
        let rz = r.get(2).copied().unwrap_or(0.0);

        let vx = self.velocity.first().copied().unwrap_or(0.0);
        let vy = self.velocity.get(1).copied().unwrap_or(0.0);
        let vz = self.velocity.get(2).copied().unwrap_or(0.0);
        let v2 = vx * vx + vy * vy + vz * vz;

        if v2 == 0.0 {
            return (ct, vec![rx, ry, rz]);
        }

        let gamma = self.gamma();
        if gamma.is_infinite() {
            return (f64::INFINITY, vec![f64::INFINITY, f64::INFINITY, f64::INFINITY]);
        }

        let v_dot_r = vx * rx + vy * ry + vz * rz;
        let beta_dot_r = v_dot_r / self.c;

        // ct' = γ (ct - β · r)
        let ct_prime = gamma * (ct - beta_dot_r);

        // r' = r + ((γ - 1) * (v · r) / v² - γ * (ct / c)) * v
        let t = ct / self.c;
        let factor = ((gamma - 1.0) / v2) * v_dot_r - gamma * t;
        let rx_prime = rx + factor * vx;
        let ry_prime = ry + factor * vy;
        let rz_prime = rz + factor * vz;

        (ct_prime, vec![rx_prime, ry_prime, rz_prime])
    }

    /// Transform 3D position at time t (compatibility with Galilean interface).
    pub fn transform_position(&self, pos: &[f64], t: f64) -> Vec<f64> {
        self.transform_event(t, pos).1
    }

    /// Invariant spacetime interval: s² = (ct)² - (x² + y² + z²).
    pub fn invariant_interval_squared(ct: f64, r: &[f64]) -> f64 {
        let x = r.first().copied().unwrap_or(0.0);
        let y = r.get(1).copied().unwrap_or(0.0);
        let z = r.get(2).copied().unwrap_or(0.0);
        ct * ct - (x * x + y * y + z * z)
    }

    /// Inverse boost morphism — transforms back by reversing velocity.
    pub fn inverse(&self) -> Self {
        Self {
            velocity: self.velocity.iter().map(|v| -v).collect(),
            c: self.c,
        }
    }

    /// Convert to Vibe Value record for scripting.
    pub fn to_value(&self) -> Value {
        let mut rec = std::collections::BTreeMap::new();
        rec.insert(
            "velocity".into(),
            Value::List(self.velocity.iter().map(|v| Value::F64(*v)).collect()),
        );
        rec.insert("c".into(), Value::F64(self.c));
        rec.insert("gamma".into(), Value::F64(self.gamma()));
        rec.insert("honesty".into(), Value::String(self.honesty_label().into()));
        Value::Record(rec)
    }

    /// Honesty label — Lorentz boost is fully implemented.
    pub fn honesty_label(&self) -> &'static str {
        "real"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── GalileanMorphism tests ────────────────────────────────────────

    #[test]
    fn w5_identity_transforms_unchanged() {
        let m = GalileanMorphism::identity();
        let pos = vec![1.0, 2.0, 3.0];
        let result = m.transform_position(&pos, 0.0);
        assert_eq!(result, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn w5_translation_transforms_position() {
        let m = GalileanMorphism::translation(10.0, 20.0, 30.0);
        let pos = vec![1.0, 2.0, 3.0];
        let result = m.transform_position(&pos, 0.0);
        assert_eq!(result, vec![-9.0, -18.0, -27.0]);
    }

    #[test]
    fn w5_velocity_transforms_position_over_time() {
        let m = GalileanMorphism::velocity(1.0, 0.0, 0.0);
        let pos = vec![10.0, 0.0, 0.0];
        // At t=5, the frame has moved 5m in x
        let result = m.transform_position(&pos, 5.0);
        assert_eq!(result, vec![5.0, 0.0, 0.0]);
    }

    #[test]
    fn w5_inverse_round_trip() {
        let m = GalileanMorphism::translation(10.0, 20.0, 30.0);
        let pos = vec![1.0, 2.0, 3.0];
        let transformed = m.transform_position(&pos, 0.0);
        let inv = m.inverse();
        let recovered = inv.transform_position(&transformed, 0.0);
        for i in 0..3 {
            assert!((recovered[i] - pos[i]).abs() < 1e-10);
        }
    }

    #[test]
    fn w5_compose_identity_with_any_is_any() {
        let id = GalileanMorphism::identity();
        let m = GalileanMorphism::translation(5.0, 0.0, 0.0);
        let composed = id.compose(&m);
        let pos = vec![1.0, 2.0, 3.0];
        let result = composed.transform_position(&pos, 0.0);
        assert_eq!(result, vec![-4.0, 2.0, 3.0]);
    }

    #[test]
    fn w5_compose_two_translations() {
        let m1 = GalileanMorphism::translation(5.0, 0.0, 0.0);
        let m2 = GalileanMorphism::translation(0.0, 10.0, 0.0);
        let composed = m1.compose(&m2);
        let pos = vec![0.0, 0.0, 0.0];
        let result = composed.transform_position(&pos, 0.0);
        // Apply m2 first (translate by [0,10,0]), then m1 (translate by [5,0,0])
        // Result: [0-0-5, 0-10-0, 0] = [-5, -10, 0]
        assert_eq!(result, vec![-5.0, -10.0, 0.0]);
    }

    #[test]
    fn w5_transform_pose() {
        let m = GalileanMorphism::translation(10.0, 0.0, 0.0);
        let pose = Pose {
            position: vec![5.0, 0.0, 0.0],
            orientation: vec![1.0, 0.0, 0.0, 0.0],
            frame: Some("source".into()),
        };
        let result = m.transform_pose(&pose, 0.0);
        assert_eq!(result.position, vec![-5.0, 0.0, 0.0]);
    }

    #[test]
    fn w5_velocity_inverse_round_trip() {
        let m = GalileanMorphism::velocity(3.0, 4.0, 0.0);
        let pos = vec![10.0, 20.0, 30.0];
        let t = 2.0;
        let transformed = m.transform_position(&pos, t);
        let inv = m.inverse();
        let recovered = inv.transform_position(&transformed, t);
        for i in 0..3 {
            assert!((recovered[i] - pos[i]).abs() < 1e-10);
        }
    }

    #[test]
    fn w5_to_value() {
        let m = GalileanMorphism::translation(1.0, 2.0, 3.0);
        let v = m.to_value();
        match v {
            Value::Record(r) => {
                assert_eq!(r.get("kind"), Some(&Value::String("galilean".into())));
                assert!(r.contains_key("translation"));
                assert!(r.contains_key("rotation"));
                assert!(r.contains_key("velocity"));
            }
            _ => panic!("expected Record"),
        }
    }

    // ── LorentzMorphism tests ─────────────────────────────────────────

    #[test]
    fn w5_lorentz_gamma_at_rest() {
        let m = LorentzMorphism {
            velocity: vec![0.0, 0.0, 0.0],
            c: 299_792_458.0,
        };
        assert!((m.gamma() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn w5_lorentz_gamma_at_half_c() {
        let c = 299_792_458.0;
        let m = LorentzMorphism {
            velocity: vec![c * 0.5, 0.0, 0.0],
            c,
        };
        let expected = 1.0 / (1.0_f64 - 0.25).sqrt();
        assert!((m.gamma() - expected).abs() < 1e-10);
    }

    #[test]
    fn w5_lorentz_gamma_at_c_is_infinite() {
        let c = 299_792_458.0;
        let m = LorentzMorphism {
            velocity: vec![c, 0.0, 0.0],
            c,
        };
        assert!(m.gamma().is_infinite());
    }

    #[test]
    fn w5_lorentz_honesty_is_real() {
        let m = LorentzMorphism {
            velocity: vec![1000.0, 0.0, 0.0],
            c: 299_792_458.0,
        };
        assert_eq!(m.honesty_label(), "real");
    }

    #[test]
    fn w5_lorentz_invariant_interval_preserved() {
        let c = 299_792_458.0;
        let m = LorentzMorphism::new(vec![0.6 * c, 0.0, 0.0], c);
        let t = 2.0;
        let pos = vec![300_000_000.0, 50.0, -100.0];
        let ct = c * t;
        let s2_orig = LorentzMorphism::invariant_interval_squared(ct, &pos);

        let (ct_prime, r_prime) = m.boost_event(ct, &pos);
        let s2_boosted = LorentzMorphism::invariant_interval_squared(ct_prime, &r_prime);

        let diff = (s2_orig - s2_boosted).abs();
        assert!(diff / s2_orig.abs() < 1e-10, "Minkowski metric s^2 must be invariant under Lorentz boost: orig={s2_orig}, boosted={s2_boosted}");
    }

    #[test]
    fn w5_lorentz_inverse_recovers_original_event() {
        let c = 299_792_458.0;
        let m = LorentzMorphism::new(vec![0.3 * c, -0.2 * c, 0.1 * c], c);
        let inv = m.inverse();

        let t = 1.5;
        let pos = vec![100.0, 200.0, 300.0];
        let (t_prime, r_prime) = m.transform_event(t, &pos);
        let (t_rec, r_rec) = inv.transform_event(t_prime, &r_prime);

        assert!((t - t_rec).abs() < 1e-10, "Time must invert accurately");
        assert!((pos[0] - r_rec[0]).abs() < 1e-6, "X must invert accurately");
        assert!((pos[1] - r_rec[1]).abs() < 1e-6, "Y must invert accurately");
        assert!((pos[2] - r_rec[2]).abs() < 1e-6, "Z must invert accurately");
    }
}
