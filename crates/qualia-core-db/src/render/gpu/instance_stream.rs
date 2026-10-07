//! Bounded per-instance transform storage for indexed mesh batches.

use crate::gpu_context::{global_vram_ledger, VramReservation, VramResourceClass};
use crate::render::instance_culling::GpuInstanceRecord;
use wgpu::util::DeviceExt;

pub(super) const MAX_MESH_INSTANCES: usize =
    crate::render::instance_culling::MAX_GPU_MESH_INSTANCES;
const IDENTITY: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];
const IDENTITY3: [[f32; 4]; 3] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
];
const DEFAULT_INSTANCE: GpuInstanceRecord = GpuInstanceRecord {
    world_from_local: IDENTITY,
    normal_from_local: IDENTITY3,
    semantic_id_words: [0; 2],
    orientation_sign: 1.0,
    _padding: 0,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshInstanceUploadError {
    TooManyInstances { count: usize, maximum: usize },
    InvalidTransform { index: usize },
    TransparentInstancesRequireSortedSubmission,
    ReservationRefused,
    BufferSizeOverflow,
}

pub(super) struct GpuInstanceStream {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    _reservation: VramReservation<'static>,
    capacity: usize,
    count: u32,
}

impl GpuInstanceStream {
    pub(super) fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
    ) -> Result<Self, MeshInstanceUploadError> {
        let bytes = std::mem::size_of::<GpuInstanceRecord>() as u64;
        let reservation = global_vram_ledger()
            .try_reserve_graphics(VramResourceClass::Geometry, bytes)
            .map_err(|_| MeshInstanceUploadError::ReservationRefused)?;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("portal-mesh-instances"),
            contents: bytemuck::bytes_of(&DEFAULT_INSTANCE),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = bind_group(device, layout, &buffer);
        Ok(Self {
            buffer,
            bind_group,
            _reservation: reservation,
            capacity: 1,
            count: 1,
        })
    }

    pub(super) fn replace_packed(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        packed_records: &[u8],
    ) -> Result<(), MeshInstanceUploadError> {
        let record_size = std::mem::size_of::<GpuInstanceRecord>();
        if packed_records.len() % record_size != 0 {
            return Err(MeshInstanceUploadError::InvalidTransform { index: 0 });
        }
        let records = packed_records.len() / record_size;
        if records > MAX_MESH_INSTANCES {
            return Err(MeshInstanceUploadError::TooManyInstances {
                count: records,
                maximum: MAX_MESH_INSTANCES,
            });
        }
        for (index, bytes) in packed_records.chunks_exact(record_size).enumerate() {
            let record = bytemuck::pod_read_unaligned::<GpuInstanceRecord>(bytes);
            if !supports_affine_transform(&record) {
                return Err(MeshInstanceUploadError::InvalidTransform { index });
            }
        }

        let (upload, count) = if records == 0 {
            (bytemuck::bytes_of(&DEFAULT_INSTANCE), 1_u32)
        } else {
            (
                packed_records,
                u32::try_from(records).map_err(|_| MeshInstanceUploadError::BufferSizeOverflow)?,
            )
        };
        let required = upload.len() / record_size;
        if required <= self.capacity {
            queue.write_buffer(&self.buffer, 0, upload);
            self.count = count;
            return Ok(());
        }

        // Grow geometrically but never beyond the declared 10k-instance workload. Reserve the
        // replacement while the old stream remains valid; if admission fails, current draws stay
        // intact and the caller receives a deterministic refusal.
        let capacity = required
            .checked_next_power_of_two()
            .unwrap_or(MAX_MESH_INSTANCES)
            .min(MAX_MESH_INSTANCES);
        let buffer_bytes = (capacity as u64)
            .checked_mul(record_size as u64)
            .ok_or(MeshInstanceUploadError::BufferSizeOverflow)?;
        let reservation = global_vram_ledger()
            .try_reserve_graphics(VramResourceClass::Geometry, buffer_bytes)
            .map_err(|_| MeshInstanceUploadError::ReservationRefused)?;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("portal-mesh-instances-grown"),
            size: buffer_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&buffer, 0, upload);
        let bind_group = bind_group(device, layout, &buffer);
        self.buffer = buffer;
        self.bind_group = bind_group;
        self._reservation = reservation;
        self.capacity = capacity;
        self.count = count;
        Ok(())
    }

    /// Replace the camera-visible stream. Unlike the source stream, zero visible records means
    /// no indexed instances should be drawn (an empty source stream still means legacy identity).
    pub(super) fn replace_culled_packed(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        packed_records: &[u8],
    ) -> Result<(), MeshInstanceUploadError> {
        if packed_records.is_empty() {
            self.count = 0;
            return Ok(());
        }
        self.replace_packed(device, queue, layout, packed_records)
    }

    pub(super) fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }

    pub(super) fn count(&self) -> u32 {
        self.count
    }
}

fn bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("portal-mesh-instance-bind"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    })
}

/// Reject transforms that cannot produce a stable, matching inverse-transpose normal matrix.
fn supports_affine_transform(record: &GpuInstanceRecord) -> bool {
    let matrix = record.world_from_local;
    if matrix.iter().flatten().any(|value| !value.is_finite())
        || matrix[0][3].abs() > 1.0e-5
        || matrix[1][3].abs() > 1.0e-5
        || matrix[2][3].abs() > 1.0e-5
        || (matrix[3][3] - 1.0).abs() > 1.0e-5
    {
        return false;
    }
    let columns = [
        [matrix[0][0], matrix[0][1], matrix[0][2]],
        [matrix[1][0], matrix[1][1], matrix[1][2]],
        [matrix[2][0], matrix[2][1], matrix[2][2]],
    ];
    let length = |column: [f32; 3]| {
        (column[0] * column[0] + column[1] * column[1] + column[2] * column[2]).sqrt()
    };
    let volume_scale = length(columns[0]) * length(columns[1]) * length(columns[2]);
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
    if !determinant.is_finite()
        || !volume_scale.is_finite()
        || volume_scale == 0.0
        || determinant.abs() <= 1.0e-6 * volume_scale
    {
        return false;
    }
    let expected = GpuInstanceRecord::new(matrix, record.semantic_id());
    if !record
        .normal_from_local
        .iter()
        .flatten()
        .all(|value| value.is_finite())
        || !record.orientation_sign.is_finite()
        || (record.orientation_sign - expected.orientation_sign).abs() > 1.0e-5
    {
        return false;
    }
    record
        .normal_from_local
        .iter()
        .flatten()
        .zip(expected.normal_from_local.iter().flatten())
        .all(|(actual, expected)| (actual - expected).abs() <= 1.0e-4 * (1.0 + expected.abs()))
}

#[cfg(test)]
mod tests {
    use super::{supports_affine_transform, GpuInstanceRecord};

    #[test]
    fn accepts_rigid_and_nonuniform_affine_scale() {
        let mut transform = [
            [0.0, 2.0, 0.0, 0.0],
            [-2.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 2.0, 0.0],
            [3.0, -4.0, 5.0, 1.0],
        ];
        assert!(supports_affine_transform(&GpuInstanceRecord::new(
            transform, 1
        )));
        transform[0][0] = f32::NAN;
        assert!(!supports_affine_transform(&GpuInstanceRecord::new(
            transform, 1
        )));
    }

    #[test]
    fn accepts_reflection_and_rejects_singular_scale() {
        let mut nonuniform = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 2.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        assert!(supports_affine_transform(&GpuInstanceRecord::new(
            nonuniform, 1
        )));
        nonuniform[0][0] = -1.0;
        nonuniform[1][1] = 1.0;
        assert!(supports_affine_transform(&GpuInstanceRecord::new(
            nonuniform, 1
        )));
        nonuniform[1][1] = 0.0;
        assert!(!supports_affine_transform(&GpuInstanceRecord::new(
            nonuniform, 1
        )));
    }

    #[test]
    fn rejects_a_forged_normal_frame() {
        let matrix = [
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 3.0, 0.0, 0.0],
            [0.0, 0.0, 4.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let mut record = GpuInstanceRecord::new(matrix, 8);
        assert!((record.normal_from_local[0][0] - 0.5).abs() < 1.0e-6);
        assert!((record.normal_from_local[1][1] - 1.0 / 3.0).abs() < 1.0e-6);
        assert!((record.normal_from_local[2][2] - 0.25).abs() < 1.0e-6);
        assert!(supports_affine_transform(&record));
        record.normal_from_local[0][0] = 1.0;
        assert!(!supports_affine_transform(&record));
    }

    #[test]
    fn determinant_guard_is_scale_relative_for_small_instances() {
        let transform = [
            [1.0e-10, 0.0, 0.0, 0.0],
            [0.0, 1.0e-10, 0.0, 0.0],
            [0.0, 0.0, 1.0e-10, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let record = GpuInstanceRecord::new(transform, 9);
        assert!(supports_affine_transform(&record));
        assert!((record.normal_from_local[0][0] - 1.0e10).abs() < 2048.0);
    }
}
