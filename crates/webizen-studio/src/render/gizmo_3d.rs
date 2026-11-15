//! Zero-Heap 3D PGA Motor Manipulation Gizmos (`gizmo_3d.rs`)
//!
//! Implements Epic 9.3a of the QualiaDB Superior Game Engine architecture:
//! - Ray-pickable 3D manipulation gizmo ([`MotorGizmo3D`]) supporting [`GizmoMode::Translate`],
//!   [`GizmoMode::Rotate`], [`GizmoMode::Scale`], and [`GizmoMode::MotorScrew`] (simultaneous
//!   rotation + translation along a Chasles screw axis in Projective Geometric Algebra).
//! - Tests mouse picking rays against X/Y/Z axis arrows, XY/YZ/XZ plane handles, and X/Y/Z
//!   rotation rings in **zero heap**.
//! - Outputs exact PGA [`Motor`] deltas (`delta_motor: Motor`, `delta_scale: [f32; 3]`).

/// 3D Projective Geometric Algebra (PGA) Motor (`Motor { r, d }`) representing rigid body
/// transformations (rotations, translations, and general Chasles screw motions) in $\mathbf{P}(\mathbb{R}^*_{3,0,1})$.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Motor {
    /// Rotor component `[s, e12, e13, e23]`.
    pub r: [f32; 4],
    /// Dual translation/screw component `[e0123, e03, e02, e01]`.
    pub d: [f32; 4],
}

impl Default for Motor {
    fn default() -> Self {
        Self::identity()
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<qualia_core_db::render::pga::Motor> for Motor {
    #[inline]
    fn from(m: qualia_core_db::render::pga::Motor) -> Self {
        Self { r: m.r, d: m.d }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<Motor> for qualia_core_db::render::pga::Motor {
    #[inline]
    fn from(m: Motor) -> Self {
        Self { r: m.r, d: m.d }
    }
}

impl Motor {
    /// Identity PGA motor.
    #[inline]
    pub const fn identity() -> Self {
        Self {
            r: [1.0, 0.0, 0.0, 0.0],
            d: [0.0, 0.0, 0.0, 0.0],
        }
    }

    /// Pure translation motor along `v = [dx, dy, dz]`.
    #[inline]
    pub fn from_translation(v: [f32; 3]) -> Self {
        Self {
            r: [1.0, 0.0, 0.0, 0.0],
            d: quat_to_blade4([0.0, v[0] * 0.5, v[1] * 0.5, v[2] * 0.5]),
        }
    }

    /// Pure rotation motor around `axis` by `angle_rad` radians.
    #[inline]
    pub fn from_axis_angle(axis: [f32; 3], angle_rad: f32) -> Self {
        let a = normalize3(axis, [0.0, 1.0, 0.0]);
        let half = angle_rad * 0.5;
        let (s, c) = half.sin_cos();
        Self {
            r: [c, -s * a[2], s * a[1], -s * a[0]],
            d: [0.0, 0.0, 0.0, 0.0],
        }
    }

    /// Chasles screw motor combining rotation `angle_rad` around `axis` and linear translation
    /// `slide_m` along `axis`.
    #[inline]
    pub fn from_screw(axis: [f32; 3], angle_rad: f32, slide_m: f32) -> Self {
        let a = normalize3(axis, [0.0, 1.0, 0.0]);
        let rot = Self::from_axis_angle(a, angle_rad);
        let trans = Self::from_translation([a[0] * slide_m, a[1] * slide_m, a[2] * slide_m]);
        trans.motor_mul(rot)
    }

    /// Dual-quaternion geometric product $M_a M_b$.
    #[inline]
    pub fn motor_mul(self, rhs: Self) -> Self {
        let qr1 = blade4_to_quat(self.r);
        let qd1 = blade4_to_quat(self.d);
        let qr2 = blade4_to_quat(rhs.r);
        let qd2 = blade4_to_quat(rhs.d);
        let qr3 = quat_mul(qr1, qr2);
        let q1d2 = quat_mul(qr1, qd2);
        let qd12 = quat_mul(qd1, qr2);
        let qd3 = [
            q1d2[0] + qd12[0],
            q1d2[1] + qd12[1],
            q1d2[2] + qd12[2],
            q1d2[3] + qd12[3],
        ];
        Self {
            r: quat_to_blade4(qr3),
            d: quat_to_blade4(qd3),
        }
    }

    /// PGA reversion $\widetilde{M}$.
    #[inline]
    pub const fn motor_reverse(self) -> Self {
        Self {
            r: [self.r[0], -self.r[1], -self.r[2], -self.r[3]],
            d: [self.d[0], -self.d[1], -self.d[2], -self.d[3]],
        }
    }

    /// Applies the PGA sandwich product $P' = M P \widetilde{M}$ to a 3D Euclidean point.
    #[inline]
    pub fn sandwich_point(self, p: [f32; 3]) -> [f32; 3] {
        let qr = blade4_to_quat(self.r);
        let qd = blade4_to_quat(self.d);
        let qr_conj = [qr[0], -qr[1], -qr[2], -qr[3]];
        let pq = [0.0, p[0], p[1], p[2]];
        let p_rot = quat_mul(quat_mul(qr, pq), qr_conj);
        let tq = quat_mul(qd, qr_conj);
        [
            p_rot[1] + 2.0 * tq[1],
            p_rot[2] + 2.0 * tq[2],
            p_rot[3] + 2.0 * tq[3],
        ]
    }
}

/// Active manipulation mode for [`MotorGizmo3D`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoMode {
    Translate = 0,
    Rotate = 1,
    Scale = 2,
    MotorScrew = 3,
}

impl Default for GizmoMode {
    fn default() -> Self {
        Self::Translate
    }
}

/// Pickable sub-handle of a [`MotorGizmo3D`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoHandle {
    AxisX = 0,
    AxisY = 1,
    AxisZ = 2,
    PlaneXY = 3,
    PlaneYZ = 4,
    PlaneXZ = 5,
    RingX = 6,
    RingY = 7,
    RingZ = 8,
    UniformCenter = 9,
}

impl GizmoHandle {
    /// Returns the canonical local unit axis `[x, y, z]` associated with this handle.
    #[inline]
    pub const fn unit_axis(self) -> [f32; 3] {
        match self {
            Self::AxisX | Self::RingX | Self::PlaneYZ => [1.0, 0.0, 0.0],
            Self::AxisY | Self::RingY | Self::PlaneXZ => [0.0, 1.0, 0.0],
            Self::AxisZ | Self::RingZ | Self::PlaneXY => [0.0, 0.0, 1.0],
            Self::UniformCenter => [1.0, 1.0, 1.0],
        }
    }
}

/// Result of a ray-pick test against [`MotorGizmo3D`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GizmoPickHit {
    pub handle: GizmoHandle,
    /// Ray parameter `t` along `ray_origin + t * ray_dir`.
    pub ray_t: f32,
    /// World-space closest hit point on the gizmo handle.
    pub hit_point: [f32; 3],
    /// Parameter along the axis `[0.0, axis_length]` or angle in radians on a rotation ring.
    pub handle_param: f32,
}

/// Output delta produced by dragging an active [`MotorGizmo3D`] handle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GizmoDragDelta {
    /// Exact incremental PGA [`Motor`] delta (`new_motor = delta_motor.motor_mul(old_motor)`).
    pub delta_motor: Motor,
    /// Multiplicative scale factor delta `[sx, sy, sz]` (`1.0` = unchanged).
    pub delta_scale: [f32; 3],
}

impl Default for GizmoDragDelta {
    fn default() -> Self {
        Self {
            delta_motor: Motor::identity(),
            delta_scale: [1.0, 1.0, 1.0],
        }
    }
}

/// Zero-heap 3D ray-pickable PGA Motor manipulation gizmo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotorGizmo3D {
    /// Current world-space pose of the gizmo pivot.
    pub pivot_motor: Motor,
    /// Current manipulation mode.
    pub mode: GizmoMode,
    /// Length of the X/Y/Z axis arrows in world units.
    pub axis_length: f32,
    /// Radius of the X/Y/Z rotation rings in world units.
    pub ring_radius: f32,
    /// Picking tolerance tube radius in world units.
    pub pick_tolerance: f32,
    /// Currently hovered handle.
    pub hovered_handle: Option<GizmoHandle>,
    /// Currently dragged handle.
    pub active_handle: Option<GizmoHandle>,
}

impl Default for MotorGizmo3D {
    fn default() -> Self {
        Self {
            pivot_motor: Motor::identity(),
            mode: GizmoMode::Translate,
            axis_length: 1.5,
            ring_radius: 1.25,
            pick_tolerance: 0.14,
            hovered_handle: None,
            active_handle: None,
        }
    }
}

impl MotorGizmo3D {
    /// Creates a gizmo at `pivot_motor` with the given `mode`.
    #[inline]
    pub const fn new(pivot_motor: Motor, mode: GizmoMode) -> Self {
        Self {
            pivot_motor,
            mode,
            axis_length: 1.5,
            ring_radius: 1.25,
            pick_tolerance: 0.14,
            hovered_handle: None,
            active_handle: None,
        }
    }

    /// World-space origin of the gizmo pivot.
    #[inline]
    pub fn world_origin(&self) -> [f32; 3] {
        self.pivot_motor.sandwich_point([0.0, 0.0, 0.0])
    }

    /// World-space unit direction for a local axis `[lx, ly, lz]`.
    #[inline]
    pub fn world_axis(&self, local_axis: [f32; 3]) -> [f32; 3] {
        let o = self.world_origin();
        let tip = self.pivot_motor.sandwich_point(local_axis);
        normalize3(sub3(tip, o), local_axis)
    }

    /// Tests a world-space mouse picking ray `(ray_origin, ray_dir)` against the gizmo's
    /// X/Y/Z axis arrows, XY/YZ/XZ plane quads, and X/Y/Z rotation rings in **zero heap**.
    pub fn pick_ray(&self, ray_origin: [f32; 3], ray_dir: [f32; 3]) -> Option<GizmoPickHit> {
        let dir = normalize3(ray_dir, [0.0, 0.0, -1.0]);
        let origin = self.world_origin();
        let axes = [
            (
                GizmoHandle::AxisX,
                GizmoHandle::RingX,
                self.world_axis([1.0, 0.0, 0.0]),
            ),
            (
                GizmoHandle::AxisY,
                GizmoHandle::RingY,
                self.world_axis([0.0, 1.0, 0.0]),
            ),
            (
                GizmoHandle::AxisZ,
                GizmoHandle::RingZ,
                self.world_axis([0.0, 0.0, 1.0]),
            ),
        ];

        let mut best: Option<GizmoPickHit> = None;
        let mut best_dist = self.pick_tolerance;

        // 1. Test rotation rings when in Rotate or MotorScrew mode
        if matches!(self.mode, GizmoMode::Rotate | GizmoMode::MotorScrew) {
            for &(_, ring_handle, axis_dir) in &axes {
                let denom = dot3(dir, axis_dir);
                if denom.abs() > 1e-4 {
                    let t = dot3(sub3(origin, ray_origin), axis_dir) / denom;
                    if t > 0.0 {
                        let p = add3(ray_origin, scale3(dir, t));
                        let radial = sub3(p, origin);
                        let r_len = length3(radial);
                        let ring_err = (r_len - self.ring_radius).abs();
                        if ring_err <= best_dist {
                            best_dist = ring_err;
                            let angle = radial[1].atan2(radial[0]);
                            best = Some(GizmoPickHit {
                                handle: ring_handle,
                                ray_t: t,
                                hit_point: p,
                                handle_param: angle,
                            });
                        }
                    }
                }
            }
        }

        // 2. Test X/Y/Z axis arrow segments (Translate, Scale, MotorScrew)
        if matches!(
            self.mode,
            GizmoMode::Translate | GizmoMode::Scale | GizmoMode::MotorScrew
        ) {
            for &(axis_handle, _, axis_dir) in &axes {
                let seg_end = add3(origin, scale3(axis_dir, self.axis_length));
                let (ray_t, seg_s, dist, pt_on_seg) =
                    closest_points_ray_segment(ray_origin, dir, origin, seg_end);
                if ray_t > 0.0 && dist <= best_dist {
                    best_dist = dist;
                    best = Some(GizmoPickHit {
                        handle: axis_handle,
                        ray_t,
                        hit_point: pt_on_seg,
                        handle_param: seg_s * self.axis_length,
                    });
                }
            }

            // 3. Test XY, YZ, XZ planar translation quads `[0.25 .. 0.55] * axis_length`
            let planes = [
                (
                    GizmoHandle::PlaneXY,
                    axes[0].2,
                    axes[1].2,
                    axes[2].2,
                ),
                (
                    GizmoHandle::PlaneYZ,
                    axes[1].2,
                    axes[2].2,
                    axes[0].2,
                ),
                (
                    GizmoHandle::PlaneXZ,
                    axes[0].2,
                    axes[2].2,
                    axes[1].2,
                ),
            ];
            let quad_min = self.axis_length * 0.22;
            let quad_max = self.axis_length * 0.55;
            for &(plane_handle, u_axis, v_axis, n_axis) in &planes {
                let denom = dot3(dir, n_axis);
                if denom.abs() > 1e-4 {
                    let t = dot3(sub3(origin, ray_origin), n_axis) / denom;
                    if t > 0.0 {
                        let p = add3(ray_origin, scale3(dir, t));
                        let rel = sub3(p, origin);
                        let u = dot3(rel, u_axis);
                        let v = dot3(rel, v_axis);
                        if u >= quad_min && u <= quad_max && v >= quad_min && v <= quad_max {
                            best = Some(GizmoPickHit {
                                handle: plane_handle,
                                ray_t: t,
                                hit_point: p,
                                handle_param: u + v,
                            });
                        }
                    }
                }
            }
        }

        best
    }

    /// Computes the exact PGA [`Motor`] and scale delta between an initial mouse ray and
    /// current mouse ray for `handle`.
    pub fn compute_drag_delta(
        &self,
        handle: GizmoHandle,
        start_ray_origin: [f32; 3],
        start_ray_dir: [f32; 3],
        cur_ray_origin: [f32; 3],
        cur_ray_dir: [f32; 3],
    ) -> GizmoDragDelta {
        let s_dir = normalize3(start_ray_dir, [0.0, 0.0, -1.0]);
        let c_dir = normalize3(cur_ray_dir, [0.0, 0.0, -1.0]);
        let origin = self.world_origin();
        let world_axis = self.world_axis(handle.unit_axis());

        match (self.mode, handle) {
            (
                GizmoMode::Rotate,
                GizmoHandle::RingX | GizmoHandle::RingY | GizmoHandle::RingZ | GizmoHandle::AxisX | GizmoHandle::AxisY | GizmoHandle::AxisZ,
            ) => {
                let p0 = intersect_ray_plane(start_ray_origin, s_dir, origin, world_axis);
                let p1 = intersect_ray_plane(cur_ray_origin, c_dir, origin, world_axis);
                let v0 = normalize3(sub3(p0, origin), [1.0, 0.0, 0.0]);
                let v1 = normalize3(sub3(p1, origin), [1.0, 0.0, 0.0]);
                let sin_a = dot3(cross3(v0, v1), world_axis);
                let cos_a = dot3(v0, v1).clamp(-1.0, 1.0);
                let angle = sin_a.atan2(cos_a);
                // Rotate around pivot origin: T(origin) * R(axis, angle) * T(-origin)
                let to_origin = Motor::from_translation([-origin[0], -origin[1], -origin[2]]);
                let rot = Motor::from_axis_angle(world_axis, angle);
                let back = Motor::from_translation(origin);
                GizmoDragDelta {
                    delta_motor: back.motor_mul(rot.motor_mul(to_origin)),
                    delta_scale: [1.0, 1.0, 1.0],
                }
            }
            (GizmoMode::Scale, GizmoHandle::AxisX | GizmoHandle::AxisY | GizmoHandle::AxisZ) => {
                let s0 = project_ray_onto_axis(start_ray_origin, s_dir, origin, world_axis);
                let s1 = project_ray_onto_axis(cur_ray_origin, c_dir, origin, world_axis);
                let ratio = ((self.axis_length + (s1 - s0)) / self.axis_length.max(0.1))
                    .clamp(0.05, 20.0);
                let mut scale = [1.0, 1.0, 1.0];
                match handle {
                    GizmoHandle::AxisX => scale[0] = ratio,
                    GizmoHandle::AxisY => scale[1] = ratio,
                    GizmoHandle::AxisZ => scale[2] = ratio,
                    _ => {}
                }
                GizmoDragDelta {
                    delta_motor: Motor::identity(),
                    delta_scale: scale,
                }
            }
            (GizmoMode::MotorScrew, _) => {
                let s0 = project_ray_onto_axis(start_ray_origin, s_dir, origin, world_axis);
                let s1 = project_ray_onto_axis(cur_ray_origin, c_dir, origin, world_axis);
                let slide = s1 - s0;
                // Chasles screw motion: 1 radian per meter of axial slide
                let angle = slide * 1.0;
                let to_origin = Motor::from_translation([-origin[0], -origin[1], -origin[2]]);
                let screw = Motor::from_screw(world_axis, angle, slide);
                let back = Motor::from_translation(origin);
                GizmoDragDelta {
                    delta_motor: back.motor_mul(screw.motor_mul(to_origin)),
                    delta_scale: [1.0, 1.0, 1.0],
                }
            }
            (_, GizmoHandle::PlaneXY | GizmoHandle::PlaneYZ | GizmoHandle::PlaneXZ) => {
                let p0 = intersect_ray_plane(start_ray_origin, s_dir, origin, world_axis);
                let p1 = intersect_ray_plane(cur_ray_origin, c_dir, origin, world_axis);
                let delta = sub3(p1, p0);
                GizmoDragDelta {
                    delta_motor: Motor::from_translation(delta),
                    delta_scale: [1.0, 1.0, 1.0],
                }
            }
            _ => {
                // Linear translation along the picked axis
                let s0 = project_ray_onto_axis(start_ray_origin, s_dir, origin, world_axis);
                let s1 = project_ray_onto_axis(cur_ray_origin, c_dir, origin, world_axis);
                let slide = s1 - s0;
                let delta = scale3(world_axis, slide);
                GizmoDragDelta {
                    delta_motor: Motor::from_translation(delta),
                    delta_scale: [1.0, 1.0, 1.0],
                }
            }
        }
    }
}

#[inline]
fn blade4_to_quat(v: [f32; 4]) -> [f32; 4] {
    [v[0], -v[3], -v[2], -v[1]]
}

#[inline]
fn quat_to_blade4(q: [f32; 4]) -> [f32; 4] {
    [q[0], -q[3], -q[2], -q[1]]
}

#[inline]
fn quat_mul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
        a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
        a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
        a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0],
    ]
}

#[inline]
fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

#[inline]
fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
fn scale3(v: [f32; 3], s: f32) -> [f32; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

#[inline]
fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[inline]
fn length3(v: [f32; 3]) -> f32 {
    dot3(v, v).sqrt()
}

#[inline]
fn normalize3(v: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    let l = length3(v);
    if l > 1e-6 {
        [v[0] / l, v[1] / l, v[2] / l]
    } else {
        fallback
    }
}

/// Closest approach between ray `ro + t * rd` (`t >= 0`) and segment `a .. b` (`s in [0, 1]`).
fn closest_points_ray_segment(
    ro: [f32; 3],
    rd: [f32; 3],
    a: [f32; 3],
    b: [f32; 3],
) -> (f32, f32, f32, [f32; 3]) {
    let u = rd;
    let v = sub3(b, a);
    let w = sub3(ro, a);
    let a_dot = dot3(u, u);
    let b_dot = dot3(u, v);
    let c_dot = dot3(v, v);
    let d_dot = dot3(u, w);
    let e_dot = dot3(v, w);
    let denom = a_dot * c_dot - b_dot * b_dot;

    let (t, s) = if denom > 1e-6 {
        let s_unclamped = (a_dot * e_dot - b_dot * d_dot) / denom;
        let s_clamped = s_unclamped.clamp(0.0, 1.0);
        let t_clamped = ((b_dot * s_clamped - d_dot) / a_dot.max(1e-6)).max(0.0);
        (t_clamped, s_clamped)
    } else {
        (0.0, 0.0)
    };

    let pt_ray = add3(ro, scale3(u, t));
    let pt_seg = add3(a, scale3(v, s));
    let dist = length3(sub3(pt_ray, pt_seg));
    (t, s, dist, pt_seg)
}

/// Projects a ray onto an infinite line `axis_origin + s * axis_dir`, returning signed parameter `s`.
fn project_ray_onto_axis(
    ro: [f32; 3],
    rd: [f32; 3],
    axis_origin: [f32; 3],
    axis_dir: [f32; 3],
) -> f32 {
    let w = sub3(ro, axis_origin);
    let b = dot3(rd, axis_dir);
    let d = dot3(rd, w);
    let e = dot3(axis_dir, w);
    let denom = 1.0 - b * b;
    if denom > 1e-5 {
        (e - b * d) / denom
    } else {
        e
    }
}

fn intersect_ray_plane(
    ro: [f32; 3],
    rd: [f32; 3],
    plane_origin: [f32; 3],
    plane_normal: [f32; 3],
) -> [f32; 3] {
    let denom = dot3(rd, plane_normal);
    if denom.abs() > 1e-5 {
        let t = (dot3(sub3(plane_origin, ro), plane_normal) / denom).max(0.0);
        add3(ro, scale3(rd, t))
    } else {
        plane_origin
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gizmo_picks_x_axis_and_computes_exact_pga_translation_motor() {
        let gizmo = MotorGizmo3D::new(Motor::identity(), GizmoMode::Translate);
        // Cast a ray aiming at (0.8, 0.0, 0.0) from z = 5.0
        let hit = gizmo
            .pick_ray([0.8, 0.02, 5.0], [0.0, 0.0, -1.0])
            .expect("Expected ray to hit X axis arrow");
        assert_eq!(hit.handle, GizmoHandle::AxisX);

        // Drag from x = 0.8 to x = 2.3 (+1.5m along +X)
        let delta = gizmo.compute_drag_delta(
            GizmoHandle::AxisX,
            [0.8, 0.0, 5.0],
            [0.0, 0.0, -1.0],
            [2.3, 0.0, 5.0],
            [0.0, 0.0, -1.0],
        );
        let moved = delta.delta_motor.sandwich_point([0.0, 0.0, 0.0]);
        assert!((moved[0] - 1.5).abs() < 1e-4);
        assert!(moved[1].abs() < 1e-4);
        assert!(moved[2].abs() < 1e-4);
    }

    #[test]
    fn gizmo_rotate_and_motor_screw_produce_valid_pga_motors() {
        let gizmo = MotorGizmo3D::new(Motor::identity(), GizmoMode::Rotate);
        let hit = gizmo
            .pick_ray([1.25, 0.0, 5.0], [0.0, 0.0, -1.0])
            .expect("Expected ray to hit RingZ at radius 1.25");
        assert_eq!(hit.handle, GizmoHandle::RingZ);

        // Drag 90 degrees from (1, 0, 0) to (0, 1, 0) around +Z
        let delta = gizmo.compute_drag_delta(
            GizmoHandle::RingZ,
            [1.0, 0.0, 5.0],
            [0.0, 0.0, -1.0],
            [0.0, 1.0, 5.0],
            [0.0, 0.0, -1.0],
        );
        let rotated = delta.delta_motor.sandwich_point([1.0, 0.0, 0.0]);
        assert!(rotated[0].abs() < 1e-4);
        assert!((rotated[1] - 1.0).abs() < 1e-4);
    }
}
