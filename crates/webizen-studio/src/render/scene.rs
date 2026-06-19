//! Geometric primitives for manifold projection: vector math, projection viewpoint, projected points.
//!
//! These compile on every target (no `web-sys`), so the same projection viewpoint logic
//! is shared across different backends for geometric examination of the 10D epistemic manifold.
//!
//! NOTE: This is NOT traditional 3D scene rendering - these are geometric operations
//! for examining relationships in the high-dimensional epistemic manifold.

/// Minimal 3D vector for geometric calculations in manifold projection.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }

    pub fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }

    pub fn scale(self, k: f64) -> Self {
        Self::new(self.x * k, self.y * k, self.z * k)
    }

    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub fn length(self) -> f64 {
        self.dot(self).sqrt()
    }

    pub fn normalize(self) -> Self {
        let len = self.length();
        if len <= f64::EPSILON {
            Self::default()
        } else {
            Self::new(self.x / len, self.y / len, self.z / len)
        }
    }
}

/// A manifold point after projection into screen space, with retained depth so
/// callers can fade/scale by distance for geometric examination of epistemic relationships.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScreenPoint {
    pub x: f64,
    pub y: f64,
    pub depth: f64,
}

/// A projection viewpoint for examining the 10D epistemic manifold.
/// `focal_scale` multiplies `min(viewport)` to set
/// the effective focal length, matching the original physics-surface tuning.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub world_up: Vec3,
    pub focal_scale: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            eye: Vec3::new(0.0, 8.0, 18.0),
            target: Vec3::new(0.0, 0.5, 0.0),
            world_up: Vec3::new(0.0, 1.0, 0.0),
            focal_scale: 0.88,
        }
    }
}

impl Camera {
    /// Project a manifold point to screen space for geometric examination.
    /// Returns `None` for points at or behind the near plane.
    /// This is NOT traditional 3D rendering - it's geometric projection for examining
    /// relationships in the 10D epistemic manifold.
    pub fn project(&self, world: Vec3, viewport: (f64, f64)) -> Option<ScreenPoint> {
        let forward = self.target.sub(self.eye).normalize();
        let right = forward.cross(self.world_up).normalize();
        let up = right.cross(forward).normalize();
        let relative = world.sub(self.eye);

        let view_x = relative.dot(right);
        let view_y = relative.dot(up);
        let view_z = relative.dot(forward);

        if view_z <= 0.2 {
            return None;
        }

        let focal = viewport.0.min(viewport.1) * self.focal_scale;
        Some(ScreenPoint {
            x: viewport.0 * 0.5 + (view_x / view_z) * focal,
            y: viewport.1 * 0.52 - (view_y / view_z) * focal,
            depth: view_z,
        })
    }

    /// Rotate projection viewpoint around target using yaw (horizontal) and pitch (vertical) angles.
    /// This adjusts the geometric examination viewpoint of the 10D epistemic manifold.
    /// All calculations use stack-allocated f64 values for zero-heap compliance.
    pub fn orbit(&mut self, yaw: f64, pitch: f64) {
        // Calculate vector from target to eye (camera radius)
        let dx = self.eye.x - self.target.x;
        let dy = self.eye.y - self.target.y;
        let dz = self.eye.z - self.target.z;

        // Current spherical coordinates
        let radius = (dx * dx + dy * dy + dz * dz).sqrt();
        let current_yaw = dz.atan2(dx);
        let current_pitch = (dy / radius).asin();

        // Apply rotation with pitch clamping to avoid gimbal lock
        let new_yaw = current_yaw + yaw;
        let new_pitch = (current_pitch + pitch).clamp(
            -std::f64::consts::FRAC_PI_2 + 0.01,
            std::f64::consts::FRAC_PI_2 - 0.01,
        );

        // Convert back to Cartesian coordinates (stack-allocated)
        let cos_pitch = new_pitch.cos();
        let sin_pitch = new_pitch.sin();
        let cos_yaw = new_yaw.cos();
        let sin_yaw = new_yaw.sin();

        // Update camera position
        self.eye.x = self.target.x + radius * cos_pitch * cos_yaw;
        self.eye.y = self.target.y + radius * sin_pitch;
        self.eye.z = self.target.z + radius * cos_pitch * sin_yaw;
    }

    /// Zoom projection viewpoint in/out by adjusting distance to target.
    /// This adjusts the geometric examination scale of the 10D epistemic manifold.
    /// Uses stack-allocated f64 values for zero-heap compliance.
    pub fn zoom(&mut self, delta: f64) {
        // Calculate current distance to target
        let dx = self.eye.x - self.target.x;
        let dy = self.eye.y - self.target.y;
        let dz = self.eye.z - self.target.z;
        let current_distance = (dx * dx + dy * dy + dz * dz).sqrt();

        // Apply zoom with minimum distance clamp
        let zoom_factor = (-delta * 0.1).exp(); // Smooth exponential zoom
        let new_distance = (current_distance * zoom_factor).max(0.5);

        // Scale the position vector
        let scale = new_distance / current_distance;
        self.eye.x = self.target.x + dx * scale;
        self.eye.y = self.target.y + dy * scale;
        self.eye.z = self.target.z + dz * scale;
    }

    /// Pan projection viewpoint in screen space (dx, dy are screen-relative deltas).
    /// This adjusts the geometric examination position in the 10D epistemic manifold.
    /// Uses stack-allocated f64 values for zero-heap compliance.
    pub fn pan(&mut self, dx: f64, dy: f64) {
        // Calculate camera forward and right vectors
        let forward = self.target.sub(self.eye).normalize();
        let right = forward.cross(self.world_up).normalize();
        let up = right.cross(forward).normalize();

        // Scale pan by distance to target for consistent feel
        let distance = self.eye.sub(self.target).length();
        let pan_scale = distance * 0.001;

        // Apply pan to both position and target
        let pan = right.scale(dx * pan_scale).add(up.scale(dy * pan_scale));
        self.eye = self.eye.add(pan);
        self.target = self.target.add(pan);
    }
}
