/// Physical anchor tying a virtual coordinate to a physical spatial location
/// Compliant with the Spatial Web Anchoring Spec (UWB/VPS anchoring).
pub struct PhysicalAnchor {
    pub virtual_position: (f64, f64, f64), // XYZ in scene coordinates
    pub physical_id: String,               // UWB beacon ID or VPS feature hash
    pub confidence: f64,                   // 0.0 to 1.0 confidence in tracking
    pub is_active: bool,
}

impl PhysicalAnchor {
    pub fn new(vx: f64, vy: f64, vz: f64, id: String) -> Self {
        Self {
            virtual_position: (vx, vy, vz),
            physical_id: id,
            confidence: 1.0,
            is_active: true,
        }
    }
}

/// Represents the state of the AR device (camera tracking, passthrough mode)
pub struct ArDeviceState {
    pub camera_pose: (f64, f64, f64, f64, f64, f64), // (x,y,z, yaw,pitch,roll)
    pub passthrough_enabled: bool,
    pub tracking_confidence: f64, // 0.0 = lost, 1.0 = perfect
    pub anchors: Vec<PhysicalAnchor>,
}

impl ArDeviceState {
    pub fn new() -> Self {
        Self {
            camera_pose: (0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            passthrough_enabled: false,
            tracking_confidence: 0.0,
            anchors: Vec::new(),
        }
    }

    pub fn update_pose(&mut self, pose: (f64, f64, f64, f64, f64, f64), confidence: f64) {
        self.camera_pose = pose;
        self.tracking_confidence = confidence;
    }

    pub fn add_anchor(&mut self, anchor: PhysicalAnchor) {
        self.anchors.push(anchor);
    }
}
