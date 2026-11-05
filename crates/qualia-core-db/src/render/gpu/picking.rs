//! Stable-ID encoding for the bounded R32Uint mesh-picking target.
//!
//! Tensor picks occupy the low half of the target. Mesh picks carry a high-bit tag and a compact
//! visible-instance slot; the CPU resolves that slot through a frame snapshot to the original
//! 64-bit semantic identity. The snapshot is retained until asynchronous map completion.

use crate::render::instance_culling::GpuInstanceRecord;

#[cfg(test)]
use etalon_render::picking::{decode_mesh_slot, encode_mesh_slot};
pub(super) use etalon_render::picking::MESH_PICK_FLAG;

#[allow(dead_code)]
pub(super) fn snapshot_semantic_ids(
    source: &[GpuInstanceRecord],
    out: &mut [u64],
) -> Option<usize> {
    etalon_render::picking::snapshot_ids(source, out, |record| record.semantic_id())
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
