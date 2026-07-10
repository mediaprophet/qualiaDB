//! Observation → fixed NQuin-compatible records (epistemic, not ground truth).
//!
//! Layout matches Qualia `NQuin` (6 × u64). We keep a local definition so
//! Phase-1 does not force a full `qualia-core-db` link for every UI binary;
//! field meanings are identical and can be cast/transmuted at the core boundary.

use crate::types::Detection;

/// Predicate / context IRIs (hashed with FNV-1a 64, same family as Qualia `q_hash`).
pub const P_VISUAL_OBSERVATION: &str = "https://ns.webizen.org/q42/VisualObservation";
pub const P_PROPOSES_CLASS: &str = "https://ns.webizen.org/q42/proposesClass";
pub const CTX_VISION: &str = "https://ns.webizen.org/q42/vision-observation";

/// Max observation quins emitted per batch (2 per detection: observation + class).
pub const MAX_OBS_QUINS: usize = 128;

/// 48-byte semantic quin (parity = XOR fold of other fields).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisionQuin {
    pub subject: u64,
    pub predicate: u64,
    pub object: u64,
    pub context: u64,
    pub metadata: u64,
    pub parity: u64,
}

impl VisionQuin {
    #[inline]
    pub fn with_parity(
        subject: u64,
        predicate: u64,
        object: u64,
        context: u64,
        metadata: u64,
    ) -> Self {
        let parity = subject ^ predicate ^ object ^ context ^ metadata;
        Self {
            subject,
            predicate,
            object,
            context,
            metadata,
            parity,
        }
    }
}

/// Content digest of media bytes (FNV-1a over up to 64 KiB prefix + length).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaDigest {
    pub hash: u64,
    pub byte_len: u64,
}

/// FNV-1a 64 — matches Qualia URI hashing family.
#[inline]
pub fn q_hash(s: &str) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x100_0000_01b3;
    let mut h = FNV_OFFSET;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

#[inline]
pub fn q_hash_bytes(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x100_0000_01b3;
    let mut h = FNV_OFFSET;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

pub fn media_digest(bytes: &[u8]) -> MediaDigest {
    let take = bytes.len().min(65_536);
    let mut h = q_hash_bytes(&bytes[..take]);
    // Mix length so different sizes of same prefix differ.
    h ^= (bytes.len() as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    MediaDigest {
        hash: h,
        byte_len: bytes.len() as u64,
    }
}

/// One observation quin: media --VisualObservation--> instance, score in metadata low 16.
pub fn observation_quin(media: MediaDigest, det: &Detection, model_hash: u64) -> VisionQuin {
    let subject = media.hash;
    let predicate = q_hash(P_VISUAL_OBSERVATION);
    let object = det.instance_hash;
    let context = q_hash(CTX_VISION) ^ model_hash;
    // metadata: score | frame in high bits (simple packing).
    let metadata = (det.score_u16 as u64)
        | ((det.frame_index as u64) << 16)
        | ((det.flags as u64) << 48);
    VisionQuin::with_parity(subject, predicate, object, context, metadata)
}

/// Class proposal quin: instance --proposesClass--> class_hash.
pub fn class_proposal_quin(det: &Detection, model_hash: u64) -> VisionQuin {
    let subject = det.instance_hash;
    let predicate = q_hash(P_PROPOSES_CLASS);
    let object = det.class_hash;
    let context = q_hash(CTX_VISION) ^ model_hash;
    let metadata = det.score_u16 as u64;
    VisionQuin::with_parity(subject, predicate, object, context, metadata)
}

/// Compile detections into observation + class quins (epistemic layer).
///
/// Returns number of quins written. Each detection contributes up to 2 quins.
pub fn compile_observation_quins(
    media: MediaDigest,
    detections: &[Detection],
    model_hash: u64,
    out: &mut [VisionQuin],
) -> usize {
    let mut w = 0usize;
    for det in detections {
        if det.class_hash == 0 && det.score_u16 == 0 {
            continue;
        }
        if w + 2 > out.len() {
            break;
        }
        out[w] = observation_quin(media, det, model_hash);
        w += 1;
        out[w] = class_proposal_quin(det, model_hash);
        w += 1;
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Detection;

    #[test]
    fn parity_valid() {
        let d = Detection {
            class_hash: 0xABC,
            instance_hash: 0xDEF,
            score_u16: 40_000,
            x_min_u16: 0,
            y_min_u16: 0,
            x_max_u16: 65535,
            y_max_u16: 65535,
            frame_index: 0,
            track_id: 0,
            flags: Detection::FLAG_REFERENCE_BACKEND,
        };
        let m = MediaDigest {
            hash: 0x1111,
            byte_len: 100,
        };
        let q = observation_quin(m, &d, 0x42);
        assert_eq!(
            q.parity,
            q.subject ^ q.predicate ^ q.object ^ q.context ^ q.metadata
        );
    }

    #[test]
    fn compile_two_per_detection() {
        let mut out = [VisionQuin::with_parity(0, 0, 0, 0, 0); 8];
        let d = Detection {
            class_hash: 1,
            instance_hash: 2,
            score_u16: 1000,
            ..Detection::empty()
        };
        let n = compile_observation_quins(
            MediaDigest {
                hash: 9,
                byte_len: 1,
            },
            &[d],
            7,
            &mut out,
        );
        assert_eq!(n, 2);
    }
}
