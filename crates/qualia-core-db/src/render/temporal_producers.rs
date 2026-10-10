//! Reference math and host producer models for temporal accumulation.
//!
//! Temporal reconstruction requires two host-derived producer streams that
//! cannot be inferred from a single-frame colour or depth attachment:
//!
//! 1. **Motion Vectors:** Per-pixel or per-vertex 2D vectors in top-left-origin UV
//!    space (`[du, dv]`) pointing from the current frame pixel coordinate to its
//!    corresponding position in the previous frame.
//! 2. **Reactive Mask:** A scalar `[0.0, 1.0]` map indicating how strongly the
//!    history sample should be rejected/attenuated (e.g. for alpha-tested fringes,
//!    water reflections, translucent smoke/particles, and fast-moving decals).

/// A 2D motion vector expressed as a UV-space delta: `[du, dv]`.
///
/// Values point from current frame UV to previous frame UV:
/// `motion = previous_uv - current_uv`.
/// Positive values point towards increasing UV coordinates (rightwards and downwards
/// in top-left origin coordinates).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MotionVector2d {
    pub du: f32,
    pub dv: f32,
}

impl MotionVector2d {
    pub const ZERO: Self = Self { du: 0.0, dv: 0.0 };

    pub const fn new(du: f32, dv: f32) -> Self {
        Self { du, dv }
    }

    /// Magnitude squared of the motion vector in UV coordinates.
    pub fn length_squared(self) -> f32 {
        self.du * self.du + self.dv * self.dv
    }

    /// Euclidean length of the motion vector in UV coordinates.
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Whether the motion vector represents negligible or zero motion under a small epsilon.
    pub fn is_static(self, epsilon: f32) -> bool {
        self.length_squared() <= epsilon.max(0.0) * epsilon.max(0.0)
    }
}

/// A 4x4 column-major floating-point matrix for camera projection and transforms.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Mat4ColumnMajor {
    pub cols: [[f32; 4]; 4],
}

impl Mat4ColumnMajor {
    pub const IDENTITY: Self = Self {
        cols: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    };

    pub const fn from_cols(cols: [[f32; 4]; 4]) -> Self {
        Self { cols }
    }

    pub const fn from_f32_16(raw: [f32; 16]) -> Self {
        Self {
            cols: [
                [raw[0], raw[1], raw[2], raw[3]],
                [raw[4], raw[5], raw[6], raw[7]],
                [raw[8], raw[9], raw[10], raw[11]],
                [raw[12], raw[13], raw[14], raw[15]],
            ],
        }
    }

    /// Transform a 3D point `[x, y, z]` into homogeneous clip space `[x, y, z, w]`.
    pub fn transform_point3(&self, point: [f32; 3]) -> [f32; 4] {
        let x = point[0];
        let y = point[1];
        let z = point[2];
        [
            self.cols[0][0] * x + self.cols[1][0] * y + self.cols[2][0] * z + self.cols[3][0],
            self.cols[0][1] * x + self.cols[1][1] * y + self.cols[2][1] * z + self.cols[3][1],
            self.cols[0][2] * x + self.cols[1][2] * y + self.cols[2][2] * z + self.cols[3][2],
            self.cols[0][3] * x + self.cols[1][3] * y + self.cols[2][3] * z + self.cols[3][3],
        ]
    }
    /// Multiply two 4x4 column-major matrices: `self * rhs`.
    pub fn mul(&self, rhs: &Self) -> Self {
        let mut out = [[0.0_f32; 4]; 4];
        for col in 0..4 {
            for row in 0..4 {
                out[col][row] = self.cols[0][row] * rhs.cols[col][0]
                    + self.cols[1][row] * rhs.cols[col][1]
                    + self.cols[2][row] * rhs.cols[col][2]
                    + self.cols[3][row] * rhs.cols[col][3];
            }
        }
        Self { cols: out }
    }

    /// Compute the inverse matrix, or return `None` if the matrix is singular or non-finite.
    pub fn inverse(&self) -> Option<Self> {
        let mut minors = [[0.0_f32; 4]; 4];
        for row in 0..4 {
            let mut r = [0usize; 3];
            let mut ri = 0;
            for i in 0..4 {
                if i != row {
                    r[ri] = i;
                    ri += 1;
                }
            }
            for col in 0..4 {
                let mut c = [0usize; 3];
                let mut ci = 0;
                for j in 0..4 {
                    if j != col {
                        c[ci] = j;
                        ci += 1;
                    }
                }
                minors[col][row] = det3(
                    self.cols[c[0]][r[0]], self.cols[c[1]][r[0]], self.cols[c[2]][r[0]],
                    self.cols[c[0]][r[1]], self.cols[c[1]][r[1]], self.cols[c[2]][r[1]],
                    self.cols[c[0]][r[2]], self.cols[c[1]][r[2]], self.cols[c[2]][r[2]],
                );
            }
        }

        let det = self.cols[0][0] * minors[0][0]
            - self.cols[1][0] * minors[1][0]
            + self.cols[2][0] * minors[2][0]
            - self.cols[3][0] * minors[3][0];

        if !det.is_finite() || det.abs() <= 1e-12 {
            return None;
        }

        let inv_det = 1.0 / det;
        let mut out = [[0.0_f32; 4]; 4];
        for col in 0..4 {
            for row in 0..4 {
                let sign = if (row + col) % 2 == 0 { 1.0 } else { -1.0 };
                out[col][row] = sign * minors[row][col] * inv_det;
            }
        }
        Some(Self { cols: out })
    }
}

#[inline]
fn det3(
    a: f32, b: f32, c: f32,
    d: f32, e: f32, f: f32,
    g: f32, h: f32, i: f32,
) -> f32 {
    a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)
}

/// Project a 4D homogeneous clip-space coordinate into top-left UV space `[u, v]`.
///
/// Returns `None` if the point is behind the camera plane ($w \le 10^{-6}$) or non-finite.
pub fn clip_to_top_left_uv(clip: [f32; 4]) -> Option<[f32; 2]> {
    let w = clip[3];
    if w <= 1e-6 || !w.is_finite() {
        return None;
    }
    let ndc_x = clip[0] / w;
    let ndc_y = clip[1] / w;
    if !ndc_x.is_finite() || !ndc_y.is_finite() {
        return None;
    }
    // NDC is [-1.0, 1.0] with Y pointing UP.
    // Top-left UV is [0.0, 1.0] with V pointing DOWN.
    let u = (ndc_x + 1.0) * 0.5;
    let v = (1.0 - ndc_y) * 0.5;
    Some([u, v])
}

/// Compute the motion vector for a world point given current and previous camera view-projection matrices.
///
/// Returns `MotionVector2d` pointing from current UV to previous UV:
/// `motion = previous_uv - current_uv`.
/// Returns `None` if either current or previous point is behind the near plane or degenerate.
pub fn calculate_camera_motion_vector(
    current_view_proj: &Mat4ColumnMajor,
    previous_view_proj: &Mat4ColumnMajor,
    world_pos: [f32; 3],
) -> Option<MotionVector2d> {
    let current_clip = current_view_proj.transform_point3(world_pos);
    let previous_clip = previous_view_proj.transform_point3(world_pos);

    let [curr_u, curr_v] = clip_to_top_left_uv(current_clip)?;
    let [prev_u, prev_v] = clip_to_top_left_uv(previous_clip)?;

    Some(MotionVector2d::new(prev_u - curr_u, prev_v - curr_v))
}

/// Compute the motion vector for an animated or moving object point given
/// current and previous object world transforms and camera matrices.
pub fn calculate_object_motion_vector(
    current_model: &Mat4ColumnMajor,
    previous_model: &Mat4ColumnMajor,
    current_view_proj: &Mat4ColumnMajor,
    previous_view_proj: &Mat4ColumnMajor,
    local_pos: [f32; 3],
) -> Option<MotionVector2d> {
    let current_world_4 = current_model.transform_point3(local_pos);
    let previous_world_4 = previous_model.transform_point3(local_pos);

    let curr_world_3 = [current_world_4[0], current_world_4[1], current_world_4[2]];
    let prev_world_3 = [previous_world_4[0], previous_world_4[1], previous_world_4[2]];

    let current_clip = current_view_proj.transform_point3(curr_world_3);
    let previous_clip = previous_view_proj.transform_point3(prev_world_3);

    let [curr_u, curr_v] = clip_to_top_left_uv(current_clip)?;
    let [prev_u, prev_v] = clip_to_top_left_uv(previous_clip)?;

    Some(MotionVector2d::new(prev_u - curr_u, prev_v - curr_v))
}

/// Disocclusion metric between reprojected previous depth and current linear depth.
///
/// Returns `true` if depth disparity exceeds `threshold`, indicating that the
/// surface at the current pixel was occluded in the previous frame and history should
/// be rejected.
pub fn is_disoccluded(current_linear_depth: f32, previous_linear_depth: f32, threshold: f32) -> bool {
    let diff = (current_linear_depth - previous_linear_depth).abs();
    diff > threshold.max(0.0)
}

/// Reactive mask classification parameters.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReactiveMaskClassification {
    pub alpha_edge_weight: f32,
    pub water_weight: f32,
    pub particle_weight: f32,
}

impl ReactiveMaskClassification {
    pub const DEFAULT: Self = Self {
        alpha_edge_weight: 0.85,
        water_weight: 0.50,
        particle_weight: 0.95,
    };

    /// Compute reactive mask value for a fragment based on surface tags.
    pub fn compute_reactive_value(
        &self,
        is_alpha_edge: bool,
        is_water: bool,
        is_particle: bool,
        additional_reactivity: f32,
    ) -> f32 {
        let mut value = additional_reactivity.clamp(0.0, 1.0);
        if is_alpha_edge {
            value = value.max(self.alpha_edge_weight);
        }
        if is_water {
            value = value.max(self.water_weight);
        }
        if is_particle {
            value = value.max(self.particle_weight);
        }
        value.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_camera_yields_zero_motion() {
        let mat = Mat4ColumnMajor::IDENTITY;
        let pos = [0.0, 0.0, 5.0];
        let mv = calculate_camera_motion_vector(&mat, &mat, pos);
        assert!(mv.is_some());
        let mv = mv.unwrap();
        assert!(mv.is_static(1e-5));
        assert_eq!(mv.du, 0.0);
        assert_eq!(mv.dv, 0.0);
    }

    #[test]
    fn camera_pan_yields_expected_motion_vector() {
        // Simple translation: current at origin, previous moved +1.0 in X
        let current = Mat4ColumnMajor::IDENTITY;
        let mut prev_cols = Mat4ColumnMajor::IDENTITY.cols;
        prev_cols[3][0] = -1.0; // shifted in X
        let previous = Mat4ColumnMajor::from_cols(prev_cols);

        let pos = [0.0, 0.0, 2.0];
        let mv = calculate_camera_motion_vector(&current, &previous, pos);
        assert!(mv.is_some());
        let mv = mv.unwrap();
        // Negative X shift moves previous point left in NDC, meaning negative du in UV
        assert!(mv.du < 0.0);
        assert_eq!(mv.dv, 0.0);
    }

    #[test]
    fn object_motion_yields_motion_vector() {
        let view_proj = Mat4ColumnMajor::IDENTITY;
        let current_model = Mat4ColumnMajor::IDENTITY;
        let mut prev_cols = Mat4ColumnMajor::IDENTITY.cols;
        prev_cols[3][1] = 0.5; // object was higher in previous frame
        let previous_model = Mat4ColumnMajor::from_cols(prev_cols);

        let local_pos = [0.0, 0.0, 2.0];
        let mv = calculate_object_motion_vector(
            &current_model,
            &previous_model,
            &view_proj,
            &view_proj,
            local_pos,
        );
        assert!(mv.is_some());
        let mv = mv.unwrap();
        assert_eq!(mv.du, 0.0);
        // Previous Y was higher (+0.5 in NDC), so previous V in UV (top-down) is lower (-dv)
        assert!(mv.dv < 0.0);
    }

    #[test]
    fn behind_camera_point_is_rejected() {
        let view_proj = Mat4ColumnMajor::IDENTITY;
        // Point with negative W (behind camera)
        let pos = [0.0, 0.0, -1.0];
        let mut cols = Mat4ColumnMajor::IDENTITY.cols;
        cols[3][3] = -1.0; // w becomes -1.0
        let bad_mat = Mat4ColumnMajor::from_cols(cols);

        let mv = calculate_camera_motion_vector(&bad_mat, &view_proj, pos);
        assert!(mv.is_none());
    }

    #[test]
    fn disocclusion_metric_respects_threshold() {
        assert!(!is_disoccluded(1.0, 1.005, 0.01));
        assert!(is_disoccluded(1.0, 1.02, 0.01));
        assert!(is_disoccluded(1.0, 0.95, 0.01));
    }

    #[test]
    fn reactive_mask_classification_prioritizes_highest_reactivity() {
        let classifier = ReactiveMaskClassification::DEFAULT;
        let v_none = classifier.compute_reactive_value(false, false, false, 0.0);
        assert_eq!(v_none, 0.0);

        let v_water = classifier.compute_reactive_value(false, true, false, 0.0);
        assert_eq!(v_water, 0.50);

        let v_alpha = classifier.compute_reactive_value(true, false, false, 0.0);
        assert_eq!(v_alpha, 0.85);

        let v_particle = classifier.compute_reactive_value(false, false, true, 0.0);
        assert_eq!(v_particle, 0.95);

        let v_combined = classifier.compute_reactive_value(true, true, true, 0.1);
        assert_eq!(v_combined, 0.95);
    }

    #[test]
    fn matrix_identity_inverse_is_identity() {
        let id = Mat4ColumnMajor::IDENTITY;
        let inv = id.inverse().expect("inverse");
        assert_eq!(inv, id);
    }

    #[test]
    fn matrix_inverse_round_trip() {
        let mut m = Mat4ColumnMajor::IDENTITY;
        m.cols[0][0] = 2.0;
        m.cols[1][1] = 3.0;
        m.cols[2][2] = 4.0;
        m.cols[3][0] = 5.0;
        m.cols[3][1] = 6.0;
        m.cols[3][2] = 7.0;

        let inv = m.inverse().expect("inverse");
        let prod = m.mul(&inv);

        for c in 0..4 {
            for r in 0..4 {
                let expected = if c == r { 1.0 } else { 0.0 };
                assert!((prod.cols[c][r] - expected).abs() < 1e-5);
            }
        }
    }
}
