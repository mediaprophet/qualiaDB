//! Owned semantic producer path. Search and subsequent tensor lookup use the
//! same immutable bytes, independent of later global substrate replacement.
use super::*;
use crate::tensor::resident_substrate::ResidentSnapshot;
use sha2::{Digest, Sha256};

pub(super) fn produce_owned(source: &ResidentSnapshot, distance: f32, vocab: u32) -> usize {
    let query = load_query_tensor();
    let subject = query_subject_hash();
    let mut hits = [0usize; MAX_KNN_HITS];
    let count = epoch::owned_gpu_search(&query, distance, &mut hits)
        .unwrap_or_else(|| {
            source
                .tensor_search_into(&query, distance, &mut hits)
                .unwrap_or(0)
        })
        .min(hits.len());
    publish_attention_route_mask(build_attention_route_mask(&query, distance, &hits, count));
    let pushed = push_tensor_search_hits(&hits[..count], subject, query.w, distance);
    let mut batch = TopologyDraftBatch::empty();
    let n = count.min(4).min(MAX_DRAFT_LEN);
    for i in 0..n {
        batch.concept_hashes[i] = source.subject_hash_at(hits[i] as u32) ^ subject;
        batch.draft_ids[i] = batch.concept_hashes[i] as u32 % vocab.max(1);
    }
    batch.draft_len = n as u8;
    let _ = topology_draft_ring().try_push(batch);
    pushed
}
pub(crate) fn context_tensor_at(index: u32) -> Option<Tensor10D> {
    match epoch::source() {
        Some(source) => source.tensor_at(index),
        None => global_resident_substrate().tensor_at(index),
    }
}
pub(crate) fn context_node_count() -> u32 {
    epoch::source().map_or_else(
        || global_resident_substrate().node_count(),
        |source| source.node_count(),
    )
}
pub(crate) fn context_live(identity: u64, digest: [u8; 32]) -> bool {
    epoch::live(identity, digest)
}

/// Caller holds the epoch pause guard until the numerical step completes.
/// The WORK mutex excludes producer search, device work, and ring publication.
/// Scoped publication rejects other threads; only this owner thread consumes
/// the rings. The digest and the complete consuming numerical step occur under
/// one pause guard, so these slot reads cannot race a request producer.
/// Bind every pending source record, including tensors resolved from indices,
/// rather than treating an epoch label as source provenance.
pub(crate) fn context_input_digest() -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"qualia:actual-context-step:v1\0");
    h.update(epoch::current().to_le_bytes());
    h.update(epoch::kv_digest());
    if let Some(source) = epoch::source() {
        h.update(source.digest());
    }
    let t = load_query_tensor();
    for f in [t.q, t.v, t.w, t.x, t.y, t.z, t.t, t.alpha, t.mu, t.sigma] {
        h.update(f.to_bits().to_le_bytes());
    }
    h.update(query_subject_hash().to_le_bytes());
    h.update(DECODE_TOKEN_ID.load(Ordering::Acquire).to_le_bytes());
    h.update(DECODE_STEP.load(Ordering::Acquire).to_le_bytes());
    let route = attention_route_mask();
    for w in route.words {
        h.update(w.to_le_bytes());
    }
    h.update(route.active_bits.to_le_bytes());
    let ring = context_inject_ring();
    let r = ring.read_seq.load(Ordering::Acquire);
    let w = ring.write_seq.load(Ordering::Acquire);
    for offset in 0..w.wrapping_sub(r).min(CONTEXT_INJECT_RING_CAP as u32) {
        let seq = r.wrapping_add(offset);
        let slot = (seq % CONTEXT_INJECT_RING_CAP as u32) as usize;
        let token = unsafe { (*ring.slots.get())[slot] };
        h.update(token.tensor_index.to_le_bytes());
        h.update(token.subject_hash.to_le_bytes());
        h.update(token.distance.to_bits().to_le_bytes());
        h.update(token.manifold_w.to_bits().to_le_bytes());
        if let Some(t) = context_tensor_at(token.tensor_index) {
            for f in [t.q, t.v, t.w, t.x, t.y, t.z, t.t, t.alpha, t.mu, t.sigma] {
                h.update(f.to_bits().to_le_bytes());
            }
        }
    }
    let ring = topology_draft_ring();
    let r = ring.read_seq.load(Ordering::Acquire);
    let w = ring.write_seq.load(Ordering::Acquire);
    for offset in 0..w.wrapping_sub(r).min(TOPOLOGY_DRAFT_RING_CAP as u32) {
        let seq = r.wrapping_add(offset);
        let batch = unsafe { (*ring.slots.get())[(seq % TOPOLOGY_DRAFT_RING_CAP as u32) as usize] };
        h.update([batch.draft_len]);
        for id in batch.draft_ids {
            h.update(id.to_le_bytes());
        }
        for concept in batch.concept_hashes {
            h.update(concept.to_le_bytes());
        }
    }
    h.finalize().into()
}
