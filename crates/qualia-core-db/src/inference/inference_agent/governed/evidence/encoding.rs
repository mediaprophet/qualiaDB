use super::{BoundProposal, Payload};
use sha2::{Digest, Sha256};
pub(super) fn payload_digest(bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"qualia:inference:exact-payload:v3\0");
    h.update((bytes.len() as u64).to_le_bytes());
    h.update(bytes);
    h.finalize().into()
}
pub(super) fn candidate_digest(bound: &BoundProposal<'_>) -> [u8; 32] {
    let p = bound.proposal;
    let mut h = Sha256::new();
    h.update(match p.payload {
        Payload::Token { .. } => b"qualia:inference:token-candidate:v3\0".as_slice(),
        Payload::Artifact { .. } => b"qualia:inference:artifact-candidate:v3\0".as_slice(),
    });
    h.update(3u32.to_le_bytes());
    h.update([p.source as u8]);
    h.update([p.artifact_origin.map_or(0, |origin| origin as u8)]);
    match p.payload {
        Payload::Token {
            token, position, ..
        } => {
            h.update([0]);
            h.update(token.to_le_bytes());
            h.update(position.to_le_bytes());
        }
        Payload::Artifact { media, .. } => {
            h.update([media as u8]);
        }
    }
    match p.numerical {
        None => h.update([0]),
        Some(s) => {
            h.update([s.meaning as u8]);
            h.update(s.value.to_bits().to_le_bytes());
        }
    }
    h.update((bound.bytes().len() as u64).to_le_bytes());
    h.update(bound.payload_digest);
    h.update(p.provenance_digest);
    for v in [
        p.observation.subject,
        p.observation.predicate,
        p.observation.object,
        p.observation.context,
        p.observation.metadata,
        p.observation.parity,
    ] {
        h.update(v.to_le_bytes());
    }
    let i = p.identity;
    for v in [
        i.session_id,
        i.session_generation,
        i.request_id,
        i.request_generation,
    ] {
        h.update(v.to_le_bytes());
    }
    h.update(p.request_digest);
    h.update(i.model_id);
    for v in [i.model_generation, i.device_id, i.device_generation] {
        h.update(v.to_le_bytes());
    }
    h.update(i.policy_id);
    h.update(i.policy_generation.to_le_bytes());
    h.update(p.sequence.to_le_bytes());
    h.finalize().into()
}
