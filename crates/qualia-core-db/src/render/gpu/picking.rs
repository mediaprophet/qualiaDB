//! Stable-ID encoding for the bounded R32Uint mesh-picking target.
//!
//! Tensor picks occupy the low half of the target. Mesh picks carry a high-bit tag and a compact
//! visible-instance slot; the CPU resolves that slot through a frame snapshot to the original
//! 64-bit semantic identity. The snapshot is retained until asynchronous map completion.

use super::instance_groups::MeshInstanceGroup;
use crate::render::instance_culling::GpuInstanceRecord;

pub(super) use etalon_render::picking::MESH_PICK_FLAG;
#[cfg(test)]
use etalon_render::picking::{decode_mesh_slot, encode_mesh_slot};

#[allow(dead_code)]
pub(super) fn snapshot_semantic_ids(
    source: &[GpuInstanceRecord],
    out: &mut [u64],
) -> Option<usize> {
    etalon_render::picking::snapshot_ids(source, out, |record| record.semantic_id())
}

/// Translate a compact draw slot back into its pre-cull source instance index.
/// Group culling stores each selection relative to the corresponding prototype's
/// source range, while the draw uses a compact range across all prototypes.
pub(super) fn visible_to_source_slot(
    visible_slot: usize,
    selected: &[u32],
    groups: &[MeshInstanceGroup],
    source_count: usize,
) -> Option<u32> {
    if groups.is_empty() {
        let slot = *selected.get(visible_slot)?;
        return ((slot as usize) < source_count).then_some(slot);
    }
    let visible_slot = u32::try_from(visible_slot).ok()?;
    // Failed group visibility clears all visible ranges and never records a mesh pick.
    let index = groups.partition_point(|group| group.visible.end <= visible_slot);
    let group = groups.get(index)?;
    if !group.visible.contains(&visible_slot) {
        return None;
    }
    let relative = visible_slot - group.visible.start;
    let selected_index = group.source.start.checked_add(relative)?;
    let slot = group
        .source
        .start
        .checked_add(*selected.get(selected_index as usize)?)?;
    (group.source.contains(&slot) && (slot as usize) < source_count).then_some(slot)
}

#[cfg(test)]
mod tests {
    use super::{
        decode_mesh_slot, encode_mesh_slot, snapshot_semantic_ids, visible_to_source_slot,
        MeshInstanceGroup, MESH_PICK_FLAG,
    };
    use crate::render::instance_culling::GpuInstanceRecord;

    const IDENTITY: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];

    #[test]
    fn mesh_pick_slots_are_disjoint_from_tensor_indices() {
        for slot in [0, 1, 9_999, MESH_PICK_FLAG - 1] {
            let encoded = encode_mesh_slot(slot).unwrap();
            assert_eq!(decode_mesh_slot(encoded), Some(slot as usize));
            assert!(encoded & MESH_PICK_FLAG != 0);
        }
        assert_eq!(encode_mesh_slot(MESH_PICK_FLAG), None);
        assert_eq!(decode_mesh_slot(17), None);
    }

    #[test]
    fn snapshot_keeps_full_semantic_ids_in_visible_slot_order() {
        let records = [
            GpuInstanceRecord::new(IDENTITY, 0x0123_4567_89ab_cdef),
            GpuInstanceRecord::new(IDENTITY, u64::MAX),
        ];
        let mut snapshot = [0; 2];
        assert_eq!(snapshot_semantic_ids(&records, &mut snapshot), Some(2));
        assert_eq!(snapshot, [0x0123_4567_89ab_cdef, u64::MAX]);
        assert_eq!(snapshot_semantic_ids(&records, &mut [0]), None);
    }

    #[test]
    fn compact_pick_slots_resolve_original_indices_across_culled_groups() {
        let groups = [
            MeshInstanceGroup {
                indices: 0..3,
                source: 0..2,
                visible: 0..0,
                bounds_min: [0.0; 3],
                bounds_max: [1.0; 3],
            },
            MeshInstanceGroup {
                indices: 3..6,
                source: 2..5,
                visible: 0..2,
                bounds_min: [0.0; 3],
                bounds_max: [1.0; 3],
            },
            MeshInstanceGroup {
                indices: 6..9,
                source: 5..6,
                visible: 2..3,
                bounds_min: [0.0; 3],
                bounds_max: [1.0; 3],
            },
        ];
        let selected = [0, 0, 0, 2, 0, 0];
        assert_eq!(visible_to_source_slot(0, &selected, &groups, 6), Some(2));
        assert_eq!(visible_to_source_slot(1, &selected, &groups, 6), Some(4));
        assert_eq!(visible_to_source_slot(2, &selected, &groups, 6), Some(5));
        assert_eq!(visible_to_source_slot(3, &selected, &groups, 6), None);
        assert_eq!(visible_to_source_slot(0, &[2, 3], &[], 4), Some(2));
    }
}
