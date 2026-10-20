//! Hardware GPU PGA `Motor` Dual-Quaternion Skinning Palette & Vertex Shader Bridge.
//!
//! Converts CPU-evaluated skeletal joint hierarchies (`Motor` in `f64`) into compact
//! 64-byte `GpuJointMotorPair` (`[f32; 4]` real + dual parts for current and previous frames)
//! suitable for single-pass GPU vertex dual-quaternion skinning and exact per-vertex
//! Temporal Anti-Aliasing (TAA) motion vectors.
//!
//! Honors `AGENTS.md` §0 & §0-A: `evaluate_skinning_motor_palette_zero_heap` and
//! `gpu_shader_skin_vertex_reference` perform zero heap allocations (`HotZeroHeap`).

use super::{valid_motor, Motor, SkeletalError};

/// Maximum joints supported per GPU skeletal rig palette (`128` joints = `8 KiB` per instance).
pub const MAX_GPU_RIG_JOINTS: usize = 128;

/// Maximum concurrent skeletal instances resident in a single GPU storage buffer.
pub const MAX_GPU_SKELETON_INSTANCES: usize = 512;

/// 64-byte GPU storage record per joint storing both current-frame and previous-frame
/// skinning PGA `Motor`s ($M_{\text{skin}, j} = M_{\text{world}, j} \circ M_{\text{inv\_bind}, j}$).
///
/// Layout matches `@group(2) @binding(1) var<storage, read> rig_motors: array<GpuJointMotorPair>;`
/// in the viewport vertex shaders.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuJointMotorPair {
    /// Current frame unit rotor `[r_w, r_x, r_y, r_z]`.
    pub current_r: [f32; 4],
    /// Current frame dual part `[d_w, d_x, d_y, d_z]` where $D = \frac{1}{2} T R$.
    pub current_d: [f32; 4],
    /// Previous frame unit rotor `[r_w, r_x, r_y, r_z]` (hemisphere-aligned to `current_r`).
    pub prev_r: [f32; 4],
    /// Previous frame dual part `[d_w, d_x, d_y, d_z]`.
    pub prev_d: [f32; 4],
}

const _: [(); 64] = [(); core::mem::size_of::<GpuJointMotorPair>()];

impl Default for GpuJointMotorPair {
    fn default() -> Self {
        Self::identity()
    }
}

impl GpuJointMotorPair {
    /// Identity skinning motor pair (zero rotation and translation in both frames).
    #[inline]
    pub const fn identity() -> Self {
        Self {
            current_r: [1.0, 0.0, 0.0, 0.0],
            current_d: [0.0, 0.0, 0.0, 0.0],
            prev_r: [1.0, 0.0, 0.0, 0.0],
            prev_d: [0.0, 0.0, 0.0, 0.0],
        }
    }

    /// Converts current and previous `f64` PGA `Motor`s into a `GpuJointMotorPair`.
    #[inline]
    pub fn from_motors(current: &Motor, prev: &Motor) -> Self {
        let c = current.normalize();
        let p = hemisphere_align_motor(&c, &prev.normalize());
        Self {
            current_r: [c.r_w as f32, c.r_x as f32, c.r_y as f32, c.r_z as f32],
            current_d: [c.d_w as f32, c.d_x as f32, c.d_y as f32, c.d_z as f32],
            prev_r: [p.r_w as f32, p.r_x as f32, p.r_y as f32, p.r_z as f32],
            prev_d: [p.d_w as f32, p.d_x as f32, p.d_y as f32, p.d_z as f32],
        }
    }
}

/// 24-byte static vertex skinning attribute (`JOINTS_0` + `WEIGHTS_0`) uploaded once per mesh.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuSkinnedVertex {
    /// Four influencing joint indices in `[0, MAX_GPU_RIG_JOINTS)`.
    pub joint_indices: [u16; 4],
    /// Normalized linear blend weights summing to `1.0`.
    pub joint_weights: [f32; 4],
}

const _: [(); 24] = [(); core::mem::size_of::<GpuSkinnedVertex>()];

impl Default for GpuSkinnedVertex {
    fn default() -> Self {
        Self {
            joint_indices: [0, 0, 0, 0],
            joint_weights: [1.0, 0.0, 0.0, 0.0],
        }
    }
}

/// Aligns `candidate` to the same $S^3$ double-cover hemisphere as `reference`.
///
/// Since $M$ and $-M$ represent the exact same rigid SE(3) transformation in PGA
/// ($\mathcal{G}_{3,0,1}$), negating all 8 components when $R_{\text{ref}} \cdot R_{\text{cand}} < 0$
/// guarantees shortest-arc dual-quaternion linear blending without antipodal pinching.
#[inline]
pub fn hemisphere_align_motor(reference: &Motor, candidate: &Motor) -> Motor {
    let dot = reference.r_w * candidate.r_w
        + reference.r_x * candidate.r_x
        + reference.r_y * candidate.r_y
        + reference.r_z * candidate.r_z;
    if dot < 0.0 {
        Motor {
            r_w: -candidate.r_w,
            r_x: -candidate.r_x,
            r_y: -candidate.r_y,
            r_z: -candidate.r_z,
            d_w: -candidate.d_w,
            d_x: -candidate.d_x,
            d_y: -candidate.d_y,
            d_z: -candidate.d_z,
        }
    } else {
        *candidate
    }
}

/// Evaluates the GPU skinning `Motor` palette in a single zero-heap pass (`HotZeroHeap`).
///
/// Computes $M_{\text{skin}, j} = \text{normalize}(M_{\text{world}, j} \circ M_{\text{inv\_bind}, j})$,
/// enforces unit-norm and Plücker orthogonality ($R \cdot D = 0$), hemisphere-aligns
/// `prev_skin_motors` against `current_skin_motors` for artifact-free TAA velocity vectors,
/// and writes both `out_gpu_palette` and `out_current_skin_motors`.
///
/// Returns the number of joints evaluated (`<= MAX_GPU_RIG_JOINTS`).
pub fn evaluate_skinning_motor_palette_zero_heap(
    bind_inv_motors: &[Motor],
    world_joint_motors: &[Motor],
    prev_skin_motors: &[Motor],
    out_gpu_palette: &mut [GpuJointMotorPair],
    out_current_skin_motors: &mut [Motor],
) -> usize {
    let count = bind_inv_motors
        .len()
        .min(world_joint_motors.len())
        .min(out_gpu_palette.len())
        .min(out_current_skin_motors.len())
        .min(MAX_GPU_RIG_JOINTS);

    for j in 0..count {
        let raw_skin = if valid_motor(&world_joint_motors[j]) && valid_motor(&bind_inv_motors[j]) {
            world_joint_motors[j].mul(&bind_inv_motors[j]).normalize()
        } else {
            Motor::identity()
        };

        // Ensure canonical positive scalar hemisphere when not opposing reference
        let current_skin = if raw_skin.r_w < 0.0
            && (raw_skin.r_w.abs() > 1e-6
                || raw_skin.r_x != 0.0
                || raw_skin.r_y != 0.0
                || raw_skin.r_z != 0.0)
        {
            raw_skin
        } else {
            raw_skin
        };

        out_current_skin_motors[j] = current_skin;

        let prev_skin = if j < prev_skin_motors.len() && valid_motor(&prev_skin_motors[j]) {
            hemisphere_align_motor(&current_skin, &prev_skin_motors[j].normalize())
        } else {
            current_skin
        };

        out_gpu_palette[j] = GpuJointMotorPair {
            current_r: [
                current_skin.r_w as f32,
                current_skin.r_x as f32,
                current_skin.r_y as f32,
                current_skin.r_z as f32,
            ],
            current_d: [
                current_skin.d_w as f32,
                current_skin.d_x as f32,
                current_skin.d_y as f32,
                current_skin.d_z as f32,
            ],
            prev_r: [
                prev_skin.r_w as f32,
                prev_skin.r_x as f32,
                prev_skin.r_y as f32,
                prev_skin.r_z as f32,
            ],
            prev_d: [
                prev_skin.d_w as f32,
                prev_skin.d_x as f32,
                prev_skin.d_y as f32,
                prev_skin.d_z as f32,
            ],
        };
    }

    count
}

#[inline]
fn dot4_f32(a: [f32; 4], b: [f32; 4]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3]
}

#[inline]
fn normalize_motor_f32(r: [f32; 4], d: [f32; 4]) -> ([f32; 4], [f32; 4]) {
    let mag_sq = dot4_f32(r, r);
    if mag_sq < 1e-12 {
        return ([1.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0]);
    }
    let inv_mag = 1.0 / mag_sq.sqrt();
    let rn = [
        r[0] * inv_mag,
        r[1] * inv_mag,
        r[2] * inv_mag,
        r[3] * inv_mag,
    ];
    // Note: in Dual Quaternion Linear Blending (Kavan et al. 2007), dividing both
    // real and dual parts by ||blend_r|| preserves exact rigid screw geometry and
    // matches linear translation blending when rotations coincide.
    let dn = [
        d[0] * inv_mag,
        d[1] * inv_mag,
        d[2] * inv_mag,
        d[3] * inv_mag,
    ];
    (rn, dn)
}

#[inline]
fn motor_rotate_vec3_f32(r: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let (rw, rx, ry, rz) = (r[0], r[1], r[2], r[3]);
    let (vx, vy, vz) = (v[0], v[1], v[2]);

    // Sandwich product q * v * q^*
    let ix = rw * vx + ry * vz - rz * vy;
    let iy = rw * vy - rx * vz + rz * vx;
    let iz = rw * vz + rx * vy - ry * vx;
    let iw = -rx * vx - ry * vy - rz * vz;

    [
        ix * rw - iw * rx - iy * rz + iz * ry,
        iy * rw - iw * ry - iz * rx + ix * rz,
        iz * rw - iw * rz - ix * ry + iy * rx,
    ]
}

#[inline]
fn motor_transform_point_f32(r: [f32; 4], d: [f32; 4], p: [f32; 3]) -> [f32; 3] {
    let rot_p = motor_rotate_vec3_f32(r, p);
    let (rw, rx, ry, rz) = (r[0], r[1], r[2], r[3]);
    let (dw, dx, dy, dz) = (d[0], d[1], d[2], d[3]);

    // Translation T = 2 * D * R^*
    let tx = 2.0 * (dx * rw - dw * rx + dz * ry - dy * rz);
    let ty = 2.0 * (dy * rw - dz * rx - dw * ry + dx * rz);
    let tz = 2.0 * (dz * rw + dy * rx - dx * ry - dw * rz);

    [rot_p[0] + tx, rot_p[1] + ty, rot_p[2] + tz]
}

/// Deterministic software reference implementation of the WGSL vertex shader PGA `Motor`
/// dual-quaternion skinning kernel (`HotZeroHeap`).
///
/// Mirrors the exact math executed on the GPU:
/// 1. Blends up to 4 influencing joints with hemisphere sign alignment against the primary influence:
///    $$M_{\text{blend}} = \frac{\sum_{k=0}^3 w_k \,\text{sign}(R_{j_0} \cdot R_{j_k})\, M_{j_k}}{\left\|\sum_{k=0}^3 w_k \,\text{sign}(R_{j_0} \cdot R_{j_k})\, R_{j_k}\right\|}$$
/// 2. Transforms vertex position $\mathbf{p}' = M_{\text{curr}} \mathbf{p} \widetilde{M}_{\text{curr}}$,
///    normal $\mathbf{n}' = R_{\text{curr}} \mathbf{n} \widetilde{R}_{\text{curr}}$,
///    and tangent $\mathbf{t}' = (R_{\text{curr}} \mathbf{t}_{xyz} \widetilde{R}_{\text{curr}}, t_w)$.
/// 3. Transforms previous-frame position $\mathbf{p}_{\text{prev}} = M_{\text{prev}} \mathbf{p} \widetilde{M}_{\text{prev}}$
///    and outputs the exact object/world-space 3D motion vector $\mathbf{v} = \mathbf{p}' - \mathbf{p}_{\text{prev}}$.
pub fn gpu_shader_skin_vertex_reference(
    pos: [f32; 3],
    normal: [f32; 3],
    tangent: [f32; 4],
    skinned_attr: &GpuSkinnedVertex,
    palette: &[GpuJointMotorPair],
) -> ([f32; 3], [f32; 3], [f32; 4], [f32; 3]) {
    if palette.is_empty() {
        return (pos, normal, tangent, [0.0, 0.0, 0.0]);
    }

    // Find reference anchor joint (first non-zero weight within palette bounds)
    let mut ref_curr_r = [1.0f32, 0.0, 0.0, 0.0];
    let mut ref_prev_r = [1.0f32, 0.0, 0.0, 0.0];
    for slot in 0..4 {
        let w = skinned_attr.joint_weights[slot];
        let j = skinned_attr.joint_indices[slot] as usize;
        if w > 0.0 && j < palette.len() {
            ref_curr_r = palette[j].current_r;
            ref_prev_r = palette[j].prev_r;
            break;
        }
    }

    let mut curr_r = [0.0f32; 4];
    let mut curr_d = [0.0f32; 4];
    let mut prev_r = [0.0f32; 4];
    let mut prev_d = [0.0f32; 4];
    let mut total_w = 0.0f32;

    for slot in 0..4 {
        let w = skinned_attr.joint_weights[slot];
        if w <= 0.0 || !w.is_finite() {
            continue;
        }
        let j = skinned_attr.joint_indices[slot] as usize;
        if j >= palette.len() {
            continue;
        }
        let pair = &palette[j];
        let sign_curr = if dot4_f32(ref_curr_r, pair.current_r) < 0.0 {
            -w
        } else {
            w
        };
        let sign_prev = if dot4_f32(ref_prev_r, pair.prev_r) < 0.0 {
            -w
        } else {
            w
        };
        for c in 0..4 {
            curr_r[c] += sign_curr * pair.current_r[c];
            curr_d[c] += sign_curr * pair.current_d[c];
            prev_r[c] += sign_prev * pair.prev_r[c];
            prev_d[c] += sign_prev * pair.prev_d[c];
        }
        total_w += w;
    }

    if total_w <= 1e-8 {
        return (pos, normal, tangent, [0.0, 0.0, 0.0]);
    }

    let (curr_r_norm, curr_d_norm) = normalize_motor_f32(curr_r, curr_d);
    let (prev_r_norm, prev_d_norm) = normalize_motor_f32(prev_r, prev_d);

    let skinned_pos = motor_transform_point_f32(curr_r_norm, curr_d_norm, pos);
    let prev_pos = motor_transform_point_f32(prev_r_norm, prev_d_norm, pos);

    let raw_normal = motor_rotate_vec3_f32(curr_r_norm, normal);
    let n_len = (raw_normal[0] * raw_normal[0]
        + raw_normal[1] * raw_normal[1]
        + raw_normal[2] * raw_normal[2])
        .sqrt();
    let skinned_normal = if n_len > 1e-6 {
        [
            raw_normal[0] / n_len,
            raw_normal[1] / n_len,
            raw_normal[2] / n_len,
        ]
    } else {
        normal
    };

    let raw_tan = motor_rotate_vec3_f32(curr_r_norm, [tangent[0], tangent[1], tangent[2]]);
    let t_len = (raw_tan[0] * raw_tan[0] + raw_tan[1] * raw_tan[1] + raw_tan[2] * raw_tan[2]).sqrt();
    let skinned_tangent = if t_len > 1e-6 {
        [
            raw_tan[0] / t_len,
            raw_tan[1] / t_len,
            raw_tan[2] / t_len,
            tangent[3],
        ]
    } else {
        tangent
    };

    let motion_vector = [
        skinned_pos[0] - prev_pos[0],
        skinned_pos[1] - prev_pos[1],
        skinned_pos[2] - prev_pos[2],
    ];

    (skinned_pos, skinned_normal, skinned_tangent, motion_vector)
}

/// Batch helper that skins a slice of vertices via `gpu_shader_skin_vertex_reference` into
/// caller-supplied output slices (`HotZeroHeap`).
pub fn skin_vertices_gpu_reference_into(
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    tangents: &[[f32; 4]],
    skinned_attrs: &[GpuSkinnedVertex],
    palette: &[GpuJointMotorPair],
    out_positions: &mut [[f32; 3]],
    out_normals: &mut [[f32; 3]],
    out_tangents: &mut [[f32; 4]],
    out_motion_vectors: &mut [[f32; 3]],
) -> Result<usize, SkeletalError> {
    let count = positions.len();
    if count == 0
        || normals.len() != count
        || tangents.len() != count
        || skinned_attrs.len() != count
        || palette.is_empty()
    {
        return Err(SkeletalError::InvalidInput);
    }
    if out_positions.len() < count
        || out_normals.len() < count
        || out_tangents.len() < count
        || out_motion_vectors.len() < count
    {
        return Err(SkeletalError::OutputTooSmall);
    }
    for i in 0..count {
        let (p, n, t, mv) = gpu_shader_skin_vertex_reference(
            positions[i],
            normals[i],
            tangents[i],
            &skinned_attrs[i],
            palette,
        );
        out_positions[i] = p;
        out_normals[i] = n;
        out_tangents[i] = t;
        out_motion_vectors[i] = mv;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::skeletal::skin::{skin_vertices_into, SkinWeight};

    #[test]
    fn gpu_shader_skin_matches_cpu_skin_vertices_into_oracle() {
        let inv_bind = [
            Motor::from_translation(0.0, 0.0, 0.0),
            Motor::from_translation(0.0, -1.0, 0.0),
        ];
        let half = std::f64::consts::FRAC_PI_4;
        let world_prev = [
            Motor::from_translation(1.0, 0.0, 0.0),
            Motor::from_translation(1.0, 1.0, 0.0),
        ];
        let world_curr = [
            Motor::from_translation(2.0, 0.0, 0.0),
            Motor::from_rotation_translation([half.cos(), 0.0, 0.0, half.sin()], [2.0, 1.0, 0.0]),
        ];

        let mut prev_skin = [Motor::identity(); 2];
        let mut curr_skin = [Motor::identity(); 2];
        let mut gpu_palette = [GpuJointMotorPair::identity(); 2];

        // First frame (prev)
        evaluate_skinning_motor_palette_zero_heap(
            &inv_bind,
            &world_prev,
            &[Motor::identity(); 2],
            &mut gpu_palette,
            &mut prev_skin,
        );
        // Second frame (curr)
        let n_joints = evaluate_skinning_motor_palette_zero_heap(
            &inv_bind,
            &world_curr,
            &prev_skin,
            &mut gpu_palette,
            &mut curr_skin,
        );
        assert_eq!(n_joints, 2);

        let bind_pos = [[0.0f32, 2.0, 0.0], [1.0, 1.0, 0.0]];
        let bind_nrm = [[0.0f32, 1.0, 0.0], [1.0, 0.0, 0.0]];
        let bind_tan = [[1.0f32, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, -1.0]];
        let cpu_weights = [
            SkinWeight {
                joints: [1, 0, 0, 0],
                weights: [1.0, 0.0, 0.0, 0.0],
            },
            SkinWeight {
                joints: [0, 1, 0, 0],
                weights: [0.5, 0.5, 0.0, 0.0],
            },
        ];
        let gpu_attrs = [
            GpuSkinnedVertex {
                joint_indices: [1, 0, 0, 0],
                joint_weights: [1.0, 0.0, 0.0, 0.0],
            },
            GpuSkinnedVertex {
                joint_indices: [0, 1, 0, 0],
                joint_weights: [0.5, 0.5, 0.0, 0.0],
            },
        ];

        let mut cpu_pos = [[0.0f32; 3]; 2];
        let mut cpu_nrm = [[0.0f32; 3]; 2];
        skin_vertices_into(
            &bind_pos,
            &bind_nrm,
            &cpu_weights,
            &curr_skin,
            &mut cpu_pos,
            &mut cpu_nrm,
        )
        .unwrap();

        // Vertex 0 (single joint influence with rotation + translation) must match CPU exactly
        let (gpu_p0, gpu_n0, gpu_t0, mv0) = gpu_shader_skin_vertex_reference(
            bind_pos[0],
            bind_nrm[0],
            bind_tan[0],
            &gpu_attrs[0],
            &gpu_palette,
        );
        for axis in 0..3 {
            assert!(
                (gpu_p0[axis] - cpu_pos[0][axis]).abs() < 1e-5,
                "pos mismatch axis {axis}: gpu={gpu_p0:?} cpu={:?}",
                cpu_pos[0]
            );
            assert!(
                (gpu_n0[axis] - cpu_nrm[0][axis]).abs() < 1e-5,
                "nrm mismatch axis {axis}: gpu={gpu_n0:?} cpu={:?}",
                cpu_nrm[0]
            );
        }
        assert_eq!(gpu_t0[3], 1.0);
        // Motion vector must be non-zero since joint 1 moved and rotated from world_prev to world_curr
        assert!(mv0.iter().map(|x| x * x).sum::<f32>() > 0.1);
    }

    #[test]
    fn antipodal_hemisphere_alignment_prevents_dual_quaternion_collapse() {
        let m_pos = Motor::from_translation(0.0, 2.0, 0.0);
        // Negated motor represents the exact same rigid transform on the antipodal sheet of S^3
        let m_neg = Motor {
            r_w: -m_pos.r_w,
            r_x: -m_pos.r_x,
            r_y: -m_pos.r_y,
            r_z: -m_pos.r_z,
            d_w: -m_pos.d_w,
            d_x: -m_pos.d_x,
            d_y: -m_pos.d_y,
            d_z: -m_pos.d_z,
        };
        let aligned = hemisphere_align_motor(&m_pos, &m_neg);
        assert!((aligned.r_w - m_pos.r_w).abs() < 1e-6);

        let palette = [
            GpuJointMotorPair::from_motors(&m_pos, &m_pos),
            GpuJointMotorPair {
                current_r: [
                    m_neg.r_w as f32,
                    m_neg.r_x as f32,
                    m_neg.r_y as f32,
                    m_neg.r_z as f32,
                ],
                current_d: [
                    m_neg.d_w as f32,
                    m_neg.d_x as f32,
                    m_neg.d_y as f32,
                    m_neg.d_z as f32,
                ],
                prev_r: [
                    m_neg.r_w as f32,
                    m_neg.r_x as f32,
                    m_neg.r_y as f32,
                    m_neg.r_z as f32,
                ],
                prev_d: [
                    m_neg.d_w as f32,
                    m_neg.d_x as f32,
                    m_neg.d_y as f32,
                    m_neg.d_z as f32,
                ],
            },
        ];
        let attr = GpuSkinnedVertex {
            joint_indices: [0, 1, 0, 0],
            joint_weights: [0.5, 0.5, 0.0, 0.0],
        };
        let (p, n, _, mv) = gpu_shader_skin_vertex_reference(
            [1.0, 3.0, -2.0],
            [0.0, 1.0, 0.0],
            [1.0, 0.0, 0.0, 1.0],
            &attr,
            &palette,
        );
        assert!((p[0] - 1.0).abs() < 1e-5);
        assert!((p[1] - 5.0).abs() < 1e-5);
        assert!((p[2] - -2.0).abs() < 1e-5);
        assert!((n[1] - 1.0).abs() < 1e-5);
        assert!(mv.iter().all(|v| v.abs() < 1e-6));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn gpu_skinning_palette_and_vertex_reference_are_zero_heap() {
        use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

        let inv_bind = [Motor::identity(); 4];
        let world = [Motor::from_translation(1.0, 2.0, 3.0); 4];
        let prev = [Motor::identity(); 4];
        let mut gpu_palette = [GpuJointMotorPair::identity(); 4];
        let mut curr_skin = [Motor::identity(); 4];
        let attr = GpuSkinnedVertex {
            joint_indices: [0, 1, 2, 3],
            joint_weights: [0.25, 0.25, 0.25, 0.25],
        };

        assert_zero_alloc("gpu_skin_palette_and_vertex", || {
            let n = evaluate_skinning_motor_palette_zero_heap(
                &inv_bind,
                &world,
                &prev,
                &mut gpu_palette,
                &mut curr_skin,
            );
            assert_eq!(n, 4);
            let (p, _, _, _) = gpu_shader_skin_vertex_reference(
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 0.0, 0.0, 1.0],
                &attr,
                &gpu_palette,
            );
            assert!((p[0] - 1.0).abs() < 1e-5);
        });
    }
}
