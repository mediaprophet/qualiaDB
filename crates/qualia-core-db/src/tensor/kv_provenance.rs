//! Bake-time map: tensor SOA index ↔ KV cache prompt slot (B3.2b).

use std::sync::{OnceLock, RwLock};

pub const MAX_KV_PROVENANCE: usize = 1024;
pub const KV_SLOT_UNMAPPED: u32 = u32::MAX;

pub struct KvProvenanceMap {
    tensor_to_kv: [u32; MAX_KV_PROVENANCE],
    generation: u32,
}

impl KvProvenanceMap {
    pub fn new() -> Self {
        Self {
            tensor_to_kv: [KV_SLOT_UNMAPPED; MAX_KV_PROVENANCE],
            generation: 0,
        }
    }

    #[inline]
    pub fn generation(&self) -> u32 {
        self.generation
    }

    /// Record a single tensor index → KV slot binding (cold path).
    pub fn record(&mut self, tensor_index: u32, kv_slot: u32) {
        let ti = tensor_index as usize;
        if ti < MAX_KV_PROVENANCE && kv_slot < MAX_KV_PROVENANCE as u32 {
            self.tensor_to_kv[ti] = kv_slot;
            self.generation = self.generation.wrapping_add(1);
        }
    }

    /// 1:1 prompt alignment: tensor node `i` → KV slot `i` for `min(prompt, nodes)`.
    pub fn build_prompt_alignment(&mut self, prompt_token_count: u32, tensor_node_count: u32) {
        let n = prompt_token_count
            .min(tensor_node_count)
            .min(MAX_KV_PROVENANCE as u32);
        for i in 0..n {
            self.tensor_to_kv[i as usize] = i;
        }
        for i in n as usize..MAX_KV_PROVENANCE {
            self.tensor_to_kv[i] = KV_SLOT_UNMAPPED;
        }
        self.generation = self.generation.wrapping_add(1);
    }

    #[inline]
    pub fn kv_slot_for_tensor(&self, tensor_index: u32) -> Option<u32> {
        let ti = tensor_index as usize;
        if ti >= MAX_KV_PROVENANCE {
            return None;
        }
        let slot = self.tensor_to_kv[ti];
        if slot == KV_SLOT_UNMAPPED {
            None
        } else {
            Some(slot)
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
pub fn rebuild_prompt_provenance(prompt_token_count: u32, tensor_node_count: u32) {
    kv_lock()
        .write()
        .expect("kv provenance poisoned")
        .build_prompt_alignment(prompt_token_count, tensor_node_count);
}

#[inline]
pub fn record_kv_provenance(tensor_index: u32, kv_slot: u32) {
    kv_lock()
        .write()
        .expect("kv provenance poisoned")
        .record(tensor_index, kv_slot);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_alignment_maps_tensor_to_kv() {
        let mut map = KvProvenanceMap::new();
        map.build_prompt_alignment(8, 10);
        assert_eq!(map.kv_slot_for_tensor(3), Some(3));
        assert_eq!(map.kv_slot_for_tensor(9), None);
    }
}