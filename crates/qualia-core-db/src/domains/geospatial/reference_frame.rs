use serde::{Deserialize, Serialize};

/// Represents a local coordinate system with a floating origin and scale factor.
/// Used to bridge the micro-verse (e.g. molecular structures) with the macro-verse (e.g. planetary scale).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReferenceFrame {
    /// Unique identifier (e.g. q_hash of the frame URI).
    pub id: u64,
    /// Parent frame ID. If None, this is a top-level global frame.
    pub parent_id: Option<u64>,
    /// Translation relative to the parent frame [x, y, z].
    pub translation: [f64; 3],
    /// Rotation quaternion [w, x, y, z] relative to the parent frame.
    pub rotation: [f64; 4],
    /// Scale multiplier applied to everything within this frame.
    pub scale: f64,
}

impl Default for ReferenceFrame {
    fn default() -> Self {
        Self {
            id: 0,
            parent_id: None,
            translation: [0.0, 0.0, 0.0],
            rotation: [1.0, 0.0, 0.0, 0.0], // Identity quaternion
            scale: 1.0,
        }
    }
}

impl ReferenceFrame {
    pub fn new(id: u64, parent_id: Option<u64>) -> Self {
        Self {
            id,
            parent_id,
            ..Default::default()
        }
    }

    /// Applies this frame's transform to a local point, mapping it into the parent's coordinate space.
    pub fn transform_to_parent(&self, local_point: [f64; 3]) -> [f64; 3] {
        let [x, y, z] = local_point;

        // 1. Scale
        let sx = x * self.scale;
        let sy = y * self.scale;
        let sz = z * self.scale;

        // 2. Rotate (using quaternion: q * p * q^-1)
        let [qw, qx, qy, qz] = self.rotation;

        let ix = qw * sx + qy * sz - qz * sy;
        let iy = qw * sy + qz * sx - qx * sz;
        let iz = qw * sz + qx * sy - qy * sx;
        let iw = -qx * sx - qy * sy - qz * sz;

        let rx = ix * qw + iw * -qx + iy * -qz - iz * -qy;
        let ry = iy * qw + iw * -qy + iz * -qx - ix * -qz;
        let rz = iz * qw + iw * -qz + ix * -qy - iy * -qx;

        // 3. Translate
        [
            rx + self.translation[0],
            ry + self.translation[1],
            rz + self.translation[2],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_to_parent() {
        let mut frame = ReferenceFrame::new(1, None);
        frame.translation = [10.0, 20.0, 30.0];
        frame.scale = 0.5;
        // 90 degrees around Z-axis
        let half_angle = std::f64::consts::PI / 4.0;
        frame.rotation = [half_angle.cos(), 0.0, 0.0, half_angle.sin()];

        let local_pt = [2.0, 0.0, 0.0];
        let transformed = frame.transform_to_parent(local_pt);

        // Scale -> [1.0, 0.0, 0.0]
        // Rotate 90 deg around Z -> [0.0, 1.0, 0.0]
        // Translate -> [10.0, 21.0, 30.0]

        assert!((transformed[0] - 10.0).abs() < 1e-8);
        assert!((transformed[1] - 21.0).abs() < 1e-8);
        assert!((transformed[2] - 30.0).abs() < 1e-8);
    }
}
