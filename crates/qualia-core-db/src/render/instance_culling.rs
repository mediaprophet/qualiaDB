//! Deterministic, caller-buffered CPU frustum selection for reusable mesh instances.
//!
//! This is the portable first-stage visibility path. It does not allocate, reorder instances, or
//! infer semantic identity from a compacted draw slot: returned `u32`s are stable source indices
//! into the caller's instance slice. Invalid or numerically uncertain bounds fail open.

/// Version of the packed native/WASM instance record ABI.
pub const GPU_INSTANCE_RECORD_ABI_VERSION: u32 = 2;
pub const MAX_GPU_MESH_INSTANCES: usize = 10_000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundedInstance {
    /// Local-space AABB. Bounds must already include authored deformation margins.
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    /// Column-major local-to-world matrix (the same convention as WGSL `mat4x4<f32>`).
    pub world_from_local: [[f32; 4]; 4],
}

/// Stable upload record for mesh instances shared by native and WASM WebGPU paths.
///
/// The normal matrix is precomputed during cold scene construction/update, avoiding an inverse in
/// the vertex shader. The two identity words encode the low then high halves of a `u64`, avoiding
/// a shader-side `u64` feature requirement. The 128-byte record keeps each matrix on a 16-byte
/// boundary in the WGSL storage-buffer array.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuInstanceRecord {
    pub world_from_local: [[f32; 4]; 4],
    /// Column-major inverse-transpose of the instance transform's upper-left 3x3.
    pub normal_from_local: [[f32; 4]; 3],
    pub semantic_id_words: [u32; 2],
    /// Sign of the transform determinant; flips tangent-frame handedness under reflection.
    pub orientation_sign: f32,
    pub _padding: u32,
}

impl GpuInstanceRecord {
    pub fn new(world_from_local: [[f32; 4]; 4], semantic_id: u64) -> Self {
        let columns = [
            [
                world_from_local[0][0],
                world_from_local[0][1],
                world_from_local[0][2],
            ],
            [
                world_from_local[1][0],
                world_from_local[1][1],
                world_from_local[1][2],
            ],
            [
                world_from_local[2][0],
                world_from_local[2][1],
                world_from_local[2][2],
            ],
        ];
        let cross = |a: [f32; 3], b: [f32; 3]| {
            [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ]
        };
        let cofactors = [
            cross(columns[1], columns[2]),
            cross(columns[2], columns[0]),
            cross(columns[0], columns[1]),
        ];
        let determinant = columns[0][0] * cofactors[0][0]
            + columns[0][1] * cofactors[0][1]
            + columns[0][2] * cofactors[0][2];
        let reciprocal = if determinant.is_finite() && determinant != 0.0 {
            determinant.recip()
        } else {
            0.0
        };
        let normal_from_local = std::array::from_fn(|column| {
            [
                cofactors[column][0] * reciprocal,
                cofactors[column][1] * reciprocal,
                cofactors[column][2] * reciprocal,
                0.0,
            ]
        });
        Self {
            world_from_local,
            normal_from_local,
            semantic_id_words: [semantic_id as u32, (semantic_id >> 32) as u32],
            orientation_sign: if determinant < 0.0 { -1.0 } else { 1.0 },
            _padding: 0,
        }
    }

    pub const fn semantic_id(self) -> u64 {
        self.semantic_id_words[0] as u64 | ((self.semantic_id_words[1] as u64) << 32)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisibilityListError {
    /// The output needs one slot per input because malformed inputs conservatively remain visible.
    OutputTooSmall { required: usize, available: usize },
    /// Source indices are returned as `u32` and cannot represent this slice length.
    TooManyInstances { count: usize },
    /// A stable source index refers outside the submitted instance stream.
    SourceIndexOutOfRange {
        visible_offset: usize,
        source_index: u32,
        source_count: usize,
    },
    /// Compacted upload records do not fit in the caller-owned output slice.
    CompactedOutputTooSmall { required: usize, available: usize },
    /// Packed transform/bounds arrays or view-projection matrix have an invalid scalar count.
    PackedInputShapeInvalid,
    /// Packed record data is not a whole number of ABI records.
    PackedRecordLengthInvalid {
        byte_length: usize,
        record_size: usize,
    },
}

/// Append the visible source indices in stable input order.
///
/// The output capacity is checked against the worst case before any write, so failure never leaves
/// a partially updated visibility list. The view-projection matrix is column-major and uses the
/// WebGPU clip-depth range `[0, w]`. Frustum-plane rejection tests all eight transformed AABB
/// corners; any non-finite or inverted input is retained rather than risking a false-negative.
pub fn select_visible_instances(
    instances: &[BoundedInstance],
    view_projection: [[f32; 4]; 4],
    visible_source_indices: &mut [u32],
) -> Result<usize, VisibilityListError> {
    if instances.len() > u32::MAX as usize {
        return Err(VisibilityListError::TooManyInstances {
            count: instances.len(),
        });
    }
    if visible_source_indices.len() < instances.len() {
        return Err(VisibilityListError::OutputTooSmall {
            required: instances.len(),
            available: visible_source_indices.len(),
        });
    }

    let planes = extract_clip_planes(view_projection);
    let mut visible_count = 0;
    for (source_index, instance) in instances.iter().enumerate() {
        if !provably_outside_clip(instance, planes) {
            visible_source_indices[visible_count] = source_index as u32;
            visible_count += 1;
        }
    }
    Ok(visible_count)
}

/// WASM-friendly SoA variant of [`select_visible_instances`]. Bounds contain three floats per
/// instance, transforms contain 16 column-major floats per instance, and view-projection contains
/// 16 column-major floats. It performs no allocation and writes stable source indices.
pub fn select_visible_instances_packed(
    bounds_min: &[f32],
    bounds_max: &[f32],
    transforms_column_major: &[f32],
    view_projection_column_major: &[f32],
    visible_source_indices: &mut [u32],
) -> Result<usize, VisibilityListError> {
    if bounds_min.len() % 3 != 0
        || bounds_max.len() % 3 != 0
        || transforms_column_major.len() % 16 != 0
        || view_projection_column_major.len() != 16
    {
        return Err(VisibilityListError::PackedInputShapeInvalid);
    }
    let count = bounds_min.len() / 3;
    if bounds_max.len() / 3 != count || transforms_column_major.len() / 16 != count {
        return Err(VisibilityListError::PackedInputShapeInvalid);
    }
    if count > MAX_GPU_MESH_INSTANCES || count > u32::MAX as usize {
        return Err(VisibilityListError::TooManyInstances { count });
    }
    if visible_source_indices.len() < count {
        return Err(VisibilityListError::OutputTooSmall {
            required: count,
            available: visible_source_indices.len(),
        });
    }
    let view_projection = std::array::from_fn(|column| {
        std::array::from_fn(|row| view_projection_column_major[column * 4 + row])
    });
    let planes = extract_clip_planes(view_projection);
    let mut visible_count = 0;
    for source_index in 0..count {
        let min = std::array::from_fn(|axis| bounds_min[source_index * 3 + axis]);
        let max = std::array::from_fn(|axis| bounds_max[source_index * 3 + axis]);
        let world_from_local = std::array::from_fn(|column| {
            std::array::from_fn(|row| transforms_column_major[source_index * 16 + column * 4 + row])
        });
        let instance = BoundedInstance {
            bounds_min: min,
            bounds_max: max,
            world_from_local,
        };
        if !provably_outside_clip(&instance, planes) {
            visible_source_indices[visible_count] = source_index as u32;
            visible_count += 1;
        }
    }
    Ok(visible_count)
}

/// Select visible copies of one retained mesh after applying each per-instance transform and the
/// renderer's shared actor/model transform. The result contains stable source indices so the
/// caller can compact records without losing semantic identity. Invalid numeric inputs fail open.
pub fn select_visible_mesh_instances(
    bounds_min: [f32; 3],
    bounds_max: [f32; 3],
    model_from_mesh: [[f32; 4]; 4],
    view_projection: [[f32; 4]; 4],
    source_records: &[GpuInstanceRecord],
    visible_source_indices: &mut [u32],
) -> Result<usize, VisibilityListError> {
    if source_records.len() > u32::MAX as usize {
        return Err(VisibilityListError::TooManyInstances {
            count: source_records.len(),
        });
    }
    if visible_source_indices.len() < source_records.len() {
        return Err(VisibilityListError::OutputTooSmall {
            required: source_records.len(),
            available: visible_source_indices.len(),
        });
    }

    let planes = extract_clip_planes(view_projection);
    let mut visible_count = 0;
    for (source_index, record) in source_records.iter().enumerate() {
        let world_from_local = multiply_mat4(model_from_mesh, record.world_from_local);
        let instance = BoundedInstance {
            bounds_min,
            bounds_max,
            world_from_local,
        };
        if !provably_outside_clip(&instance, planes) {
            visible_source_indices[visible_count] = source_index as u32;
            visible_count += 1;
        }
    }
    Ok(visible_count)
}

fn multiply_mat4(left: [[f32; 4]; 4], right: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    std::array::from_fn(|column| {
        std::array::from_fn(|row| {
            (0..4)
                .map(|inner| left[inner][row] * right[column][inner])
                .sum()
        })
    })
}

/// Copy records in visible-index order into caller-owned upload scratch.
///
/// This preserves semantic identities through culling/compaction and allocates nothing. All
/// indices and output capacity are validated before the first write, so an error leaves output
/// untouched. The source stream remains the authority for identity; draw slots are transient.
pub fn compact_visible_records(
    source_records: &[GpuInstanceRecord],
    visible_source_indices: &[u32],
    output_records: &mut [GpuInstanceRecord],
) -> Result<usize, VisibilityListError> {
    if output_records.len() < visible_source_indices.len() {
        return Err(VisibilityListError::CompactedOutputTooSmall {
            required: visible_source_indices.len(),
            available: output_records.len(),
        });
    }
    for (visible_offset, &source_index) in visible_source_indices.iter().enumerate() {
        if source_index as usize >= source_records.len() {
            return Err(VisibilityListError::SourceIndexOutOfRange {
                visible_offset,
                source_index,
                source_count: source_records.len(),
            });
        }
    }
    for (output, &source_index) in output_records.iter_mut().zip(visible_source_indices.iter()) {
        *output = source_records[source_index as usize];
    }
    Ok(visible_source_indices.len())
}

/// Byte-oriented, alignment-independent form of [`compact_visible_records`] for WASM hosts.
/// Output capacity and every source index are checked before the first byte is written.
pub fn compact_visible_packed_records(
    source_records: &[u8],
    visible_source_indices: &[u32],
    output_records: &mut [u8],
) -> Result<usize, VisibilityListError> {
    let record_size = std::mem::size_of::<GpuInstanceRecord>();
    if source_records.len() % record_size != 0 {
        return Err(VisibilityListError::PackedRecordLengthInvalid {
            byte_length: source_records.len(),
            record_size,
        });
    }
    let required = visible_source_indices
        .len()
        .checked_mul(record_size)
        .ok_or(VisibilityListError::CompactedOutputTooSmall {
            required: usize::MAX,
            available: output_records.len(),
        })?;
    if output_records.len() < required {
        return Err(VisibilityListError::CompactedOutputTooSmall {
            required,
            available: output_records.len(),
        });
    }
    let source_count = source_records.len() / record_size;
    for (visible_offset, &source_index) in visible_source_indices.iter().enumerate() {
        if source_index as usize >= source_count {
            return Err(VisibilityListError::SourceIndexOutOfRange {
                visible_offset,
                source_index,
                source_count,
            });
        }
    }
    for (slot, &source_index) in visible_source_indices.iter().enumerate() {
        let source_offset = source_index as usize * record_size;
        let output_offset = slot * record_size;
        output_records[output_offset..output_offset + record_size]
            .copy_from_slice(&source_records[source_offset..source_offset + record_size]);
    }
    Ok(visible_source_indices.len())
}

fn extract_clip_planes(view_projection: [[f32; 4]; 4]) -> [[f32; 4]; 6] {
    let row = |index| {
        [
            view_projection[0][index],
            view_projection[1][index],
            view_projection[2][index],
            view_projection[3][index],
        ]
    };
    let x = row(0);
    let y = row(1);
    let z = row(2);
    let w = row(3);
    [
        add_plane(w, x),
        subtract_plane(w, x),
        add_plane(w, y),
        subtract_plane(w, y),
        z,
        subtract_plane(w, z),
    ]
}

fn add_plane(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|index| a[index] + b[index])
}

fn subtract_plane(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|index| a[index] - b[index])
}

fn provably_outside_clip(instance: &BoundedInstance, planes: [[f32; 4]; 6]) -> bool {
    if instance
        .bounds_min
        .iter()
        .chain(instance.bounds_max.iter())
        .chain(instance.world_from_local.iter().flatten())
        .chain(planes.iter().flatten())
        .any(|value| !value.is_finite())
        || (0..3).any(|axis| instance.bounds_min[axis] > instance.bounds_max[axis])
    {
        return false;
    }

    // Transform each clip plane into local space once, then test the AABB support point.
    // This is equivalent to checking all eight corners, with six plane tests instead of
    // transforming 48 corners per object. The relative epsilon errs toward retaining bounds.
    let center: [f32; 3] =
        std::array::from_fn(|axis| (instance.bounds_min[axis] + instance.bounds_max[axis]) * 0.5);
    let extent: [f32; 3] =
        std::array::from_fn(|axis| (instance.bounds_max[axis] - instance.bounds_min[axis]) * 0.5);
    for plane in planes {
        let local_plane = mul_transpose_vec4(instance.world_from_local, plane);
        if !local_plane.iter().all(|value| value.is_finite()) {
            return false;
        }
        let center_distance = local_plane[0] * center[0]
            + local_plane[1] * center[1]
            + local_plane[2] * center[2]
            + local_plane[3];
        let support_radius = local_plane[0].abs() * extent[0]
            + local_plane[1].abs() * extent[1]
            + local_plane[2].abs() * extent[2];
        let maximum_distance = center_distance + support_radius;
        let numeric_scale = center_distance.abs() + support_radius + local_plane[3].abs() + 1.0;
        if !maximum_distance.is_finite() || !numeric_scale.is_finite() {
            return false;
        }
        if maximum_distance < -1.0e-6 * numeric_scale {
            return true;
        }
    }
    false
}

fn mul_transpose_vec4(matrix: [[f32; 4]; 4], vector: [f32; 4]) -> [f32; 4] {
    std::array::from_fn(|column| (0..4).map(|row| matrix[column][row] * vector[row]).sum())
}

#[cfg(test)]
mod tests {
    use super::{
        compact_visible_packed_records, compact_visible_records, extract_clip_planes,
        provably_outside_clip, select_visible_instances, select_visible_instances_packed,
        select_visible_mesh_instances, BoundedInstance, GpuInstanceRecord, VisibilityListError,
    };
    use bytemuck::Zeroable;

    const IDENTITY: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    fn perspective(near: f32, far: f32) -> [[f32; 4]; 4] {
        let focal = 1.0 / (45.0_f32.to_radians() * 0.5).tan();
        [
            [focal, 0.0, 0.0, 0.0],
            [0.0, focal, 0.0, 0.0],
            [0.0, 0.0, far / (near - far), -1.0],
            [0.0, 0.0, near * far / (near - far), 0.0],
        ]
    }

    fn instance(min: [f32; 3], max: [f32; 3]) -> BoundedInstance {
        BoundedInstance {
            bounds_min: min,
            bounds_max: max,
            world_from_local: IDENTITY,
        }
    }

    #[test]
    fn selection_is_stable_and_keeps_intersecting_bounds() {
        let instances = [
            instance([-0.5; 3], [0.5; 3]),
            instance([2.0, -0.2, 0.2], [3.0, 0.2, 0.8]),
            instance([0.9, -0.1, 0.2], [1.1, 0.1, 0.8]),
        ];
        let mut visible = [u32::MAX; 3];
        let count = select_visible_instances(&instances, IDENTITY, &mut visible).unwrap();
        assert_eq!(&visible[..count], &[0, 2]);
    }

    #[test]
    fn mesh_instance_selection_composes_actor_and_instance_transforms() {
        let translate = |x: f32| {
            let mut matrix = IDENTITY;
            matrix[3][0] = x;
            matrix
        };
        let records = [
            GpuInstanceRecord::new(IDENTITY, 0x10),
            GpuInstanceRecord::new(translate(2.0), 0x20),
        ];
        let mut visible = [u32::MAX; 2];
        let count = select_visible_mesh_instances(
            [-0.2; 3],
            [0.2; 3],
            translate(-2.0),
            IDENTITY,
            &records,
            &mut visible,
        )
        .unwrap();
        assert_eq!(&visible[..count], &[1]);
        assert_eq!(records[visible[0] as usize].semantic_id(), 0x20);
    }

    #[test]
    fn mesh_instance_selection_preflights_capacity_and_fails_open_on_invalid_matrix() {
        let record = GpuInstanceRecord::new(IDENTITY, 7);
        let mut short = [0xfeed_u32; 0];
        assert_eq!(
            select_visible_mesh_instances(
                [-0.5; 3],
                [0.5; 3],
                IDENTITY,
                IDENTITY,
                &[record],
                &mut short,
            ),
            Err(VisibilityListError::OutputTooSmall {
                required: 1,
                available: 0,
            })
        );

        let mut malformed = IDENTITY;
        malformed[0][0] = f32::NAN;
        let mut visible = [u32::MAX; 1];
        assert_eq!(
            select_visible_mesh_instances(
                [-0.5; 3],
                [0.5; 3],
                malformed,
                IDENTITY,
                &[record],
                &mut visible,
            ),
            Ok(1)
        );
        assert_eq!(visible, [0]);
    }

    #[test]
    fn packed_selection_matches_struct_api_and_keeps_output_atomic_on_bad_shape() {
        let bounds_min = [-0.5, -0.5, -0.5, 2.0, -0.2, 0.2, 0.9, -0.1, 0.2];
        let bounds_max = [0.5, 0.5, 0.5, 3.0, 0.2, 0.8, 1.1, 0.1, 0.8];
        let mut transforms = [0.0; 48];
        for instance in 0..3 {
            for axis in 0..4 {
                transforms[instance * 16 + axis * 4 + axis] = 1.0;
            }
        }
        let view_projection: [f32; 16] =
            std::array::from_fn(|index| IDENTITY[index / 4][index % 4]);
        let mut visible = [u32::MAX; 3];
        assert_eq!(
            select_visible_instances_packed(
                &bounds_min,
                &bounds_max,
                &transforms,
                &view_projection,
                &mut visible,
            ),
            Ok(2)
        );
        assert_eq!(&visible[..2], &[0, 2]);
        let mut unchanged = [17_u32; 3];
        assert_eq!(
            select_visible_instances_packed(
                &bounds_min[..8],
                &bounds_max,
                &transforms,
                &view_projection,
                &mut unchanged,
            ),
            Err(VisibilityListError::PackedInputShapeInvalid)
        );
        assert_eq!(unchanged, [17; 3]);
    }

    #[test]
    fn rejects_each_fully_outside_webgpu_clip_plane() {
        let cases = [
            instance([-3.0, -0.1, 0.2], [-2.0, 0.1, 0.8]),
            instance([2.0, -0.1, 0.2], [3.0, 0.1, 0.8]),
            instance([-0.1, -3.0, 0.2], [0.1, -2.0, 0.8]),
            instance([-0.1, 2.0, 0.2], [0.1, 3.0, 0.8]),
            instance([-0.1, -0.1, -2.0], [0.1, 0.1, -1.0]),
            instance([-0.1, -0.1, 2.0], [0.1, 0.1, 3.0]),
        ];
        let mut visible = [u32::MAX; 6];
        assert_eq!(
            select_visible_instances(&cases, IDENTITY, &mut visible),
            Ok(0)
        );
    }

    #[test]
    fn transformed_bounds_are_tested_in_world_space() {
        let mut translated = instance([-0.25; 3], [0.25; 3]);
        translated.world_from_local[3][0] = 4.0;
        let mut visible = [u32::MAX; 1];
        assert_eq!(
            select_visible_instances(&[translated], IDENTITY, &mut visible),
            Ok(0)
        );
    }

    #[test]
    fn perspective_projection_checks_depth_and_fails_open_for_camera_crossing_bounds() {
        let instances = [
            instance([-0.1, -0.1, -2.1], [0.1, 0.1, -1.9]),
            instance([-0.1, -0.1, -0.08], [0.1, 0.1, -0.04]),
            instance([-0.1, -0.1, -120.0], [0.1, 0.1, -110.0]),
            instance([-0.1, -0.1, 1.0], [0.1, 0.1, 2.0]),
            instance([-0.2, -0.2, -1.0], [0.2, 0.2, 1.0]),
        ];
        let mut visible = [u32::MAX; 5];
        let count =
            select_visible_instances(&instances, perspective(0.1, 100.0), &mut visible).unwrap();
        assert_eq!(&visible[..count], &[0, 4]);
    }

    #[test]
    fn ten_thousand_repeated_instances_fit_the_caller_buffer_without_reordering() {
        let repeated = instance([-0.1, -0.1, 0.2], [0.1, 0.1, 0.8]);
        let instances = vec![repeated; 10_000];
        let mut visible = vec![u32::MAX; instances.len()];
        let count = select_visible_instances(&instances, IDENTITY, &mut visible).unwrap();
        assert_eq!(count, 10_000);
        assert_eq!(visible[0], 0);
        assert_eq!(visible[9_999], 9_999);
    }

    #[test]
    fn support_radius_matches_corner_oracle_for_deterministic_affine_cases() {
        let view_projection = perspective(0.1, 80.0);
        let mut seed = 0x91E1_0DA5_u32;
        let mut instances = Vec::with_capacity(512);
        for _ in 0..512 {
            let mut next = || {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (seed as f32 / u32::MAX as f32) * 2.0 - 1.0
            };
            let min = [next() * 4.0, next() * 4.0, -next().abs() * 60.0 - 0.2];
            let extent = [next().abs() * 1.5, next().abs() * 1.5, next().abs() * 1.5];
            let angle = next() * std::f32::consts::PI;
            let (sine, cosine) = angle.sin_cos();
            let translation = [next() * 8.0, next() * 8.0, next() * 60.0 - 30.0];
            let mut transform = IDENTITY;
            transform[0] = [cosine, sine, 0.0, 0.0];
            transform[1] = [-sine, cosine, 0.0, 0.0];
            transform[3] = [translation[0], translation[1], translation[2], 1.0];
            instances.push(BoundedInstance {
                bounds_min: min,
                bounds_max: [min[0] + extent[0], min[1] + extent[1], min[2] + extent[2]],
                world_from_local: transform,
            });
        }
        let planes = extract_clip_planes(view_projection);
        for instance in instances {
            assert_eq!(
                provably_outside_clip(&instance, planes),
                corner_oracle_outside(&instance, planes),
                "instance {instance:?}"
            );
        }
    }

    #[test]
    fn compact_upload_stream_preserves_stable_semantic_ids_and_order() {
        let source = [
            GpuInstanceRecord::new(IDENTITY, 0x0000_0001_0000_0002),
            GpuInstanceRecord::new(IDENTITY, 0xAABB_CCDD_EEFF_0011),
            GpuInstanceRecord::new(IDENTITY, 0xFEDC_BA98_7654_3210),
        ];
        assert_eq!(std::mem::size_of::<GpuInstanceRecord>(), 128);
        assert_eq!(
            std::mem::offset_of!(GpuInstanceRecord, semantic_id_words),
            112
        );
        assert_eq!(
            std::mem::offset_of!(GpuInstanceRecord, orientation_sign),
            120
        );
        let indices = [2, 0];
        let mut compacted = [GpuInstanceRecord::zeroed(); 2];
        assert_eq!(
            compact_visible_records(&source, &indices, &mut compacted),
            Ok(2)
        );
        assert_eq!(compacted[0].semantic_id(), source[2].semantic_id());
        assert_eq!(compacted[1].semantic_id(), source[0].semantic_id());
    }

    #[test]
    fn packed_compaction_is_alignment_independent_and_preserves_identity() {
        let source = [
            GpuInstanceRecord::new(IDENTITY, 11),
            GpuInstanceRecord::new(IDENTITY, 22),
            GpuInstanceRecord::new(IDENTITY, 33),
        ];
        let source_bytes = bytemuck::cast_slice(&source);
        let mut output = [0_u8; 256];
        assert_eq!(
            compact_visible_packed_records(source_bytes, &[2, 0], &mut output),
            Ok(2)
        );
        let first = bytemuck::pod_read_unaligned::<GpuInstanceRecord>(&output[..128]);
        let second = bytemuck::pod_read_unaligned::<GpuInstanceRecord>(&output[128..]);
        assert_eq!([first.semantic_id(), second.semantic_id()], [33, 11]);
    }

    #[test]
    fn compact_stream_errors_are_all_or_nothing() {
        let source = [GpuInstanceRecord::new(IDENTITY, 7)];
        let sentinel = GpuInstanceRecord::new(IDENTITY, 99);
        let mut output = [sentinel; 2];
        assert_eq!(
            compact_visible_records(&source, &[0, 0], &mut output[..1]),
            Err(VisibilityListError::CompactedOutputTooSmall {
                required: 2,
                available: 1
            })
        );
        assert_eq!(output, [sentinel; 2]);
        assert_eq!(
            compact_visible_records(&source, &[0, 4], &mut output),
            Err(VisibilityListError::SourceIndexOutOfRange {
                visible_offset: 1,
                source_index: 4,
                source_count: 1
            })
        );
        assert_eq!(output, [sentinel; 2]);
    }

    fn corner_oracle_outside(instance: &BoundedInstance, planes: [[f32; 4]; 6]) -> bool {
        if instance
            .bounds_min
            .iter()
            .chain(instance.bounds_max.iter())
            .chain(instance.world_from_local.iter().flatten())
            .chain(planes.iter().flatten())
            .any(|value| !value.is_finite())
            || (0..3).any(|axis| instance.bounds_min[axis] > instance.bounds_max[axis])
        {
            return false;
        }
        for plane in planes {
            let mut all_outside = true;
            for corner in 0..8 {
                let point = [
                    if corner & 1 == 0 {
                        instance.bounds_min[0]
                    } else {
                        instance.bounds_max[0]
                    },
                    if corner & 2 == 0 {
                        instance.bounds_min[1]
                    } else {
                        instance.bounds_max[1]
                    },
                    if corner & 4 == 0 {
                        instance.bounds_min[2]
                    } else {
                        instance.bounds_max[2]
                    },
                    1.0,
                ];
                let world = std::array::from_fn::<_, 4, _>(|row| {
                    (0..4)
                        .map(|column| instance.world_from_local[column][row] * point[column])
                        .sum::<f32>()
                });
                let distance = plane.iter().zip(world).map(|(a, b)| a * b).sum::<f32>();
                let epsilon = 1.0e-6 * (distance.abs() + 1.0);
                all_outside &= distance < -epsilon;
            }
            if all_outside {
                return true;
            }
        }
        false
    }

    #[test]
    fn invalid_or_inverted_bounds_fail_open() {
        let cases = [
            instance([f32::NAN, 0.0, 0.0], [f32::NAN, 0.0, 0.0]),
            instance([1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]),
        ];
        let mut visible = [u32::MAX; 2];
        assert_eq!(
            select_visible_instances(&cases, IDENTITY, &mut visible),
            Ok(2)
        );
        assert_eq!(visible, [0, 1]);
    }

    #[test]
    fn insufficient_output_capacity_fails_before_writing() {
        let cases = [instance([-0.5; 3], [0.5; 3]), instance([2.0; 3], [3.0; 3])];
        let mut visible = [77; 1];
        assert_eq!(
            select_visible_instances(&cases, IDENTITY, &mut visible),
            Err(VisibilityListError::OutputTooSmall {
                required: 2,
                available: 1
            })
        );
        assert_eq!(visible, [77]);
    }
}
