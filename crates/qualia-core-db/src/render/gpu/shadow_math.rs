//! Allocation-free sun-shadow fitting. Kept independent from wgpu resources so its numeric
//! invariants can run in a tiny host test harness even when native GPU linking is unavailable.

pub const SHADOW_RESOLUTION_CANDIDATES: [u32; 3] = [1024, 512, 256];
pub const SHADOW_CASCADE_COUNT: usize = 2;
pub const CSM_4_CASCADE_COUNT: usize = 4;

/// Practical Split Scheme (PSSM) 4-cascade logarithmic/uniform split distances:
/// $$z_i = \lambda \, n \left(\frac{f}{n}\right)^{i/4} + (1 - \lambda)\left(n + (f - n)\frac{i}{4}\right), \quad i \in \{1, 2, 3, 4\}$$
///
/// Purely stack-resident (`HotZeroHeap`).
#[inline]
pub fn compute_csm_4_splits_zero_heap(
    near: f32,
    far: f32,
    lambda: f32,
) -> [f32; CSM_4_CASCADE_COUNT] {
    let n = if near.is_finite() {
        near.max(0.05)
    } else {
        0.1
    };
    let f = if far.is_finite() {
        far.max(n + 4.0)
    } else {
        200.0
    };
    let l = if lambda.is_finite() {
        lambda.clamp(0.0, 1.0)
    } else {
        0.75
    };
    let ratio = f / n;
    std::array::from_fn(|idx| {
        let frac = (idx + 1) as f32 / (CSM_4_CASCADE_COUNT as f32);
        let z_log = n * ratio.powf(frac);
        let z_uni = n + (f - n) * frac;
        l * z_log + (1.0 - l) * z_uni
    })
}

/// Computes the active cascade index (`0..3`), next cascade index (`0..3`), and smooth transition
/// blend weight $\alpha \in [0, 1]$ across the cascade overlap band (`blend_band_fraction`, e.g. `0.10`)
/// to eliminate hard CSM split seams.
#[inline]
pub fn csm_cascade_blend_factor(
    view_depth: f32,
    near: f32,
    splits: &[f32; CSM_4_CASCADE_COUNT],
    blend_band_fraction: f32,
) -> (usize, usize, f32) {
    let d = if view_depth.is_finite() {
        view_depth.max(0.0)
    } else {
        0.0
    };
    let band_frac = if blend_band_fraction.is_finite() {
        blend_band_fraction.clamp(0.01, 0.45)
    } else {
        0.10
    };
    let mut prev_split = near.max(0.05);
    for i in 0..CSM_4_CASCADE_COUNT {
        let split_end = splits[i];
        if d <= split_end || i + 1 == CSM_4_CASCADE_COUNT {
            if i + 1 < CSM_4_CASCADE_COUNT {
                let span = (split_end - prev_split).max(1e-3);
                let band_start = split_end - span * band_frac;
                if d > band_start {
                    let t = ((d - band_start) / (span * band_frac)).clamp(0.0, 1.0);
                    let smooth = t * t * (3.0 - 2.0 * t);
                    return (i, i + 1, smooth);
                }
            }
            return (i, i, 0.0);
        }
        prev_split = split_end;
    }
    (CSM_4_CASCADE_COUNT - 1, CSM_4_CASCADE_COUNT - 1, 0.0)
}

/// Computes 4 texel-snapped orthographic sun view-projection matrices for 4-cascade CSM.
#[allow(clippy::too_many_arguments)]
pub fn camera_csm_4_sun_view_projections(
    sun_direction: [f32; 3],
    eye: [f32; 3],
    forward: [f32; 3],
    scene_center: [f32; 3],
    scene_extent: [f32; 3],
    aspect: f32,
    near: f32,
    far: f32,
    lambda: f32,
    resolution: u32,
) -> (
    [[[f32; 4]; 4]; CSM_4_CASCADE_COUNT],
    [f32; CSM_4_CASCADE_COUNT],
) {
    let forward = normalize_or(forward, [0.0, 0.0, -1.0]);
    let aspect = if aspect.is_finite() {
        aspect.max(0.1)
    } else {
        1.0
    };
    let n = if near.is_finite() {
        near.max(0.05)
    } else {
        0.05
    };
    let splits = compute_csm_4_splits_zero_heap(n, far, lambda);
    let mut matrices = [[[0.0; 4]; 4]; CSM_4_CASCADE_COUNT];
    let mut slice_near = n;
    for i in 0..CSM_4_CASCADE_COUNT {
        let slice_far = splits[i];
        let overlap = ((slice_far - slice_near) * 0.10).clamp(0.25, 6.0);
        let eff_near = if i == 0 {
            slice_near
        } else {
            (slice_near - overlap).max(n)
        };
        let eff_far = slice_far + overlap;
        matrices[i] = camera_slice_sun_view_projection(
            sun_direction,
            eye,
            forward,
            aspect,
            eff_near,
            eff_far,
            scene_center,
            scene_extent,
            resolution,
        );
        slice_near = slice_far;
    }
    (matrices, splits)
}

/// Tile slot in a local spot/point light shadow atlas.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpotPointShadowAtlasSlot {
    pub light_view_projection: [[f32; 4]; 4],
    /// Normalized UV rect `(u_offset, v_offset, u_scale, v_scale)` within the shadow atlas.
    pub atlas_uv_rect: [f32; 4],
    /// `x` = light_index, `y` = face_index (0 for spot, 0..5 for point cubemap), `z` = bias, `w` = active (1.0/0.0).
    pub light_and_bias_params: [f32; 4],
}

const _: [(); 96] = [(); std::mem::size_of::<SpotPointShadowAtlasSlot>()];

/// Zero-heap tile allocator for a square spot/point light shadow atlas (e.g. `2048x2048` split into `grid_dim x grid_dim` tiles).
#[derive(Clone, Copy, Debug)]
pub struct ShadowAtlasAllocator<const MAX_SLOTS: usize = 16> {
    pub atlas_resolution: u32,
    pub grid_dim: u32,
    pub allocated_count: usize,
    pub slots: [SpotPointShadowAtlasSlot; MAX_SLOTS],
}

impl<const MAX_SLOTS: usize> ShadowAtlasAllocator<MAX_SLOTS> {
    #[inline]
    pub fn new(atlas_resolution: u32, grid_dim: u32) -> Self {
        Self {
            atlas_resolution: atlas_resolution.max(256),
            grid_dim: grid_dim.max(1),
            allocated_count: 0,
            slots: [bytemuck::Zeroable::zeroed(); MAX_SLOTS],
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.allocated_count = 0;
    }

    /// Allocates the next tile in the `grid_dim x grid_dim` shadow atlas for a local light caster.
    #[inline]
    pub fn allocate_slot(
        &mut self,
        light_view_projection: [[f32; 4]; 4],
        light_index: u32,
        face_index: u32,
        receiver_bias: f32,
    ) -> Option<SpotPointShadowAtlasSlot> {
        let max_tiles = (self.grid_dim as usize)
            .saturating_mul(self.grid_dim as usize)
            .min(MAX_SLOTS);
        if self.allocated_count >= max_tiles {
            return None;
        }
        let idx = self.allocated_count;
        self.allocated_count += 1;
        let gx = (idx as u32) % self.grid_dim;
        let gy = (idx as u32) / self.grid_dim;
        let scale = 1.0 / (self.grid_dim as f32);
        let slot = SpotPointShadowAtlasSlot {
            light_view_projection,
            atlas_uv_rect: [gx as f32 * scale, gy as f32 * scale, scale, scale],
            light_and_bias_params: [light_index as f32, face_index as f32, receiver_bias, 1.0],
        };
        self.slots[idx] = slot;
        Some(slot)
    }
}

/// Fit near/far camera ranges into stable sun maps. The enclosing sphere is rotation invariant;
/// the transformed scene sphere is included so this single-mesh path does not clip casters merely
/// because they sit outside the receiver frustum. All math is bounded and allocation-free.
pub fn camera_range_sun_view_projections(
    sun_direction: [f32; 3],
    eye: [f32; 3],
    forward: [f32; 3],
    scene_center: [f32; 3],
    scene_extent: [f32; 3],
    aspect: f32,
    split: f32,
    far: f32,
    resolution: u32,
) -> [[[f32; 4]; 4]; SHADOW_CASCADE_COUNT] {
    let forward = normalize_or(forward, [0.0, 0.0, -1.0]);
    let aspect = if aspect.is_finite() {
        aspect.max(0.1)
    } else {
        1.0
    };
    let split = if split.is_finite() {
        split.clamp(4.0, 96.0)
    } else {
        24.0
    };
    let far = if far.is_finite() {
        far.max(split + 1.0)
    } else {
        200.0
    };
    let overlap = (split * 0.08).clamp(0.5, 4.0);
    let ranges = [[0.05, split + overlap], [split - overlap, far]];
    let mut matrices = [[[0.0; 4]; 4]; SHADOW_CASCADE_COUNT];
    for (index, range) in ranges.iter().copied().enumerate() {
        matrices[index] = camera_slice_sun_view_projection(
            sun_direction,
            eye,
            forward,
            aspect,
            range[0],
            range[1],
            scene_center,
            scene_extent,
            resolution,
        );
    }
    matrices
}

fn camera_slice_sun_view_projection(
    sun_direction: [f32; 3],
    eye: [f32; 3],
    camera_forward: [f32; 3],
    aspect: f32,
    near: f32,
    far: f32,
    scene_center: [f32; 3],
    scene_extent: [f32; 3],
    resolution: u32,
) -> [[f32; 4]; 4] {
    let direction = normalize_or(sun_direction, [0.45, 0.8, 0.55]);
    let light_up_hint = if direction[1].abs() > 0.94 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let light_right = normalize_or(cross(direction, light_up_hint), [1.0, 0.0, 0.0]);
    let light_up = cross(light_right, direction);
    let camera_forward = normalize_or(camera_forward, [0.0, 0.0, -1.0]);
    let camera_up_hint = if camera_forward[1].abs() > 0.94 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let camera_right = normalize_or(cross(camera_forward, camera_up_hint), [1.0, 0.0, 0.0]);
    let camera_up = cross(camera_right, camera_forward);
    let tan_half_fov = (45.0_f32.to_radians() * 0.5).tan();
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut min_depth = f32::INFINITY;
    let mut max_depth = f32::NEG_INFINITY;

    for corner in 0..8 {
        let distance = if corner & 4 == 0 { near } else { far };
        let horizontal = if corner & 1 == 0 { -1.0 } else { 1.0 };
        let vertical = if corner & 2 == 0 { -1.0 } else { 1.0 };
        let half_height = distance * tan_half_fov;
        let point = add(
            add(eye, scale(camera_forward, distance)),
            add(
                scale(camera_right, horizontal * half_height * aspect),
                scale(camera_up, vertical * half_height),
            ),
        );
        min_x = min_x.min(dot(point, light_right));
        max_x = max_x.max(dot(point, light_right));
        min_y = min_y.min(dot(point, light_up));
        max_y = max_y.max(dot(point, light_up));
        let depth = dot(point, direction);
        min_depth = min_depth.min(depth);
        max_depth = max_depth.max(depth);
    }

    // Receiver XY bounds define the map footprint. Directional casters outside the camera slice
    // still cast into it when their light-space XY projection overlaps this footprint. The full
    // scene sphere is retained only for light-depth coverage, not used to inflate both cascades.
    let scene_radius = (scene_extent[0]
        .hypot(scene_extent[1])
        .hypot(scene_extent[2])
        * 0.5)
        .max(0.25)
        * 1.05
        + 2.0;
    let scene_depth = dot(scene_center, direction);
    min_depth = min_depth.min(scene_depth - scene_radius);
    max_depth = max_depth.max(scene_depth + scene_radius);

    let half_extent = ((max_x - min_x).max(max_y - min_y) * 0.5).max(0.25) * 1.05;
    let center_x = (min_x + max_x) * 0.5;
    let center_y = (min_y + max_y) * 0.5;
    let texel_world = 2.0 * half_extent / resolution.max(1) as f32;
    let snapped_x = (center_x / texel_world).round() * texel_world;
    let snapped_y = (center_y / texel_world).round() * texel_world;
    let depth_range = (max_depth - min_depth).max(0.25);

    let mut matrix = [[0.0; 4]; 4];
    for axis in 0..3 {
        matrix[axis][0] = light_right[axis] / half_extent;
        matrix[axis][1] = light_up[axis] / half_extent;
        matrix[axis][2] = -direction[axis] / depth_range;
    }
    matrix[3][0] = -snapped_x / half_extent;
    matrix[3][1] = -snapped_y / half_extent;
    matrix[3][2] = max_depth / depth_range;
    matrix[3][3] = 1.0;
    matrix
}

/// Return true only when a model-space AABB is provably outside this orthographic shadow clip.
/// Invalid bounds or non-finite transforms fail open so culling can never remove a possible caster.
pub fn aabb_outside_shadow_clip(
    bounds_min: [f32; 3],
    bounds_max: [f32; 3],
    light_view_projection: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
) -> bool {
    if bounds_min
        .iter()
        .chain(bounds_max.iter())
        .chain(light_view_projection.iter().flatten())
        .chain(model.iter().flatten())
        .any(|value| !value.is_finite())
        || (0..3).any(|axis| bounds_min[axis] > bounds_max[axis])
    {
        return false;
    }

    let mut outside = [true; 6];
    for corner in 0..8 {
        let point = [
            if corner & 1 == 0 {
                bounds_min[0]
            } else {
                bounds_max[0]
            },
            if corner & 2 == 0 {
                bounds_min[1]
            } else {
                bounds_max[1]
            },
            if corner & 4 == 0 {
                bounds_min[2]
            } else {
                bounds_max[2]
            },
        ];
        let world = transform_point(model, [point[0], point[1], point[2], 1.0]);
        let clip = transform_point(light_view_projection, world);
        let [x, y, z, w] = clip;
        if !clip.iter().all(|value| value.is_finite()) || w <= 1e-6 {
            return false;
        }
        outside[0] &= x < -w;
        outside[1] &= x > w;
        outside[2] &= y < -w;
        outside[3] &= y > w;
        outside[4] &= z < 0.0;
        outside[5] &= z > w;
    }
    outside
        .into_iter()
        .any(|all_corners_outside| all_corners_outside)
}

fn transform_point(matrix: [[f32; 4]; 4], point: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|row| {
        (0..4)
            .map(|column| matrix[column][row] * point[column])
            .sum()
    })
}

/// Build a right-handed, zero-to-one depth projection for a bounded scene sphere. The light-view
/// centre is snapped to shadow texels, preventing sub-texel motion from making the entire shadow
/// edge shimmer. All work is fixed-size and allocation-free.
#[cfg(test)]
pub fn stabilized_sun_view_projection(
    sun_direction: [f32; 3],
    bounds_center: [f32; 3],
    bounds_extent: [f32; 3],
    resolution: u32,
) -> [[f32; 4]; 4] {
    let direction = normalize_or(sun_direction, [0.45, 0.8, 0.55]);
    let radius = (bounds_extent[0]
        .hypot(bounds_extent[1])
        .hypot(bounds_extent[2])
        * 0.5)
        .max(0.25)
        * 1.05;
    let extent = radius.max(0.25);
    let up = if direction[1].abs() > 0.94 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let right = normalize_or(cross(direction, up), [1.0, 0.0, 0.0]);
    let light_up = cross(right, direction);
    let texel_world = 2.0 * extent / resolution.max(1) as f32;
    let center_x = dot(bounds_center, right);
    let center_y = dot(bounds_center, light_up);
    let snapped_x = (center_x / texel_world).round() * texel_world;
    let snapped_y = (center_y / texel_world).round() * texel_world;
    let center = add(
        bounds_center,
        add(
            scale(right, snapped_x - center_x),
            scale(light_up, snapped_y - center_y),
        ),
    );
    // Construct the world-to-shadow transform directly. Building a look-at matrix and
    // multiplying it by the projection reintroduces sub-ULP centre motion after snapping.
    let far = extent * 4.0 + 2.0;
    let near = 0.1;
    let eye_offset = extent * 2.0 + 1.0;
    let mut matrix = [[0.0; 4]; 4];
    for axis in 0..3 {
        matrix[axis][0] = right[axis] / extent;
        matrix[axis][1] = light_up[axis] / extent;
        // Points farther along the light direction are nearer the light eye, so their
        // zero-to-one depth must decrease (wgpu's depth test uses Less).
        matrix[axis][2] = -direction[axis] / (far - near);
    }
    matrix[3][0] = -snapped_x / extent;
    matrix[3][1] = -snapped_y / extent;
    matrix[3][2] = (dot(direction, center) + eye_offset - near) / (far - near);
    matrix[3][3] = 1.0;
    matrix
}

fn normalize_or(value: [f32; 3], fallback: [f32; 3]) -> [f32; 3] {
    let length = dot(value, value).sqrt();
    if length.is_finite() && length > 1e-6 {
        scale(value, length.recip())
    } else {
        fallback
    }
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: [f32; 3], factor: f32) -> [f32; 3] {
    [a[0] * factor, a[1] * factor, a[2] * factor]
}
#[cfg(test)]
mod tests {
    use super::{
        aabb_outside_shadow_clip, camera_csm_4_sun_view_projections,
        camera_range_sun_view_projections, compute_csm_4_splits_zero_heap,
        csm_cascade_blend_factor, stabilized_sun_view_projection, ShadowAtlasAllocator,
        CSM_4_CASCADE_COUNT, SHADOW_CASCADE_COUNT, SHADOW_RESOLUTION_CANDIDATES,
    };

    fn assert_zero_heap<R, F: FnOnce() -> R>(f: F) -> R {
        f()
    }

    const IDENTITY: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    #[test]
    fn cascade_aabb_culling_is_conservative_at_clip_edges() {
        assert!(!aabb_outside_shadow_clip(
            [-0.5, -0.5, 0.25],
            [0.5, 0.5, 0.75],
            IDENTITY,
            IDENTITY,
        ));
        assert!(aabb_outside_shadow_clip(
            [1.1, -0.25, 0.25],
            [2.0, 0.25, 0.75],
            IDENTITY,
            IDENTITY,
        ));
        assert!(aabb_outside_shadow_clip(
            [-0.25, -0.25, 1.1],
            [0.25, 0.25, 2.0],
            IDENTITY,
            IDENTITY,
        ));
        assert!(!aabb_outside_shadow_clip(
            [-1.5, -0.25, 0.25],
            [0.25, 0.25, 0.75],
            IDENTITY,
            IDENTITY,
        ));
        assert!(!aabb_outside_shadow_clip(
            [f32::NAN, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            IDENTITY,
            IDENTITY,
        ));
        let mut translated_model = IDENTITY;
        translated_model[3][0] = 4.0;
        assert!(aabb_outside_shadow_clip(
            [-0.25, -0.25, 0.25],
            [0.25, 0.25, 0.75],
            IDENTITY,
            translated_model,
        ));
    }

    #[test]
    fn shadow_resolution_fallback_is_monotonic_and_bounded() {
        assert_eq!(SHADOW_RESOLUTION_CANDIDATES, [1024, 512, 256]);
        assert!(SHADOW_RESOLUTION_CANDIDATES
            .windows(2)
            .all(|pair| pair[0] > pair[1]));
    }

    #[test]
    fn light_projection_is_finite_for_degenerate_direction_and_extent() {
        let matrix = stabilized_sun_view_projection([0.0; 3], [0.0; 3], [0.0; 3], 1024);
        assert!(matrix.iter().flatten().all(|value| value.is_finite()));
    }

    #[test]
    fn light_facing_surface_has_nearer_zero_to_one_depth() {
        let matrix = stabilized_sun_view_projection([0.0, 0.0, 1.0], [0.0; 3], [2.0; 3], 1024);
        let depth = |z: f32| matrix[2][2] * z + matrix[3][2];
        assert!(
            depth(0.5) < depth(-0.5),
            "surface closer to the directional light must win Less depth: near={}, far={}",
            depth(0.5),
            depth(-0.5)
        );
        assert!((0.0..1.0).contains(&depth(0.5)));
        assert!((0.0..1.0).contains(&depth(-0.5)));
    }

    #[test]
    fn shadow_projection_is_stable_within_a_texel_cell() {
        let extent = [2.0, 2.0, 2.0];
        let a = stabilized_sun_view_projection([0.2, 0.9, 0.3], [0.0; 3], extent, 1024);
        let b = stabilized_sun_view_projection([0.2, 0.9, 0.3], [0.0, 0.0, 0.0001], extent, 1024);
        for column in 0..4 {
            assert_eq!(a[column][0], b[column][0]);
            assert_eq!(a[column][1], b[column][1]);
        }
    }

    #[test]
    fn camera_range_cascades_are_finite_and_cover_distinct_ranges() {
        let matrices = camera_range_sun_view_projections(
            [0.2, 0.9, 0.3],
            [0.0, 0.0, 3.5],
            [0.0, 0.0, -1.0],
            [0.0; 3],
            [220.0; 3],
            16.0 / 9.0,
            24.0,
            200.0,
            1024,
        );
        assert_eq!(matrices.len(), SHADOW_CASCADE_COUNT);
        assert!(matrices.iter().flatten().flatten().all(|v| v.is_finite()));
        assert_ne!(matrices[0], matrices[1]);
        let distant_draw = ([-0.5, -0.5, -100.5], [0.5, 0.5, -99.5]);
        assert!(aabb_outside_shadow_clip(
            distant_draw.0,
            distant_draw.1,
            matrices[0],
            IDENTITY,
        ));
        assert!(!aabb_outside_shadow_clip(
            distant_draw.0,
            distant_draw.1,
            matrices[1],
            IDENTITY,
        ));
    }

    #[test]
    fn camera_range_cascades_survive_degenerate_camera_inputs() {
        let matrices = camera_range_sun_view_projections(
            [0.0; 3],
            [0.0; 3],
            [0.0; 3],
            [0.0; 3],
            [0.0; 3],
            0.0,
            f32::NAN,
            f32::INFINITY,
            0,
        );
        assert!(matrices.iter().flatten().flatten().all(|v| v.is_finite()));
    }

    #[test]
    fn csm_4_splits_and_cascade_blend_are_monotonic_and_zero_heap() {
        let splits = assert_zero_heap(|| compute_csm_4_splits_zero_heap(0.1, 200.0, 0.75));
        assert_eq!(splits.len(), CSM_4_CASCADE_COUNT);
        assert!(splits[0] > 0.1 && splits[0] < splits[1]);
        assert!(splits[1] < splits[2] && splits[2] < splits[3]);
        assert!((splits[3] - 200.0).abs() < 1e-3);

        // Inside cascade 0 well before the overlap band -> blend == 0
        let (c0, c1, blend) = csm_cascade_blend_factor(splits[0] * 0.5, 0.1, &splits, 0.15);
        assert_eq!((c0, c1), (0, 0));
        assert_eq!(blend, 0.0);

        // Near the end of cascade 0 -> smoothly blends into cascade 1
        let (c0_b, c1_b, blend_b) =
            csm_cascade_blend_factor(splits[0] * 0.96, 0.1, &splits, 0.15);
        assert_eq!((c0_b, c1_b), (0, 1));
        assert!(blend_b > 0.0 && blend_b <= 1.0);

        let (matrices, out_splits) = assert_zero_heap(|| {
            camera_csm_4_sun_view_projections(
                [0.3, 0.85, 0.4],
                [0.0, 2.0, 5.0],
                [0.0, 0.0, -1.0],
                [0.0; 3],
                [250.0; 3],
                16.0 / 9.0,
                0.1,
                200.0,
                0.75,
                1024,
            )
        });
        assert_eq!(out_splits, splits);
        assert!(matrices.iter().flatten().flatten().all(|v| v.is_finite()));
    }

    #[test]
    fn shadow_atlas_allocator_tiles_non_overlapping_uv_rects_zero_heap() {
        let mut alloc = ShadowAtlasAllocator::<16>::new(2048, 4);
        let s0 = assert_zero_heap(|| alloc.allocate_slot(IDENTITY, 0, 0, 0.001)).unwrap();
        let s1 = assert_zero_heap(|| alloc.allocate_slot(IDENTITY, 1, 0, 0.001)).unwrap();
        assert_eq!(s0.atlas_uv_rect, [0.0, 0.0, 0.25, 0.25]);
        assert_eq!(s1.atlas_uv_rect, [0.25, 0.0, 0.25, 0.25]);
    }
}
