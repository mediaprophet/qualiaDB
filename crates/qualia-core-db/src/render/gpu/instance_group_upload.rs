use super::{instance_groups::*, PortalGpu};
use crate::render::instance_culling::GpuInstanceRecord;

impl PortalGpu {
    /// Descriptors partition index and ABI-record streams as [first_index, count, first_record, count].
    pub fn set_mesh_instance_groups_packed(
        &mut self,
        descriptors: &[u32],
        packed_records: &[u8],
    ) -> Result<(), String> {
        let stride = std::mem::size_of::<GpuInstanceRecord>();
        if packed_records.len() % stride != 0 {
            return Err("invalid native instance record byte length".into());
        }
        let count = packed_records.len() / stride;
        if count == 0 || count > 10_000 {
            return Err("native prototype record count outside 1..=10000".into());
        }
        etalon_render::instance_stream::validate_dirty_ranges_packed(
            packed_records,
            &[etalon_render::instance_stream::DirtyInstanceRange::new(
                0,
                count as u32,
            )],
        )
        .map_err(|e| format!("invalid prototype instance: {e:?}"))?;
        let mesh = self
            .mesh
            .as_ref()
            .ok_or("instance groups require an admitted mesh")?;
        let groups = compile_groups(descriptors, &mesh.cpu_positions, &mesh.cpu_indices, count)
            .map_err(String::from)?;
        let records: Vec<GpuInstanceRecord> = packed_records
            .chunks_exact(stride)
            .map(bytemuck::pod_read_unaligned)
            .collect();
        let bounds = world_bounds(&groups, &records).ok_or("nonfinite instanced world bounds")?;
        self.set_mesh_instances_packed(packed_records)
            .map_err(|e| format!("instance stream admission: {e:?}"))?;
        self.mesh_instance_groups = groups;
        self.mesh_base_aabb = Some(crate::render::physics::Aabb {
            min: bounds.0,
            max: bounds.1,
        });
        Ok(())
    }

    pub fn mesh_instance_group_counts(&self) -> (u32, u32, u32, u64) {
        if self.mesh_instance_groups.is_empty() {
            let source = if self.mesh.is_some() {
                self.mesh_instance_source.len() as u32
            } else {
                0
            };
            let visible = if self.mesh.is_some() {
                self.visible_mesh_instances.count()
            } else {
                0
            };
            let triangles = self
                .mesh
                .as_ref()
                .map_or(0, |m| m.cpu_indices.len() as u64 / 3);
            return (0, source, visible, triangles * u64::from(source));
        }
        let triangles = self
            .mesh_instance_groups
            .iter()
            .map(|g| {
                u64::from((g.indices.end - g.indices.start) / 3)
                    * u64::from(g.source.end - g.source.start)
            })
            .sum();
        (
            self.mesh_instance_groups.len() as u32,
            self.mesh_instance_source.len() as u32,
            self.mesh_instance_groups
                .iter()
                .map(|g| g.visible.end - g.visible.start)
                .sum(),
            triangles,
        )
    }

    /// Current forward mesh submission, not pixels, shadow work, or authored expanded totals.
    pub fn mesh_submitted_triangle_count(&self) -> u64 {
        let (_, source, visible, _) = self.mesh_instance_group_counts();
        let triangles = self
            .mesh
            .as_ref()
            .map_or(0, |m| m.cpu_indices.len() as u64 / 3);
        triangle_workload(&self.mesh_instance_groups, triangles, source, visible).1
    }

    pub(super) fn refresh_instance_group_bounds(&mut self) {
        if self.mesh_instance_groups.is_empty() {
            return;
        }
        let Some(mesh) = self.mesh.as_ref() else {
            return;
        };
        for group in &mut self.mesh_instance_groups {
            if let Ok((min, max)) = geometry_bounds(
                &mesh.cpu_positions,
                &mesh.cpu_indices[group.indices.start as usize..group.indices.end as usize],
            ) {
                group.bounds_min = min;
                group.bounds_max = max;
            }
        }
        if let Some((min, max)) =
            world_bounds(&self.mesh_instance_groups, &self.mesh_instance_source)
        {
            self.mesh_base_aabb = Some(crate::render::physics::Aabb { min, max });
        }
    }
}
