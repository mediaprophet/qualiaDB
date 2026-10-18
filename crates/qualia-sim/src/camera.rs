//! Camera Unprojection, Raycasting, & Smooth Pursuit Controllers.
//!
//! Provides zero-heap screen-to-world raycasting for pixel-perfect touch and pointer picking,
//! ground plane intersection without GPU readbacks, and 2nd-order spring-damper chase cameras.

/// A 3D ray with origin and normalized direction vector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray3D {
    pub origin: [f64; 3],
    pub direction: [f64; 3],
}

impl Ray3D {
    pub const fn new(origin: [f64; 3], direction: [f64; 3]) -> Self {
        Self { origin, direction }
    }

    /// Evaluates point along the ray at distance $t \ge 0$.
    #[inline]
    pub fn point_at(&self, t: f64) -> [f64; 3] {
        [
            self.origin[0] + self.direction[0] * t,
            self.origin[1] + self.direction[1] * t,
            self.origin[2] + self.direction[2] * t,
        ]
    }

    /// Intersects ray with horizontal ground plane at altitude $Z = \text{ground\_z}$.
    ///
    /// Returns 3D world coordinates if the ray points towards the plane ($t > 0$).
    pub fn intersect_ground_plane(&self, ground_z: f64) -> Option<[f64; 3]> {
        if self.direction[2].abs() < 1e-6 {
            return None; // Parallel to ground plane
        }

        let t = (ground_z - self.origin[2]) / self.direction[2];
        if t >= 0.0 {
            Some(self.point_at(t))
        } else {
            None // Ray points away from ground plane
        }
    }
}

/// Unprojects 2D screen coordinates into a 3D world ray using a $4 \times 4$ inverse view-projection matrix.
///
/// `inv_vp`: 16-element column-major matrix $[M_{col, row}]$.
pub fn unproject_screen_to_ray(
    screen_x: f64,
    screen_y: f64,
    viewport_width: f64,
    viewport_height: f64,
    inv_vp: &[f64; 16],
) -> Option<Ray3D> {
    if viewport_width <= 0.0 || viewport_height <= 0.0 {
        return None;
    }

    // Normalized Device Coordinates (NDC)
    // X: [-1.0, 1.0], Y: [1.0, -1.0] (WebGPU/Vulkan top-left screen origin)
    let ndc_x = (2.0 * screen_x) / viewport_width - 1.0;
    let ndc_y = 1.0 - (2.0 * screen_y) / viewport_height;

    // Transform near plane (Z = -1.0 in OpenGL, 0.0 in WebGPU/DirectX) and far plane (Z = 1.0)
    let near_pt = mat4_transform_vec4(inv_vp, [ndc_x, ndc_y, 0.0, 1.0])?;
    let far_pt = mat4_transform_vec4(inv_vp, [ndc_x, ndc_y, 1.0, 1.0])?;

    let dx = far_pt[0] - near_pt[0];
    let dy = far_pt[1] - near_pt[1];
    let dz = far_pt[2] - near_pt[2];
    let len = (dx * dx + dy * dy + dz * dz).sqrt();

    if len < 1e-7 {
        return None;
    }

    Some(Ray3D {
        origin: near_pt,
        direction: [dx / len, dy / len, dz / len],
    })
}

#[inline]
fn mat4_transform_vec4(m: &[f64; 16], v: [f64; 4]) -> Option<[f64; 3]> {
    // Column-major multiplication: out = M * v
    let x = m[0] * v[0] + m[4] * v[1] + m[8] * v[2] + m[12] * v[3];
    let y = m[1] * v[0] + m[5] * v[1] + m[9] * v[2] + m[13] * v[3];
    let z = m[2] * v[0] + m[6] * v[1] + m[10] * v[2] + m[14] * v[3];
    let w = m[3] * v[0] + m[7] * v[1] + m[11] * v[2] + m[15] * v[3];

    if w.abs() < 1e-12 {
        None
    } else {
        Some([x / w, y / w, z / w])
    }
}

/// 2nd-order spring-damper critically damped pursuit camera.
///
/// Prevents jitter and overshoot while dynamically tracking agile rovers or avatars.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringChaseCamera {
    pub current_pos: [f64; 3],
    pub velocity: [f64; 3],
    /// Target follow distance in meters
    pub distance_m: f64,
    /// Elevation height offset above target in meters
    pub height_offset_m: f64,
    /// Spring stiffness parameter (higher = faster response)
    pub stiffness: f64,
    /// Damping ratio (1.0 = critically damped, >1.0 = overdamped)
    pub damping: f64,
    /// Velocity look-ahead factor
    pub lookahead_factor: f64,
}

impl Default for SpringChaseCamera {
    fn default() -> Self {
        Self {
            current_pos: [0.0, -10.0, 5.0],
            velocity: [0.0, 0.0, 0.0],
            distance_m: 12.0,
            height_offset_m: 6.0,
            stiffness: 15.0,
            damping: 1.0,
            lookahead_factor: 0.5,
        }
    }
}

impl SpringChaseCamera {
    /// Advances camera position towards target with spring-damper physics over delta time `dt`.
    pub fn update(
        &mut self,
        target_pos: [f64; 3],
        _target_velocity: [f64; 3],
        target_yaw_rad: f64,
        dt: f64,
    ) {
        // Ideal camera position behind the target based on target yaw
        let backward_x = -target_yaw_rad.sin();
        let backward_y = -target_yaw_rad.cos();

        let ideal_x = target_pos[0] + backward_x * self.distance_m;
        let ideal_y = target_pos[1] + backward_y * self.distance_m;
        let ideal_z = target_pos[2] + self.height_offset_m;

        let ideal_pos = [ideal_x, ideal_y, ideal_z];

        // Spring acceleration: a = stiffness * (target - current) - 2 * damping * sqrt(stiffness) * v
        let omega = self.stiffness.sqrt();
        let c = 2.0 * self.damping * omega;

        for i in 0..3 {
            let displacement = ideal_pos[i] - self.current_pos[i];
            let spring_force = self.stiffness * displacement;
            let damping_force = c * self.velocity[i];
            let accel = spring_force - damping_force;

            self.velocity[i] += accel * dt;
            self.current_pos[i] += self.velocity[i] * dt;
        }
    }

    /// Computes the look-at target gaze coordinate, incorporating velocity look-ahead.
    pub fn look_at_target(&self, target_pos: [f64; 3], target_velocity: [f64; 3]) -> [f64; 3] {
        [
            target_pos[0] + target_velocity[0] * self.lookahead_factor,
            target_pos[1] + target_velocity[1] * self.lookahead_factor,
            target_pos[2] + 1.0, // Focus slightly above target base
        ]
    }
}

/// Orbit inspection camera around a central pivot point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitCamera {
    pub target: [f64; 3],
    pub distance: f64,
    pub yaw_rad: f64,
    pub pitch_rad: f64,
    pub min_distance: f64,
    pub max_distance: f64,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            target: [0.0, 0.0, 0.0],
            distance: 20.0,
            yaw_rad: 0.0,
            pitch_rad: 0.5, // ~28.6 degrees
            min_distance: 2.0,
            max_distance: 100.0,
        }
    }
}

impl OrbitCamera {
    /// Computes eye position in 3D world space.
    pub fn eye_position(&self) -> [f64; 3] {
        let cos_pitch = self.pitch_rad.cos();
        let sin_pitch = self.pitch_rad.sin();
        let sin_yaw = self.yaw_rad.sin();
        let cos_yaw = self.yaw_rad.cos();

        [
            self.target[0] + self.distance * cos_pitch * sin_yaw,
            self.target[1] - self.distance * cos_pitch * cos_yaw,
            self.target[2] + self.distance * sin_pitch,
        ]
    }

    /// Applies angular delta rotation (yaw and pitch in radians).
    pub fn rotate(&mut self, delta_yaw: f64, delta_pitch: f64) {
        const MAX_PITCH: f64 = 85.0 * std::f64::consts::PI / 180.0;
        const MIN_PITCH: f64 = -10.0 * std::f64::consts::PI / 180.0;

        self.yaw_rad += delta_yaw;
        self.pitch_rad = (self.pitch_rad + delta_pitch).clamp(MIN_PITCH, MAX_PITCH);
    }

    /// Applies zoom delta in distance.
    pub fn zoom(&mut self, delta_dist: f64) {
        self.distance = (self.distance + delta_dist).clamp(self.min_distance, self.max_distance);
    }
}

/// 6-DOF Free-flight camera with velocity damping for editor, debug, and free inspection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlyCamera {
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub yaw_rad: f64,
    pub pitch_rad: f64,
    pub move_speed: f64,
    pub drag: f64,
}

impl Default for FlyCamera {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 10.0],
            velocity: [0.0, 0.0, 0.0],
            yaw_rad: 0.0,
            pitch_rad: 0.0,
            move_speed: 15.0,
            drag: 8.0,
        }
    }
}

impl FlyCamera {
    /// Computes unit forward, right, and up vectors from current orientation.
    pub fn orientation_basis(&self) -> ([f64; 3], [f64; 3], [f64; 3]) {
        let cp = self.pitch_rad.cos();
        let sp = self.pitch_rad.sin();
        let cy = self.yaw_rad.cos();
        let sy = self.yaw_rad.sin();

        let forward = [sy * cp, -cy * cp, sp];
        let right = [cy, sy, 0.0];
        let up = [-sy * sp, cy * sp, cp];

        (forward, right, up)
    }

    /// Advances flycam position with smooth drag.
    ///
    /// `wish_dir`: Local input direction `[forward/back, right/left, up/down]`.
    pub fn update(&mut self, wish_dir: [f64; 3], dt: f64) {
        let (fwd, rgt, up) = self.orientation_basis();
        let accel = [
            fwd[0] * wish_dir[0] + rgt[0] * wish_dir[1] + up[0] * wish_dir[2],
            fwd[1] * wish_dir[0] + rgt[1] * wish_dir[1] + up[1] * wish_dir[2],
            fwd[2] * wish_dir[0] + rgt[2] * wish_dir[1] + up[2] * wish_dir[2],
        ];

        let target_v = [
            accel[0] * self.move_speed,
            accel[1] * self.move_speed,
            accel[2] * self.move_speed,
        ];

        let lerp_factor = (self.drag * dt).min(1.0);
        for i in 0..3 {
            self.velocity[i] += (target_v[i] - self.velocity[i]) * lerp_factor;
            self.position[i] += self.velocity[i] * dt;
        }
    }

    /// Rotates orientation by yaw and pitch deltas.
    pub fn rotate(&mut self, delta_yaw: f64, delta_pitch: f64) {
        const MAX_PITCH: f64 = 89.0 * std::f64::consts::PI / 180.0;
        self.yaw_rad += delta_yaw;
        self.pitch_rad = (self.pitch_rad + delta_pitch).clamp(-MAX_PITCH, MAX_PITCH);
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ray_ground_intersection() {
        let ray = Ray3D::new([0.0, 0.0, 10.0], [0.0, 0.0, -1.0]);
        let hit = ray.intersect_ground_plane(0.0).expect("hits ground");
        assert_eq!(hit, [0.0, 0.0, 0.0]);

        // Pointing away
        let ray_up = Ray3D::new([0.0, 0.0, 10.0], [0.0, 0.0, 1.0]);
        assert!(ray_up.intersect_ground_plane(0.0).is_none());
    }

    #[test]
    fn test_orbit_camera_eye_pos() {
        let orbit = OrbitCamera {
            target: [0.0, 0.0, 0.0],
            distance: 10.0,
            yaw_rad: 0.0,
            pitch_rad: 0.0,
            min_distance: 1.0,
            max_distance: 50.0,
        };
        let eye = orbit.eye_position();
        assert!((eye[0] - 0.0).abs() < 1e-4);
        assert!((eye[1] - (-10.0)).abs() < 1e-4);
        assert!((eye[2] - 0.0).abs() < 1e-4);
    }

    #[test]
    fn test_spring_chase_camera_damping() {
        let mut cam = SpringChaseCamera::default();
        let target = [10.0, 0.0, 0.0];
        let vel = [0.0, 0.0, 0.0];

        // Step 100 times at 64 Hz
        for _ in 0..100 {
            cam.update(target, vel, 0.0, 1.0 / 64.0);
        }

        // Camera should have converged near the ideal follow position
        assert!((cam.current_pos[0] - 10.0).abs() < 1.0);
        assert!(cam.velocity[0].abs() < 1.0);
    }
}
