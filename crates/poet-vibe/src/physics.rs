//! Physics primitives — WorldLine continuants and frame morphisms (W2, W5).
//!
//! ## W2: WorldLine as continuant's time-like self
//!
//! A WorldLine is not just an IRI — it is the time-like identity of a
//! continuant. This module adds the temporal trajectory: a sequence of
//! (Instant, Pose) waypoints that trace the continuant through time.
//! The WorldLine IS the continuant's persistence — it is what stays
//! the same while the continuant changes.
//!
//! ## W5: Frame morphisms (Galilean → Lorentz)
//!
//! A Frame morphism transforms coordinates from one frame to another.
//! The Galilean morphism is the low-velocity limit (t' = t, x' = x - vt).
//! The Lorentz morphism is the relativistic transform (with time dilation
//! and length contraction). The morphism is explicit — no implicit
//! conversion between frames.
//!
//! Reference: `docs/vibescript-full-impl-PLAN.md` wish list W2, W5.

use crate::value::{Frame, Pose, WorldLine};
use std::collections::BTreeMap;

/// A waypoint on a worldline — an instant plus the pose at that instant.
#[derive(Debug, Clone, PartialEq)]
pub struct Waypoint {
    /// Unix seconds for this waypoint.
    pub t_secs: i64,
    /// Nanoseconds within the second.
    pub t_nanos: u32,
    /// The pose at this instant.
    pub pose: Pose,
}

impl Waypoint {
    pub fn new(t_secs: i64, t_nanos: u32, pose: Pose) -> Self {
        Self {
            t_secs,
            t_nanos,
            pose,
        }
    }

    /// Total time in nanoseconds.
    pub fn total_nanos(&self) -> i128 {
        self.t_secs as i128 * 1_000_000_000 + self.t_nanos as i128
    }
}

/// A worldline trajectory — the time-like self of a continuant (W2).
///
/// This extends the `WorldLine` value type with the actual temporal
/// trajectory: a sequence of waypoints tracing the continuant through
/// time. The `WorldLine` value is the identity; the `WorldLineTrajectory`
/// is the persistence.
#[derive(Debug, Clone)]
pub struct WorldLineTrajectory {
    /// The worldline identity.
    pub worldline: WorldLine,
    /// Waypoints ordered by time.
    pub waypoints: Vec<Waypoint>,
}

impl WorldLineTrajectory {
    pub fn new(worldline: WorldLine) -> Self {
        Self {
            worldline,
            waypoints: Vec::new(),
        }
    }

    /// Add a waypoint. Waypoints are kept sorted by time.
    pub fn add(&mut self, waypoint: Waypoint) -> &mut Self {
        let insert_pos = self
            .waypoints
            .partition_point(|w| w.total_nanos() < waypoint.total_nanos());
        self.waypoints.insert(insert_pos, waypoint);
        self
    }

    /// Get the pose at a given time by linear interpolation.
    /// Returns None if the time is outside the waypoint range.
    pub fn pose_at(&self, t_secs: i64, t_nanos: u32) -> Option<Pose> {
        if self.waypoints.is_empty() {
            return None;
        }
        let target = t_secs as i128 * 1_000_000_000 + t_nanos as i128;

        // Before first waypoint.
        let first = self.waypoints.first()?;
        let first_t = first.total_nanos();
        if target <= first_t {
            return Some(first.pose.clone());
        }

        // After last waypoint.
        let last = self.waypoints.last()?;
        let last_t = last.total_nanos();
        if target >= last_t {
            return Some(last.pose.clone());
        }

        // Find bracketing waypoints.
        let idx = self.waypoints.partition_point(|w| w.total_nanos() < target);
        let (w0, w1) = (&self.waypoints[idx - 1], &self.waypoints[idx]);
        let t0 = w0.total_nanos();
        let t1 = w1.total_nanos();
        let alpha = (target - t0) as f64 / (t1 - t0) as f64;

        // Linear interpolation of position.
        let pos: Vec<f64> = w0
            .pose
            .position
            .iter()
            .zip(w1.pose.position.iter())
            .map(|(a, b)| a + (b - a) * alpha)
            .collect();
        Some(Pose {
            position: pos,
            orientation: w0.pose.orientation.clone(), // hold orientation
            frame: w0.pose.frame.clone(),
        })
    }

    /// Number of waypoints.
    pub fn len(&self) -> usize {
        self.waypoints.len()
    }

    /// Is the trajectory empty?
    pub fn is_empty(&self) -> bool {
        self.waypoints.is_empty()
    }

    /// Total time span in nanoseconds (last - first).
    pub fn span_nanos(&self) -> Option<i128> {
        if self.waypoints.len() < 2 {
            return None;
        }
        let first = self.waypoints.first()?.total_nanos();
        let last = self.waypoints.last()?.total_nanos();
        Some(last - first)
    }
}

// ── W5: Frame morphisms ─────────────────────────────────────────────────────

/// A Galilean frame morphism — the low-velocity limit.
///
/// t' = t
/// x' = x - v*t
///
/// No time dilation, no length contraction. Valid for v << c.
#[derive(Debug, Clone, PartialEq)]
pub struct GalileanMorphism {
    /// Velocity of the target frame relative to the source frame [vx, vy, vz].
    pub velocity: Vec<f64>,
}

impl GalileanMorphism {
    pub fn new(velocity: Vec<f64>) -> Self {
        Self { velocity }
    }

    /// Transform a position [x, y, z, t] from source to target frame.
    /// Returns [x', y', z', t'].
    pub fn transform(&self, position: &[f64]) -> Vec<f64> {
        if position.len() < 4 {
            return position.to_vec();
        }
        let t = position[3];
        let mut result = Vec::with_capacity(4);
        for i in 0..3 {
            let v = self.velocity.get(i).copied().unwrap_or(0.0);
            result.push(position[i] - v * t);
        }
        result.push(t); // t' = t
        result
    }

    /// The inverse morphism (target → source).
    pub fn inverse(&self) -> Self {
        Self {
            velocity: self.velocity.iter().map(|v| -v).collect(),
        }
    }
}

/// A Lorentz frame morphism — the relativistic transform.
///
/// Along the x-axis with velocity v:
/// t' = gamma * (t - v*x/c²)
/// x' = gamma * (x - v*t)
///
/// where gamma = 1/sqrt(1 - v²/c²).
///
/// Time dilation and length contraction are explicit. Valid for all
/// v < c.
#[derive(Debug, Clone, PartialEq)]
pub struct LorentzMorphism {
    /// Velocity of the target frame along the x-axis (m/s).
    pub velocity: f64,
    /// Speed of light (m/s). Default: 299_792_458.0.
    pub c: f64,
}

impl LorentzMorphism {
    const C: f64 = 299_792_458.0;

    pub fn new(velocity: f64) -> Self {
        Self {
            velocity,
            c: Self::C,
        }
    }

    /// Create with a custom speed of light (for testing).
    pub fn with_c(velocity: f64, c: f64) -> Self {
        Self { velocity, c }
    }

    /// Lorentz factor gamma = 1/sqrt(1 - v²/c²).
    pub fn gamma(&self) -> f64 {
        let beta_sq = (self.velocity * self.velocity) / (self.c * self.c);
        1.0 / (1.0 - beta_sq).sqrt()
    }

    /// Transform [t, x, y, z] from source to target frame.
    /// Returns [t', x', y', z'].
    pub fn transform(&self, coords: &[f64]) -> Vec<f64> {
        if coords.len() < 4 {
            return coords.to_vec();
        }
        let t = coords[0];
        let x = coords[1];
        let y = coords[2];
        let z = coords[3];
        let g = self.gamma();
        let v = self.velocity;
        let c = self.c;
        let t_prime = g * (t - v * x / (c * c));
        let x_prime = g * (x - v * t);
        vec![t_prime, x_prime, y, z]
    }

    /// The inverse morphism (target → source).
    pub fn inverse(&self) -> Self {
        Self {
            velocity: -self.velocity,
            c: self.c,
        }
    }
}

/// Apply a frame morphism to a Frame, producing the transformed frame.
pub fn transform_frame(frame: &Frame, morphism: &GalileanMorphism) -> Frame {
    let origin = morphism.transform(&frame.origin);
    let basis = frame.basis.clone(); // basis unchanged for Galilean
    Frame {
        origin,
        basis,
        parent: frame.parent.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::WorldLine;

    fn make_worldline() -> WorldLine {
        WorldLine {
            iri: "worldline:observer-1".into(),
            asserted_by: "did:qualia:root:alice".into(),
            created_at: 1000,
        }
    }

    fn make_pose(x: f64, y: f64) -> Pose {
        Pose {
            position: vec![x, y, 0.0],
            orientation: vec![1.0, 0.0, 0.0, 0.0],
            frame: None,
        }
    }

    // ── W2: WorldLine trajectory tests ────────────────────────────────

    #[test]
    fn w2_empty_trajectory() {
        let wl = WorldLineTrajectory::new(make_worldline());
        assert!(wl.is_empty());
        assert_eq!(wl.len(), 0);
        assert!(wl.span_nanos().is_none());
    }

    #[test]
    fn w2_add_waypoints_sorted() {
        let mut wl = WorldLineTrajectory::new(make_worldline());
        wl.add(Waypoint::new(2000, 0, make_pose(2.0, 0.0)));
        wl.add(Waypoint::new(1000, 0, make_pose(1.0, 0.0)));
        wl.add(Waypoint::new(3000, 0, make_pose(3.0, 0.0)));
        assert_eq!(wl.len(), 3);
        assert_eq!(wl.waypoints[0].t_secs, 1000);
        assert_eq!(wl.waypoints[1].t_secs, 2000);
        assert_eq!(wl.waypoints[2].t_secs, 3000);
    }

    #[test]
    fn w2_pose_at_exact_waypoint() {
        let mut wl = WorldLineTrajectory::new(make_worldline());
        wl.add(Waypoint::new(1000, 0, make_pose(1.0, 0.0)));
        wl.add(Waypoint::new(2000, 0, make_pose(2.0, 0.0)));
        let pose = wl.pose_at(1000, 0).unwrap();
        assert_eq!(pose.position, vec![1.0, 0.0, 0.0]);
    }

    #[test]
    fn w2_pose_at_interpolated() {
        let mut wl = WorldLineTrajectory::new(make_worldline());
        wl.add(Waypoint::new(1000, 0, make_pose(0.0, 0.0)));
        wl.add(Waypoint::new(2000, 0, make_pose(10.0, 0.0)));
        // Halfway between t=1000 and t=2000 → x=5.0
        let pose = wl.pose_at(1500, 0).unwrap();
        assert!((pose.position[0] - 5.0).abs() < 1e-9);
    }

    #[test]
    fn w2_pose_at_before_first() {
        let mut wl = WorldLineTrajectory::new(make_worldline());
        wl.add(Waypoint::new(1000, 0, make_pose(5.0, 0.0)));
        let pose = wl.pose_at(500, 0).unwrap();
        assert_eq!(pose.position, vec![5.0, 0.0, 0.0]);
    }

    #[test]
    fn w2_pose_at_after_last() {
        let mut wl = WorldLineTrajectory::new(make_worldline());
        wl.add(Waypoint::new(1000, 0, make_pose(5.0, 0.0)));
        let pose = wl.pose_at(2000, 0).unwrap();
        assert_eq!(pose.position, vec![5.0, 0.0, 0.0]);
    }

    #[test]
    fn w2_span_nanos() {
        let mut wl = WorldLineTrajectory::new(make_worldline());
        wl.add(Waypoint::new(1000, 0, make_pose(0.0, 0.0)));
        wl.add(Waypoint::new(2000, 500_000_000, make_pose(10.0, 0.0)));
        let span = wl.span_nanos().unwrap();
        // 2000s + 500ms - 1000s = 1000.5 seconds = 1_000_500_000_000 nanos
        assert_eq!(span, 1_000_500_000_000);
    }

    #[test]
    fn w2_empty_pose_at_returns_none() {
        let wl = WorldLineTrajectory::new(make_worldline());
        assert!(wl.pose_at(1000, 0).is_none());
    }

    // ── W5: Frame morphism tests ──────────────────────────────────────

    #[test]
    fn w5_galilean_transform() {
        let m = GalileanMorphism::new(vec![1.0, 0.0, 0.0]); // v=1 m/s along x
        let result = m.transform(&[10.0, 0.0, 0.0, 5.0]); // x=10, t=5
        assert!((result[0] - 5.0).abs() < 1e-9); // x' = 10 - 1*5 = 5
        assert!((result[3] - 5.0).abs() < 1e-9); // t' = t = 5
    }

    #[test]
    fn w5_galilean_time_unchanged() {
        let m = GalileanMorphism::new(vec![2.0, 3.0, 0.0]);
        let result = m.transform(&[0.0, 0.0, 0.0, 10.0]);
        assert!((result[3] - 10.0).abs() < 1e-9); // t' = t
    }

    #[test]
    fn w5_galilean_inverse() {
        let m = GalileanMorphism::new(vec![1.0, 0.0, 0.0]);
        let inv = m.inverse();
        assert_eq!(inv.velocity, vec![-1.0, 0.0, 0.0]);
    }

    #[test]
    fn w5_galilean_round_trip() {
        let m = GalileanMorphism::new(vec![1.5, 0.0, 0.0]);
        let original = vec![10.0, 0.0, 0.0, 4.0];
        let transformed = m.transform(&original);
        let restored = m.inverse().transform(&transformed);
        for i in 0..4 {
            assert!((restored[i] - original[i]).abs() < 1e-9);
        }
    }

    #[test]
    fn w5_lorentz_gamma_at_rest() {
        let m = LorentzMorphism::new(0.0);
        assert!((m.gamma() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn w5_lorentz_gamma_at_half_c() {
        let m = LorentzMorphism::with_c(5.0, 10.0); // v = 0.5c (c=10)
        let expected = 1.0 / (1.0_f64 - 0.25).sqrt();
        assert!((m.gamma() - expected).abs() < 1e-9);
    }

    #[test]
    fn w5_lorentz_transform_at_rest() {
        let m = LorentzMorphism::new(0.0);
        let result = m.transform(&[5.0, 10.0, 0.0, 0.0]);
        // v=0 → no change
        assert!((result[0] - 5.0).abs() < 1e-9);
        assert!((result[1] - 10.0).abs() < 1e-9);
    }

    #[test]
    fn w5_lorentz_transform_time_dilation() {
        let m = LorentzMorphism::with_c(10.0, 10.0); // v = c, but that's singular
                                                     // Use v = 0.6c instead
        let m = LorentzMorphism::with_c(6.0, 10.0);
        let g = m.gamma();
        // t' = gamma * (t - v*x/c²)
        // With t=10, x=0: t' = gamma * 10
        let result = m.transform(&[10.0, 0.0, 0.0, 0.0]);
        assert!((result[0] - g * 10.0).abs() < 1e-6);
    }

    #[test]
    fn w5_lorentz_transform_length_contraction() {
        let m = LorentzMorphism::with_c(6.0, 10.0); // v = 0.6c
        let g = m.gamma();
        // x' = gamma * (x - v*t)
        // With x=10, t=0: x' = gamma * 10
        let result = m.transform(&[0.0, 10.0, 0.0, 0.0]);
        assert!((result[1] - g * 10.0).abs() < 1e-6);
    }

    #[test]
    fn w5_lorentz_inverse() {
        let m = LorentzMorphism::with_c(6.0, 10.0);
        let inv = m.inverse();
        assert_eq!(inv.velocity, -6.0);
    }

    #[test]
    fn w5_lorentz_round_trip() {
        let m = LorentzMorphism::with_c(6.0, 10.0);
        let original = vec![5.0, 10.0, 3.0, 7.0];
        let transformed = m.transform(&original);
        let restored = m.inverse().transform(&transformed);
        for i in 0..4 {
            assert!((restored[i] - original[i]).abs() < 1e-6);
        }
    }

    #[test]
    fn w5_lorentz_y_z_unchanged() {
        let m = LorentzMorphism::with_c(6.0, 10.0);
        let result = m.transform(&[0.0, 0.0, 5.0, 7.0]);
        assert!((result[2] - 5.0).abs() < 1e-9); // y unchanged
        assert!((result[3] - 7.0).abs() < 1e-9); // z unchanged
    }

    #[test]
    fn w5_transform_frame_galilean() {
        let frame = Frame {
            origin: vec![10.0, 0.0, 0.0, 5.0],
            basis: vec![],
            parent: None,
        };
        let m = GalileanMorphism::new(vec![1.0, 0.0, 0.0]);
        let transformed = transform_frame(&frame, &m);
        assert!((transformed.origin[0] - 5.0).abs() < 1e-9); // 10 - 1*5 = 5
    }
}
