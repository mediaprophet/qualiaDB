//! WorldLine trajectory & temporal persistence (W2).

use crate::value::{Pose, WorldLine};

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
