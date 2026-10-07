//! Stable-ID encoding for the bounded R32Uint mesh-picking target.
//!
//! Tensor picks occupy the low half of the target. Mesh picks carry a high-bit tag and a compact
//! visible-instance slot; the CPU resolves that slot through a frame snapshot to the original
//! 64-bit semantic identity. The snapshot is retained until asynchronous map completion.

use crate::render::instance_culling::GpuInstanceRecord;

pub(super) const MESH_PICK_FLAG: u32 = 1 << 31;
pub(super) const MESH_PICK_SLOT_MASK: u32 = !MESH_PICK_FLAG;

#[cfg(test)]
pub(super) fn encode_mesh_slot(slot: u32) -> Option<u32> {
    (slot < MESH_PICK_FLAG).then_some(MESH_PICK_FLAG | slot)
}

pub(super) fn decode_mesh_slot(encoded: u32) -> Option<usize> {
    (encoded & MESH_PICK_FLAG != 0).then_some((encoded & MESH_PICK_SLOT_MASK) as usize)
}

pub(super) fn snapshot_semantic_ids(
    source: &[GpuInstanceRecord],
    out: &mut [u64],
) -> Option<usize> {
    if source.len() > out.len() || source.len() >= MESH_PICK_FLAG as usize {
        return None;
    }
    for (record, slot) in source.iter().zip(out.iter_mut()) {
        *slot = record.semantic_id();
    }
    Some(source.len())
}

#[cfg(test)]
mod tests {
    use super::{decode_mesh_slot, encode_mesh_slot, snapshot_semantic_ids, MESH_PICK_FLAG};
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
}
