//! Carry authored layer opacity through the material coverage contract.
use crate::container_10d::{MaterialRecord, OpacityMode, SubmeshRange};

pub(crate) fn apply_layer_opacity(
    materials: &mut [MaterialRecord],
    ranges: &[SubmeshRange],
    indices: &[u32],
    colors: &[[f32; 4]],
) {
    // Materials are sorted by ID before sealing. Inspect only referenced
    // vertices, so a hidden unused vertex cannot change an unrelated draw.
    let mut coverage = vec![0u8; materials.len()];
    for range in ranges {
        let Ok(material) = materials.binary_search_by_key(&range.material_id, |m| m.id) else {
            continue;
        };
        let start = range.first_index as usize;
        let end = start.saturating_add(range.index_count as usize).min(indices.len());
        for &vertex in indices.get(start..end).unwrap_or(&[]) {
            if let Some(color) = colors.get(vertex as usize) {
                if color[3] > 0.0 && color[3] < 1.0 {
                    coverage[material] = 2;
                } else if color[3] == 0.0 {
                    coverage[material] = coverage[material].max(1);
                }
            }
        }
    }
    for (material, coverage) in materials.iter_mut().zip(coverage) {
        if coverage == 0 {
            continue;
        }
        material.multiply_vertex_color = true;
        if coverage == 2 || material.opacity_mode == OpacityMode::Blend {
            material.opacity_mode = OpacityMode::Blend;
        } else {
            material.opacity_mode = OpacityMode::Mask;
            material.alpha_cutoff = 0.5;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_visibility_selects_mask_or_blend_without_changing_opaque_draws() {
        let mut materials: Vec<_> = (1..=3).map(|id| {
            let mut material = MaterialRecord::legacy_default();
            material.id = id;
            material.opacity_mode = OpacityMode::Opaque;
            material
        }).collect();
        let ranges: Vec<_> = (0..3).map(|i| SubmeshRange {
            first_index: i * 3, index_count: 3, material_id: (i + 1) as u64,
            semantic_id: 0,
        }).collect();
        let mut colors = vec![[1.0; 4]; 10];
        colors[0][3] = 0.0;
        colors[3][3] = 0.4;
        colors[9][3] = 0.0; // Unreferenced; must not alter material 3.
        apply_layer_opacity(&mut materials, &ranges, &(0..9).collect::<Vec<_>>(), &colors);
        assert_eq!(materials[0].opacity_mode, OpacityMode::Mask);
        assert_eq!(materials[0].alpha_cutoff, 0.5);
        assert_eq!(materials[1].opacity_mode, OpacityMode::Blend);
        assert_eq!(materials[2].opacity_mode, OpacityMode::Opaque);
        assert!(materials[0].multiply_vertex_color);
    }
}
