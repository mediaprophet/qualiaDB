//! Preserve authored tissue coverage without making an entire mixed layer transparent.
use super::{MaterialRecord, OpacityMode, SubmeshRange, TissueSubmeshSlice};

pub(super) fn append_tissue_ranges(
    base: MaterialRecord,
    slice: &TissueSubmeshSlice,
    indices: &[u32],
    colors: &[[u8; 4]],
    materials: &mut Vec<MaterialRecord>,
    ranges: &mut Vec<SubmeshRange>,
) -> Result<(), String> {
    let start = slice.index_start as usize;
    let end = start
        .checked_add(slice.index_count as usize)
        .ok_or("Tissue range overflow")?;
    let faces = indices.get(start..end).ok_or("Tissue range exceeds mesh")?;
    if faces.len() % 3 != 0 {
        return Err("Tissue range must contain complete triangles".into());
    }
    let mut emitted = [false; 3];
    for (triangle_index, triangle) in faces.chunks_exact(3).enumerate() {
        let mut coverage = 0usize;
        for &vertex in triangle {
            let alpha = colors
                .get(vertex as usize)
                .ok_or("Tissue color index exceeds mesh")?[3];
            coverage = coverage.max(match alpha {
                255 => 0,
                0 => 1,
                _ => 2,
            });
        }
        let id = if coverage == 0 {
            base.id
        } else {
            crate::q_hash(&format!(
                "urn:qualia:botanical:coverage:{}:{coverage}",
                base.id
            ))
        };
        if !emitted[coverage] {
            let mut material = base.clone();
            material.id = id;
            material.multiply_vertex_color = true;
            material.opacity_mode = match coverage {
                0 => OpacityMode::Opaque,
                1 => OpacityMode::Mask,
                _ => OpacityMode::Blend,
            };
            if coverage == 1 {
                material.alpha_cutoff = 0.5;
            }
            // The shadow pass has no translucent transmittance integration.
            // A translucent cuticle must not cast a fully opaque rectangular shadow.
            if coverage == 2 {
                material.casts_shadow = false;
            }
            materials.push(material);
            emitted[coverage] = true;
        }
        let first_index = slice.index_start + triangle_index as u32 * 3;
        if let Some(previous) = ranges.last_mut().filter(|range| {
            range.material_id == id
                && range.semantic_id == base.id
                && range.first_index + range.index_count == first_index
        }) {
            previous.index_count += 3;
        } else {
            ranges.push(SubmeshRange {
                first_index,
                index_count: 3,
                material_id: id,
                semantic_id: base.id,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::HistologicalTissueLayer;
    use super::*;

    #[test]
    fn translucent_sheet_does_not_move_opaque_lamina_into_blend_pass() {
        let mut base = MaterialRecord::legacy_default();
        base.id = 7;
        let slice = TissueSubmeshSlice {
            layer: HistologicalTissueLayer::WaxyCuticle,
            vertex_start: 0,
            vertex_count: 6,
            index_start: 0,
            index_count: 9,
            plant_ontology_id: "PO:0000022",
            common_name: "cuticle",
        };
        let colors = [
            [255; 4],
            [255; 4],
            [255; 4],
            [200, 220, 210, 100],
            [200, 220, 210, 100],
            [200, 220, 210, 100],
        ];
        let mut materials = Vec::new();
        let mut ranges = Vec::new();
        append_tissue_ranges(
            base,
            &slice,
            &[0, 1, 2, 3, 4, 5, 0, 2, 1],
            &colors,
            &mut materials,
            &mut ranges,
        )
        .unwrap();
        assert_eq!(ranges.len(), 3);
        assert_eq!(ranges.iter().map(|r| r.index_count).sum::<u32>(), 9);
        assert_eq!(ranges[0].material_id, ranges[2].material_id);
        assert!(ranges.iter().all(|r| r.semantic_id == 7));
        assert_eq!(materials[0].opacity_mode, OpacityMode::Opaque);
        assert_eq!(materials[1].opacity_mode, OpacityMode::Blend);
        assert!(!materials[1].casts_shadow);
    }
}
