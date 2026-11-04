//! Cold ownership and deterministic routing metadata for native preparation.
use super::*;
use sha2::{Digest, Sha256};

impl FtwModelPackage {
    pub(crate) fn prepared_leases_match(
        &self,
        leases: &[Arc<etalon_model_io::lease::ModelLease>],
    ) -> bool {
        self.prepared_shards.as_ref().is_some_and(|sources| {
            sources.len() == leases.len()
                && sources.iter().zip(leases).all(|(source, lease)| {
                    source.same_owner(&crate::gguf_bridge::ModelByteHandle::leased(lease.clone()))
                })
        })
    }
    pub(crate) fn shard_bytes(&self, index: usize) -> Option<&[u8]> {
        match &self.prepared_shards {
            Some(shards) => shards.get(index).map(|bytes| &bytes[..]),
            None => self.shards.get(index).map(|bytes| &bytes[..]),
        }
    }

    pub(crate) fn prepare_shards(
        &self,
        sources: Vec<crate::gguf_bridge::ModelByteHandle>,
    ) -> Result<Self, String> {
        if sources.len() != self.manifest.shards.len() {
            return Err("FTW source count mismatch".into());
        }
        for (source, shard) in sources.iter().zip(&self.manifest.shards) {
            if (source.len() as u64) < shard.nbytes {
                return Err("FTW source length mismatch".into());
            }
        }
        let mut package = self.clone();
        package.prepared_shards = Some(sources);
        // Legacy mappings are not consumed by the prepared package. Do not retain
        // their mutable file views or charge their residency to this owner.
        package.shards.clear();
        Ok(package)
    }
    /// Conservative clone reservation. Mapping payload is charged separately by
    /// its producer; this includes owned metadata and deterministic-sort scratch.
    pub(crate) fn prepared_metadata_bytes(&self) -> Option<usize> {
        let mut bytes = std::mem::size_of::<Self>();
        for tensor in &self.manifest.tensors {
            bytes = bytes
                .checked_add(std::mem::size_of::<FtwTensorEntry>())?
                .checked_add(tensor.name.capacity())?
                .checked_add(tensor.kind.capacity())?
                .checked_add(tensor.dtype.capacity())?
                .checked_add(tensor.shape.capacity().checked_mul(8)?)?;
        }
        for shard in &self.manifest.shards {
            bytes = bytes
                .checked_add(std::mem::size_of::<FtwShardEntry>())?
                .checked_add(shard.file.capacity())?;
        }
        for key in self.tensor_locations.keys() {
            bytes = bytes.checked_add(key.capacity())?;
        }
        bytes = bytes
            .checked_add(self.tensor_locations.capacity().checked_mul(128)?)?
            .checked_add(self.expert_slices.capacity().checked_mul(256)?)?
            .checked_add(self.tensor_index.entries.capacity().checked_mul(64)?)?;
        bytes.checked_mul(2)
    }

    pub(crate) fn prepared_routing_digest(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(b"qualia:ftw-consumed-routing:v1\0");
        for field in [
            self.tensor_index.moe_routing.expert_count,
            self.tensor_index.moe_routing.experts_per_token,
        ] {
            hash.update([u8::from(field.is_some())]);
            hash.update(field.unwrap_or(0).to_le_bytes());
        }
        hash.update((self.manifest.shards.len() as u64).to_le_bytes());
        for shard in &self.manifest.shards {
            hash.update(shard.global_off.to_le_bytes());
            hash.update(shard.nbytes.to_le_bytes());
        }
        let mut tensors: Vec<_> = self.tensor_locations.iter().collect();
        tensors.sort_unstable_by(|a, b| a.0.cmp(b.0));
        hash.update((tensors.len() as u64).to_le_bytes());
        for (name, &(shard, offset, length)) in tensors {
            hash.update((name.len() as u64).to_le_bytes());
            hash.update(name.as_bytes());
            for value in [shard, offset, length] {
                hash.update((value as u64).to_le_bytes());
            }
        }
        let mut experts: Vec<_> = self.expert_slices.iter().collect();
        experts.sort_unstable_by_key(|(key, _)| **key);
        hash.update((experts.len() as u64).to_le_bytes());
        for (&(layer, expert), s) in experts {
            hash.update(layer.to_le_bytes());
            hash.update(expert.to_le_bytes());
            for value in [
                s.shard_idx,
                s.gate_up_packed_off,
                s.gate_up_packed_len,
                s.gate_up_scale_off,
                s.gate_up_scale_len,
                s.gate_up_global_off,
                s.gate_up_global_len,
                s.down_packed_off,
                s.down_packed_len,
                s.down_scale_off,
                s.down_scale_len,
                s.down_global_off,
                s.down_global_len,
            ] {
                hash.update((value as u64).to_le_bytes());
            }
        }
        hash.finalize().into()
    }
}
