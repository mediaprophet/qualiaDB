//! Augmented Reality spatial anchors and VIO frame hierarchy (OCS §5).
//!
//! Reference: OCS Specification v2.2.0 §5.

use crate::cosmic::transforms::Geodetic;
use crate::value::Value;
use std::collections::BTreeMap;

/// AR frame hierarchy level (OCS §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArFrameLevel {
    /// L_6.0 — Local geodetic anchor (GPS/RTK/VPS).
    L60GeodeticAnchor,
    /// L_6.1 — VIO/SLAM metric world root.
    L61VioWorldRoot,
    /// L_6.2 — Head/device pose frame (6-DOF, 1000 Hz).
    L62HeadPose,
    /// L_6.3 — Eye & display frustum frame.
    L63EyeFrustum,
}

impl ArFrameLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::L60GeodeticAnchor => "L_6.0",
            Self::L61VioWorldRoot => "L_6.1",
            Self::L62HeadPose => "L_6.2",
            Self::L63EyeFrustum => "L_6.3",
        }
    }
}

/// A persistent spatial anchor — world lock (OCS §5.2).
#[derive(Debug, Clone, PartialEq)]
pub struct SpatialAnchor {
    pub anchor_id: String,
    pub parent_frame: String,
    pub geodetic_anchor: [f64; 3],  // [lat, lon, alt]
    pub enu_offset: [f32; 3],       // Local [East, North, Up] in meters
    pub orientation_quat: [f32; 4], // [w, x, y, z]
    pub confidence_radius_mm: f32,
    pub vps_mesh_signature: Vec<u8>,
}

impl SpatialAnchor {
    /// Create a new spatial anchor at a geodetic position.
    pub fn new(anchor_id: &str, geodetic: [f64; 3]) -> Self {
        Self {
            anchor_id: anchor_id.into(),
            parent_frame: "urn:omni:v1:physical:observable:standard:earth:wgs84".into(),
            geodetic_anchor: geodetic,
            enu_offset: [0.0, 0.0, 0.0],
            orientation_quat: [1.0, 0.0, 0.0, 0.0], // Identity quaternion
            confidence_radius_mm: 10.0,
            vps_mesh_signature: Vec::new(),
        }
    }

    /// Set ENU offset from the geodetic anchor.
    pub fn with_enu_offset(mut self, east: f32, north: f32, up: f32) -> Self {
        self.enu_offset = [east, north, up];
        self
    }

    /// Set orientation quaternion [w, x, y, z].
    pub fn with_orientation(mut self, w: f32, x: f32, y: f32, z: f32) -> Self {
        self.orientation_quat = [w, x, y, z];
        self
    }

    /// Set confidence radius in mm.
    pub fn with_confidence(mut self, radius_mm: f32) -> Self {
        self.confidence_radius_mm = radius_mm;
        self
    }

    /// Get the geodetic position of this anchor.
    pub fn geodetic(&self) -> Geodetic {
        Geodetic {
            lat_deg: self.geodetic_anchor[0],
            lon_deg: self.geodetic_anchor[1],
            alt_m: self.geodetic_anchor[2],
        }
    }

    /// Check if this anchor is sub-millimeter precision (OCS-T05).
    pub fn is_submillimeter(&self) -> bool {
        self.confidence_radius_mm < 1.0
    }

    /// Verify the quaternion is normalized.
    pub fn is_quaternion_valid(&self) -> bool {
        let [w, x, y, z] = self.orientation_quat;
        let norm = (w * w + x * x + y * y + z * z).sqrt();
        (norm - 1.0).abs() < 1e-5
    }

    pub fn to_value(&self) -> Value {
        let mut rec = BTreeMap::new();
        rec.insert("anchor_id".into(), Value::String(self.anchor_id.clone()));
        rec.insert(
            "parent_frame".into(),
            Value::String(self.parent_frame.clone()),
        );
        rec.insert(
            "geodetic_anchor".into(),
            Value::List(
                self.geodetic_anchor
                    .iter()
                    .map(|v| Value::F64(*v))
                    .collect(),
            ),
        );
        rec.insert(
            "enu_offset".into(),
            Value::List(
                self.enu_offset
                    .iter()
                    .map(|v| Value::F64(*v as f64))
                    .collect(),
            ),
        );
        rec.insert(
            "orientation_quat".into(),
            Value::List(
                self.orientation_quat
                    .iter()
                    .map(|v| Value::F64(*v as f64))
                    .collect(),
            ),
        );
        rec.insert(
            "confidence_radius_mm".into(),
            Value::F64(self.confidence_radius_mm as f64),
        );
        Value::Record(rec)
    }
}

/// Head pose — 6-DOF tracking transform (OCS §5.1, L_6.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeadPose {
    /// Position [x, y, z] in VIO world frame (meters)
    pub position: [f32; 3],
    /// Orientation quaternion [w, x, y, z]
    pub orientation: [f32; 4],
    /// Timestamp (monotonic nanos)
    pub timestamp_ns: u64,
}

impl HeadPose {
    pub fn identity() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            orientation: [1.0, 0.0, 0.0, 0.0],
            timestamp_ns: 0,
        }
    }

    /// Apply a translation to the head pose.
    pub fn translated(&self, dx: f32, dy: f32, dz: f32) -> Self {
        Self {
            position: [
                self.position[0] + dx,
                self.position[1] + dy,
                self.position[2] + dz,
            ],
            orientation: self.orientation,
            timestamp_ns: self.timestamp_ns,
        }
    }
}

/// VIO/SLAM metric world root (OCS §5.1, L_6.1).
#[derive(Debug, Clone)]
pub struct VioWorldRoot {
    /// The geodetic anchor where VIO was initialized
    pub init_anchor: SpatialAnchor,
    /// Head pose history (most recent first)
    pub pose_history: Vec<HeadPose>,
    /// Whether the world root is stable (has converged)
    pub is_stable: bool,
}

impl VioWorldRoot {
    pub fn new(anchor: SpatialAnchor) -> Self {
        Self {
            init_anchor: anchor,
            pose_history: Vec::new(),
            is_stable: false,
        }
    }

    /// Push a new head pose into the history.
    pub fn update_pose(&mut self, pose: HeadPose) {
        self.pose_history.push(pose);
        // Mark as stable after 30 poses (~30ms at 1000Hz)
        if self.pose_history.len() >= 30 {
            self.is_stable = true;
        }
    }

    /// Get the current (most recent) head pose.
    pub fn current_pose(&self) -> Option<&HeadPose> {
        self.pose_history.last()
    }

    /// Check anchor stability under camera translation (OCS-T05).
    /// Returns true if the anchor maintains sub-millimeter stability.
    pub fn check_stability(&self) -> bool {
        if self.pose_history.len() < 2 {
            return false;
        }
        // Check that the init anchor has sub-mm confidence
        if !self.init_anchor.is_submillimeter() {
            return false;
        }
        // Check that recent poses haven't jumped dramatically
        let recent = &self.pose_history;
        if recent.len() < 2 {
            return false;
        }
        let last = recent[recent.len() - 1];
        let prev = recent[recent.len() - 2];
        let dx = last.position[0] - prev.position[0];
        let dy = last.position[1] - prev.position[1];
        let dz = last.position[2] - prev.position[2];
        let drift = (dx * dx + dy * dy + dz * dz).sqrt();
        // Drift between frames should be < 0.5mm (OCS-T05)
        drift < 0.0005
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spatial_anchor_default() {
        let a = SpatialAnchor::new("anchor-1", [37.8080, -122.4177, 10.0]);
        assert_eq!(a.anchor_id, "anchor-1");
        assert_eq!(a.enu_offset, [0.0, 0.0, 0.0]);
        assert!(a.is_quaternion_valid()); // Identity quaternion
    }

    #[test]
    fn spatial_anchor_with_offset() {
        let a = SpatialAnchor::new("anchor-1", [37.8, -122.4, 10.0]).with_enu_offset(5.0, 3.0, 1.0);
        assert_eq!(a.enu_offset, [5.0, 3.0, 1.0]);
    }

    #[test]
    fn spatial_anchor_submillimeter() {
        let a = SpatialAnchor::new("precise", [37.8, -122.4, 10.0]).with_confidence(0.5);
        assert!(a.is_submillimeter());
    }

    #[test]
    fn spatial_anchor_not_submillimeter() {
        let a = SpatialAnchor::new("rough", [37.8, -122.4, 10.0]).with_confidence(50.0);
        assert!(!a.is_submillimeter());
    }

    #[test]
    fn spatial_anchor_geodetic() {
        let a = SpatialAnchor::new("a", [37.8, -122.4, 10.0]);
        let g = a.geodetic();
        assert_eq!(g.lat_deg, 37.8);
        assert_eq!(g.lon_deg, -122.4);
    }

    #[test]
    fn spatial_anchor_to_value() {
        let a = SpatialAnchor::new("test", [37.8, -122.4, 10.0]);
        let v = a.to_value();
        match v {
            Value::Record(r) => {
                assert_eq!(r.get("anchor_id"), Some(&Value::String("test".into())));
                assert!(r.contains_key("geodetic_anchor"));
            }
            _ => panic!("expected Record"),
        }
    }

    #[test]
    fn head_pose_identity() {
        let p = HeadPose::identity();
        assert_eq!(p.position, [0.0, 0.0, 0.0]);
        assert_eq!(p.orientation, [1.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn head_pose_translated() {
        let p = HeadPose::identity();
        let p2 = p.translated(1.0, 2.0, 3.0);
        assert_eq!(p2.position, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn vio_world_root_stability() {
        let anchor = SpatialAnchor::new("root", [37.8, -122.4, 10.0]).with_confidence(0.3);
        let mut root = VioWorldRoot::new(anchor);
        // Add 30 stable poses
        for i in 0..30 {
            root.update_pose(HeadPose {
                position: [i as f32 * 0.0001, 0.0, 0.0], // Very small movements
                orientation: [1.0, 0.0, 0.0, 0.0],
                timestamp_ns: i as u64 * 1_000_000,
            });
        }
        assert!(root.is_stable);
        assert!(root.check_stability());
    }

    #[test]
    fn vio_world_root_unstable_with_drift() {
        let anchor = SpatialAnchor::new("root", [37.8, -122.4, 10.0]).with_confidence(0.3);
        let mut root = VioWorldRoot::new(anchor);
        root.update_pose(HeadPose::identity());
        // Large drift between frames
        root.update_pose(HeadPose {
            position: [10.0, 0.0, 0.0], // 10 meter jump!
            orientation: [1.0, 0.0, 0.0, 0.0],
            timestamp_ns: 1_000_000,
        });
        assert!(!root.check_stability());
    }

    #[test]
    fn ar_frame_level_names() {
        assert_eq!(ArFrameLevel::L60GeodeticAnchor.as_str(), "L_6.0");
        assert_eq!(ArFrameLevel::L63EyeFrustum.as_str(), "L_6.3");
    }

    #[test]
    fn quaternion_validity_check() {
        let mut a = SpatialAnchor::new("a", [0.0; 3]);
        a.orientation_quat = [2.0, 0.0, 0.0, 0.0]; // Not normalized
        assert!(!a.is_quaternion_valid());
    }
}
