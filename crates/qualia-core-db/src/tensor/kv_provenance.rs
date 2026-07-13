//! Bake-time map: tensor SOA index ↔ KV cache prompt slot (B3.2b).

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
    generation: u32,
}

impl KvProvenanceMap {
    pub fn new() -> Self {
        Self {
            tensor_to_kv: [KvSlotInfo::UNMAPPED; MAX_KV_PROVENANCE],
            generation: 0,
        }
    }

    #[inline]
    pub fn generation(&self) -> u32 {
        self.generation
    }

    /// Record a single tensor index → KV slot binding (cold path).
    pub fn record(&mut self, tensor_index: u32, kv_slot: u32, parent_page_id: u64) {
        let ti = tensor_index as usize;
        if ti < MAX_KV_PROVENANCE && kv_slot < MAX_KV_PROVENANCE as u32 {
            self.tensor_to_kv[ti] = KvSlotInfo {
                kv_slot,
                parent_page_id,
            };
            self.generation = self.generation.wrapping_add(1);
        }
    }

    /// 1:1 prompt alignment: tensor node `i` → KV slot `i` for `min(prompt, nodes)`.
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
            self.tensor_to_kv[i as usize] = KvSlotInfo {
                kv_slot: i,
                parent_page_id: root_page_id,
            };
        }
        for i in n as usize..MAX_KV_PROVENANCE {
            self.tensor_to_kv[i] = KvSlotInfo::UNMAPPED;
        }
        self.generation = self.generation.wrapping_add(1);
    }

    #[inline]
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
pub fn global_kv_provenance() -> std::sync::RwLockReadGuard<'static, KvProvenanceMap> {
    kv_lock().read().expect("kv provenance poisoned")
}

/// Rebuild provenance table (prompt prefill / spatial encode).
pub fn rebuild_prompt_provenance(
    prompt_token_count: u32,
    tensor_node_count: u32,
    root_page_id: u64,
) {
    kv_lock()
        .write()
        .expect("kv provenance poisoned")
        .build_prompt_alignment(prompt_token_count, tensor_node_count, root_page_id);
}

#[inline]
pub fn record_kv_provenance(tensor_index: u32, kv_slot: u32, parent_page_id: u64) {
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
    fn prompt_alignment_maps_tensor_to_kv() {
        let mut map = KvProvenanceMap::new();
        map.build_prompt_alignment(8, 10, 42);
        assert_eq!(map.kv_slot_for_tensor(3), Some(3));
        assert_eq!(map.page_id_for_tensor(3), Some(42));

        assert_eq!(map.kv_slot_for_tensor(9), None);
    }
}
