//! Bounded prototype ranges shared by forward, depth, shadows and picking.
use crate::render::instance_culling::GpuInstanceRecord;
use crate::render::instance_culling::{compact_visible_records, select_visible_mesh_instances};
use std::ops::Range;

pub(super) const MAX_INSTANCE_GROUPS: usize = 512;

#[derive(Clone, Debug)]
pub(super) struct MeshInstanceGroup {
    pub indices: Range<u32>,
    pub source: Range<u32>,
    pub visible: Range<u32>,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
}

/// Cold receipt counts; submitted geometry excludes later raster clipping and other passes.
pub(super) fn triangle_workload(
    groups: &[MeshInstanceGroup],
    source_triangles: u64,
    source_instances: u32,
    visible_instances: u32,
) -> (u64, u64) {
    if groups.is_empty() {
        return (
            source_triangles * u64::from(source_instances),
            source_triangles * u64::from(visible_instances),
        );
    }
    groups.iter().fold((0, 0), |(authored, submitted), group| {
        let triangles = u64::from((group.indices.end - group.indices.start) / 3);
        (
            authored + triangles * u64::from(group.source.end - group.source.start),
            submitted + triangles * u64::from(group.visible.end - group.visible.start),
        )
    })
}

pub(super) fn compile_groups(
    packed: &[u32],
    positions: &[[f32; 3]],
    indices: &[u32],
    instance_count: usize,
) -> Result<Vec<MeshInstanceGroup>, &'static str> {
    if packed.is_empty() || packed.len() % 4 != 0 || packed.len() / 4 > MAX_INSTANCE_GROUPS {
        return Err("instance groups require 1..512 four-word descriptors");
    }
    if instance_count == 0
        || instance_count > crate::render::instance_culling::MAX_GPU_MESH_INSTANCES
    {
        return Err("instance group source count is outside the native bound");
    }
    let mut result = Vec::with_capacity(packed.len() / 4);
    let (mut next_index, mut next_instance) = (0u32, 0u32);
    for words in packed.chunks_exact(4) {
        let [first_index, index_count, first_instance, count] =
            [words[0], words[1], words[2], words[3]];
        let end_index = first_index
            .checked_add(index_count)
            .ok_or("group index overflow")?;
        let end_instance = first_instance
            .checked_add(count)
            .ok_or("group instance overflow")?;
        if first_index != next_index
            || first_instance != next_instance
            || index_count == 0
            || index_count % 3 != 0
            || count == 0
            || end_index as usize > indices.len()
            || end_instance as usize > instance_count
        {
            return Err("instance groups must partition triangles and records exactly once");
        }
        let (bounds_min, bounds_max) = geometry_bounds(
            positions,
            &indices[first_index as usize..end_index as usize],
        )?;
        result.push(MeshInstanceGroup {
            indices: first_index..end_index,
            source: first_instance..end_instance,
            visible: first_instance..end_instance,
            bounds_min,
            bounds_max,
        });
        next_index = end_index;
        next_instance = end_instance;
    }
    if next_index as usize != indices.len() || next_instance as usize != instance_count {
        return Err("instance group partition leaves unowned geometry or records");
    }
    Ok(result)
}

pub(super) fn geometry_bounds(
    positions: &[[f32; 3]],
    indices: &[u32],
) -> Result<([f32; 3], [f32; 3]), &'static str> {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for &i in indices {
        let p = positions
            .get(i as usize)
            .ok_or("prototype references a missing vertex")?;
        for axis in 0..3 {
            if !p[axis].is_finite() {
                return Err("prototype has a nonfinite position");
            }
            min[axis] = min[axis].min(p[axis]);
            max[axis] = max[axis].max(p[axis]);
        }
    }
    Ok((min, max))
}

pub(super) fn world_bounds(
    groups: &[MeshInstanceGroup],
    records: &[GpuInstanceRecord],
) -> Option<([f32; 3], [f32; 3])> {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for group in groups {
        for record in records.get(group.source.start as usize..group.source.end as usize)? {
            let m = record.world_from_local;
            for corner in 0..8 {
                let p: [f32; 3] = std::array::from_fn(|a| {
                    if corner & (1 << a) == 0 {
                        group.bounds_min[a]
                    } else {
                        group.bounds_max[a]
                    }
                });
                for axis in 0..3 {
                    let v = m[0][axis] * p[0] + m[1][axis] * p[1] + m[2][axis] * p[2] + m[3][axis];
                    if !v.is_finite() {
                        return None;
                    }
                    min[axis] = min[axis].min(v);
                    max[axis] = max[axis].max(v);
                }
            }
        }
    }
    (!groups.is_empty()).then_some((min, max))
}

pub(super) fn compact_group_visibility(
    groups: &mut [MeshInstanceGroup],
    records: &[GpuInstanceRecord],
    model: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    indices: &mut [u32],
    output: &mut [GpuInstanceRecord],
) -> Result<usize, &'static str> {
    if indices.len() < records.len()
        || output.len() < records.len()
        || groups
            .iter()
            .any(|g| g.source.start > g.source.end || g.source.end as usize > records.len())
    {
        return Err("group visibility scratch or source spans are invalid");
    }
    let mut cursor = 0;
    for group in groups {
        let start = group.source.start as usize;
        let end = group.source.end as usize;
        let source = &records[start..end];
        let selected = &mut indices[start..end];
        let count = select_visible_mesh_instances(
            group.bounds_min,
            group.bounds_max,
            model,
            view,
            source,
            selected,
        )
        .unwrap_or_else(|_| {
            for (i, item) in selected.iter_mut().enumerate() {
                *item = i as u32;
            }
            source.len()
        });
        compact_visible_records(
            source,
            &selected[..count],
            &mut output[cursor..cursor + count],
        )
        .map_err(|_| "native group compaction refused its visibility indices")?;
        group.visible = cursor as u32..(cursor + count) as u32;
        cursor += count;
    }
    Ok(cursor)
}

/// Material coalescing may cross a prototype boundary; intersect, never duplicate that draw.
pub(super) fn draw_ranges(
    groups: &[MeshInstanceGroup],
    first: u32,
    count: u32,
    fallback_count: u32,
    visible: bool,
) -> DrawRanges<'_> {
    DrawRanges {
        groups,
        next: 0,
        draw: first..first.saturating_add(count),
        fallback_count,
        visible,
    }
}

pub(super) struct DrawRanges<'a> {
    groups: &'a [MeshInstanceGroup],
    next: usize,
    draw: Range<u32>,
    fallback_count: u32,
    visible: bool,
}

impl Iterator for DrawRanges<'_> {
    type Item = (Range<u32>, Range<u32>);
    fn next(&mut self) -> Option<Self::Item> {
        if self.groups.is_empty() {
            if self.next != 0 {
                return None;
            }
            self.next = 1;
            return Some((self.draw.clone(), 0..self.fallback_count));
        }
        while let Some(group) = self.groups.get(self.next) {
            self.next += 1;
            let range =
                self.draw.start.max(group.indices.start)..self.draw.end.min(group.indices.end);
            let instances = if self.visible {
                &group.visible
            } else {
                &group.source
            };
            if !range.is_empty() && !instances.is_empty() {
                return Some((range, instances.clone()));
            }
        }
        None
    }
}

#[cfg(test)]
#[path = "instance_groups_tests.rs"]
mod tests;
