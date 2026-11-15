//! Bake-time map: tensor SOA index ↔ KV cache prompt slot (B3.2b / §7.3.3).

use std::sync::{OnceLock, RwLock};

pub const MAX_KV_PROVENANCE: usize = 1024;
pub const KV_SLOT_UNMAPPED: u32 = u32::MAX;

#[derive(Clone, Copy, Debug)]
pub struct KvSlotInfo {
    pub kv_slot: u32,
    pub parent_page_id: u64,
}

impl KvSlotInfo {
    pub const UNMAPPED: Self = Self {
        kv_slot: KV_SLOT_UNMAPPED,
        parent_page_id: 0,
    };
}

pub struct KvProvenanceMap {
    tensor_to_kv: [KvSlotInfo; MAX_KV_PROVENANCE],
    kv_to_tensor: [u32; MAX_KV_PROVENANCE],
    generation: u32,
}

impl KvProvenanceMap {
    /// Creates a default `KvProvenanceMap`.
    pub fn new() -> Self {
        Self {
            tensor_to_kv: [KvSlotInfo::UNMAPPED; MAX_KV_PROVENANCE],
            kv_to_tensor: [KV_SLOT_UNMAPPED; MAX_KV_PROVENANCE],
            generation: 0,
        }
    }

    #[inline]
    /// Generation.
    pub fn generation(&self) -> u32 {
        self.generation
    }

    /// Record a single tensor index ↔ KV slot bidirectional binding in O(1) (cold path).
    pub fn record(&mut self, tensor_index: u32, kv_slot: u32, parent_page_id: u64) {
        let ti = tensor_index as usize;
        let ks = kv_slot as usize;
        if ti < MAX_KV_PROVENANCE && ks < MAX_KV_PROVENANCE {
            // Unmap old reverse slot if this tensor was previously bound elsewhere
            let old_kv = self.tensor_to_kv[ti].kv_slot as usize;
            if old_kv < MAX_KV_PROVENANCE && self.kv_to_tensor[old_kv] == tensor_index {
                self.kv_to_tensor[old_kv] = KV_SLOT_UNMAPPED;
            }
            self.tensor_to_kv[ti] = KvSlotInfo {
                kv_slot,
                parent_page_id,
            };
            self.kv_to_tensor[ks] = tensor_index;
            self.generation = self.generation.wrapping_add(1);
        }
    }

    /// 1:1 prompt alignment: tensor node `i` ↔ KV slot `i` for `min(prompt, nodes)`.
    pub fn build_prompt_alignment(
        &mut self,
        prompt_token_count: u32,
        tensor_node_count: u32,
        root_page_id: u64,
    ) {
        let n = prompt_token_count
            .min(tensor_node_count)
            .min(MAX_KV_PROVENANCE as u32);
        for i in 0..n {
            let idx = i as usize;
            self.tensor_to_kv[idx] = KvSlotInfo {
                kv_slot: i,
                parent_page_id: root_page_id,
            };
            self.kv_to_tensor[idx] = i;
        }
        for i in n as usize..MAX_KV_PROVENANCE {
            self.tensor_to_kv[i] = KvSlotInfo::UNMAPPED;
            self.kv_to_tensor[i] = KV_SLOT_UNMAPPED;
        }
        self.generation = self.generation.wrapping_add(1);
    }

    #[inline]
    /// `KvProvenanceMap::kv_slot_for_tensor`.
    pub fn kv_slot_for_tensor(&self, tensor_index: u32) -> Option<u32> {
        let ti = tensor_index as usize;
        if ti >= MAX_KV_PROVENANCE {
            return None;
        }
        let info = self.tensor_to_kv[ti];
        if info.kv_slot == KV_SLOT_UNMAPPED {
            None
        } else {
            Some(info.kv_slot)
        }
    }

    #[inline]
    /// Reverse lookup: find the 3D tensor index corresponding to a generated KV slot in O(1).
    pub fn tensor_for_kv_slot(&self, kv_slot: u32) -> Option<u32> {
        let ks = kv_slot as usize;
        if ks >= MAX_KV_PROVENANCE {
            return None;
        }
        let t = self.kv_to_tensor[ks];
        if t == KV_SLOT_UNMAPPED {
            None
        } else {
            Some(t)
        }
    }

    #[inline]
    /// `KvProvenanceMap::page_id_for_tensor`.
    pub fn page_id_for_tensor(&self, tensor_index: u32) -> Option<u64> {
        let ti = tensor_index as usize;
        if ti >= MAX_KV_PROVENANCE {
            return None;
        }
        let info = self.tensor_to_kv[ti];
        if info.kv_slot == KV_SLOT_UNMAPPED {
            None
        } else {
            Some(info.parent_page_id)
        }
    }
}

static KV_PROVENANCE: OnceLock<RwLock<KvProvenanceMap>> = OnceLock::new();

fn kv_lock() -> &'static RwLock<KvProvenanceMap> {
    KV_PROVENANCE.get_or_init(|| RwLock::new(KvProvenanceMap::new()))
}

#[inline]
/// `KvProvenanceMap::global_kv_provenance`.
pub fn global_kv_provenance() -> std::sync::RwLockReadGuard<'static, KvProvenanceMap> {
    kv_lock().read().expect("kv provenance poisoned")
}

/// Reverse lookup helper for HUD/renderer: resolves KV slot to 3D tensor index in O(1) zero-heap.
#[inline]
pub fn global_tensor_for_kv_slot(kv_slot: u32) -> Option<u32> {
    global_kv_provenance().tensor_for_kv_slot(kv_slot)
}

/// Rebuild provenance table (prompt prefill / spatial encode).
pub fn rebuild_prompt_provenance(
    prompt_token_count: u32,
    tensor_node_count: u32,
    root_page_id: u64,
) {
    if crate::compute_universe::rebuild_context_provenance(prompt_token_count, root_page_id) {
        return;
    }
    kv_lock()
        .write()
        .expect("kv provenance poisoned")
        .build_prompt_alignment(prompt_token_count, tensor_node_count, root_page_id);
}

#[inline]
/// `KvProvenanceMap::record_kv_provenance`.
pub fn record_kv_provenance(tensor_index: u32, kv_slot: u32, parent_page_id: u64) {
    if crate::compute_universe::record_context_provenance(tensor_index, kv_slot, parent_page_id) {
        return;
    }
    kv_lock().write().expect("kv provenance poisoned").record(
        tensor_index,
        kv_slot,
        parent_page_id,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_alignment_maps_tensor_to_kv_bidirectional() {
        let mut map = KvProvenanceMap::new();
        map.build_prompt_alignment(8, 10, 42);
        assert_eq!(map.kv_slot_for_tensor(3), Some(3));
        assert_eq!(map.tensor_for_kv_slot(3), Some(3));
        assert_eq!(map.page_id_for_tensor(3), Some(42));

        assert_eq!(map.kv_slot_for_tensor(9), None);
        assert_eq!(map.tensor_for_kv_slot(9), None);
    }

    #[test]
    fn record_maintains_bidirectional_mapping() {
        let mut map = KvProvenanceMap::new();
        map.record(42, 10, 777);
        assert_eq!(map.kv_slot_for_tensor(42), Some(10));
        assert_eq!(map.tensor_for_kv_slot(10), Some(42));
        assert_eq!(map.page_id_for_tensor(42), Some(777));

        // Re-record tensor 42 to slot 20
        map.record(42, 20, 778);
        assert_eq!(map.kv_slot_for_tensor(42), Some(20));
        assert_eq!(map.tensor_for_kv_slot(20), Some(42));
        assert_eq!(map.tensor_for_kv_slot(10), None);
    }
}
