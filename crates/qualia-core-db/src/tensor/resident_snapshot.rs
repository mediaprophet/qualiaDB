//! Bounded immutable semantic source retained by an inference request.
use super::*;
use sha2::{Digest, Sha256};

pub struct ResidentSnapshot {
    nquins: Box<[NQuin]>,
    metadata: Box<[TensorMetadata]>,
    subjects: Box<[u64]>,
    digest: [u8; 32],
}
impl ResidentTensorSubstrate {
    pub fn snapshot_owned(&self, budget: usize) -> Result<ResidentSnapshot, &'static str> {
        let state = self
            .state
            .read()
            .map_err(|_| "resident substrate poisoned")?;
        let n = state.node_count as usize;
        let stride = std::mem::size_of::<NQuin>() + std::mem::size_of::<TensorMetadata>() + 8;
        let required = n
            .checked_mul(stride)
            .and_then(|n| {
                n.checked_add(
                    std::mem::size_of::<ResidentSnapshot>() + 2 * std::mem::size_of::<usize>(),
                )
            })
            .ok_or("context budget overflow")?;
        if required > budget {
            return Err("context source budget exceeded");
        }
        let mut hash = Sha256::new();
        hash.update(b"qualia:owned-resident-context:v1\0");
        hash.update((n as u64).to_le_bytes());
        for i in 0..n {
            let q = state.nquins[i];
            for word in [
                q.subject,
                q.predicate,
                q.object,
                q.context,
                q.metadata,
                q.parity,
                state.subject_hashes[i],
            ] {
                hash.update(word.to_le_bytes());
            }
            let m = state.metadata[i];
            hash.update([u8::from(m.has_tensor)]);
            hash.update(m.tensor_version.to_le_bytes());
            let t = m.tensor;
            for f in [t.q, t.v, t.w, t.x, t.y, t.z, t.t, t.alpha, t.mu, t.sigma] {
                hash.update(f.to_bits().to_le_bytes());
            }
        }
        Ok(ResidentSnapshot {
            nquins: state.nquins[..n].into(),
            metadata: state.metadata[..n].into(),
            subjects: state.subject_hashes[..n].into(),
            digest: hash.finalize().into(),
        })
    }
}
impl ResidentSnapshot {
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + 2 * std::mem::size_of::<usize>()
            + self.nquins.len() * std::mem::size_of::<NQuin>()
            + self.metadata.len() * std::mem::size_of::<TensorMetadata>()
            + self.subjects.len() * 8
    }
    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }
    pub fn node_count(&self) -> u32 {
        self.nquins.len() as u32
    }
    pub fn subject_hash_at(&self, index: u32) -> u64 {
        self.subjects.get(index as usize).copied().unwrap_or(0)
    }
    pub fn tensor_at(&self, index: u32) -> Option<Tensor10D> {
        self.metadata
            .get(index as usize)
            .filter(|m| m.has_tensor)
            .map(|m| m.tensor)
    }
    pub fn tensor_search_into(
        &self,
        query: &Tensor10D,
        distance: f32,
        out: &mut [usize],
    ) -> Result<usize, TensorVolumeError> {
        let config = TensorVolumeConfig::default();
        Q42TensorView::new(&self.nquins, &self.metadata, &config)?
            .tensor_search_into(query, distance, out)
    }
}
