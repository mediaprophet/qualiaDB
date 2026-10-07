//! Cold-built material ranges and conservative geometry bounds used by rendering passes.

use crate::container_10d::{MaterialRecord, OpacityMode, SubmeshRange};

#[derive(Clone, Copy)]
pub(super) struct MaterialDraw {
    pub first_index: u32,
    pub index_count: u32,
    pub material_offset: u32,
    pub material_index: usize,
    pub opacity_mode: OpacityMode,
    pub center: [f32; 3],
    /// Conservative model-space bounds used by shadow-cascade range culling.
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub sort_depth: f32,
}

pub(super) fn build_material_draws(
    materials: &[MaterialRecord],
    ranges: &[SubmeshRange],
    positions: &[[f32; 3]],
    indices: &[u32],
) -> Result<Vec<MaterialDraw>, String> {
    let mut draws: Vec<MaterialDraw> =
        Vec::with_capacity(ranges.len().min(super::materials::MAX_MATERIAL_DRAWS));
    for (range_index, range) in ranges.iter().enumerate() {
        let material_index = materials
            .binary_search_by_key(&range.material_id, |material| material.id)
            .map_err(|_| format!("MAT1 range {range_index} references an unknown material"))?;
        let offset =
            u32::try_from(material_index as u64 * super::materials::MATERIAL_UNIFORM_STRIDE)
                .map_err(|_| "MAT1 dynamic-uniform offset exceeds the GPU ABI".to_string())?;
        let (range_min, range_max) =
            range_bounds(positions, indices, range.first_index, range.index_count)?;
        if let Some(previous) = draws.last_mut() {
            if previous.material_offset == offset
                && previous.first_index.checked_add(previous.index_count) == Some(range.first_index)
            {
                previous.index_count = previous
                    .index_count
                    .checked_add(range.index_count)
                    .ok_or_else(|| "MAT1 merged draw range overflow".to_string())?;
                for axis in 0..3 {
                    previous.bounds_min[axis] = previous.bounds_min[axis].min(range_min[axis]);
                    previous.bounds_max[axis] = previous.bounds_max[axis].max(range_max[axis]);
                }
                previous.center = bounds_center(previous.bounds_min, previous.bounds_max);
                continue;
            }
        }
        if draws.len() == super::materials::MAX_MATERIAL_DRAWS {
            return Err(format!(
                "MAT1 draw count exceeds renderer cap {}",
                super::materials::MAX_MATERIAL_DRAWS
            ));
        }
        draws.push(MaterialDraw {
            first_index: range.first_index,
            index_count: range.index_count,
            material_offset: offset,
            material_index,
            opacity_mode: materials[material_index].opacity_mode,
            center: bounds_center(range_min, range_max),
            bounds_min: range_min,
            bounds_max: range_max,
            sort_depth: 0.0,
        });
    }
    Ok(draws)
}

fn range_bounds(
    positions: &[[f32; 3]],
    indices: &[u32],
    first_index: u32,
    index_count: u32,
) -> Result<([f32; 3], [f32; 3]), String> {
    let end = first_index
        .checked_add(index_count)
        .ok_or_else(|| "MAT1 center range overflow".to_string())? as usize;
    let start = first_index as usize;
    if end > indices.len() || start >= end {
        return Err("MAT1 center range is empty or exceeds the index stream".to_string());
    }
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for &index in &indices[start..end] {
        let point = positions
            .get(index as usize)
            .ok_or_else(|| "MAT1 center references a missing vertex".to_string())?;
        for axis in 0..3 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    }
    Ok((min, max))
}

fn bounds_center(min: [f32; 3], max: [f32; 3]) -> [f32; 3] {
    [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ]
}

pub(super) fn update_draw_bounds(
    draws: &mut [MaterialDraw],
    positions: &[[f32; 3]],
    indices: &[u32],
) {
    for draw in draws {
        if let Ok((min, max)) = range_bounds(positions, indices, draw.first_index, draw.index_count)
        {
            draw.bounds_min = min;
            draw.bounds_max = max;
            draw.center = bounds_center(min, max);
        }
    }
}
